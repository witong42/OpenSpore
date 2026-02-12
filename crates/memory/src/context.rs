use openspore_core::state::AppState;
use crate::{MemorySystem, MemoryItem};
use std::path::PathBuf;
use tokio::fs;
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

pub trait ContextCompressor: Send + Sync {
    fn compress<'a>(&'a self, current: &'a str, new_items: &'a str) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>;
}

/// Exact port of opensporejs/src/context_manager.js
#[derive(Clone)]
pub struct ContextManager {
    pub memory: MemorySystem,
    pub summary_path: PathBuf,
    pub max_raw_items: usize,
}

#[derive(Debug, Clone)]
pub struct WorkingContext {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub summary: String,
    pub recent: String,
    pub older_items: Vec<MemoryItem>,
}

impl Default for WorkingContext {
    fn default() -> Self {
        Self {
            timestamp: chrono::Local::now(),
            summary: String::new(),
            recent: String::new(),
            older_items: Vec::new(),
        }
    }
}

impl ContextManager {
    pub fn new(state: &AppState) -> Self {
        let memory = MemorySystem::new(state);
        let summary_path = memory.memory_root.join("context").join("session_summary.md");

        Self {
            memory,
            summary_path,
            max_raw_items: 12,
        }
    }

    pub fn clone_manager(&self) -> Self {
        self.clone()
    }

    /// Get working context (lines 11-37 in JS)
    pub async fn get_working_context(&self, _compressor: Option<&impl ContextCompressor>) -> Result<WorkingContext> {
        let raw_items: Vec<MemoryItem> = self.memory.get_memories("context")
            .into_iter()
            .filter(|m| m.filename != "LOGS.md" && m.filename != "session_summary.md")
            .collect();

        let len = raw_items.len();
        let split_point = if len > self.max_raw_items { len - self.max_raw_items } else { 0 };

        let older_items: Vec<MemoryItem> = raw_items[..split_point].to_vec();
        let recent_items: Vec<MemoryItem> = raw_items[split_point..].to_vec();

        let mut summary = "No session summary available.".to_string();
        if self.summary_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.summary_path).await {
                summary = content;
            }
        }

        // Filter recent items: if their key content is already in the summary, omit them
        let summary_lower = summary.to_lowercase();
        let filtered_recent: Vec<String> = recent_items.into_iter()
            .map(|m| m.content)
            .filter(|content| {
                // Heuristic: if more than 50% of the words in a short exchange are in the summary, it's redundant
                let words: Vec<&str> = content.split_whitespace().collect();
                if words.is_empty() { return true; }

                let matches = words.iter()
                    .filter(|w| w.len() > 4 && summary_lower.contains(&w.to_lowercase()))
                    .count();

                let redundancy_ratio = matches as f32 / words.len() as f32;
                redundancy_ratio < 0.8 // Keep if less than 80% redundant
            })
            .collect();

        Ok(WorkingContext {
            timestamp: chrono::Local::now(),
            summary,
            recent: filtered_recent.join("\n\n"),
            older_items,
        })
    }

    /// Explicitly trigger compression of older items
    pub async fn compress_older_items(&self, items: Vec<MemoryItem>, compressor: &impl ContextCompressor) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let mut summary = "No session summary available.".to_string();
        if self.summary_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.summary_path).await {
                summary = content;
            }
        }

        let items_content = items.iter().map(|m| m.content.clone()).collect::<Vec<_>>().join("\n---\n");
        match compressor.compress(&summary, &items_content).await {
            Ok(new_summary) => {
                self.memory.mark_as_internal_write(self.summary_path.clone()).await;
                if let Err(e) = fs::write(&self.summary_path, &new_summary).await {
                    tracing::error!("Failed to write session summary: {}", e);
                } else {
                    // Cleanup older items
                    for item in items {
                        let path = self.memory.memory_root.join("context").join(&item.filename);
                        self.memory.mark_as_internal_write(path.clone()).await;
                        let _ = fs::remove_file(path).await;
                    }
                }
                Ok(())
            },
            Err(e) => {
                tracing::error!("Context Compression Error: {}", e);
                Err(e)
            }
        }
    }

    /// Save interaction helper (missing in previous port)
    pub async fn save_interaction(&self, content: &str, tags: Vec<String>, memory_type: Option<&str>) -> Result<Option<PathBuf>> {
        self.memory.save_memory("context", &format!("Exchange_{}", chrono::Local::now().format("%Y%m%d_%H%M%S")), content, tags, memory_type).await
    }
}
