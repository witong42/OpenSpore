use openspore_core::state::AppState;
use crate::{MemorySystem, MemoryItem};
use std::path::PathBuf;
use tokio::fs;
use anyhow::Result;

/// Exact port of opensporejs/src/context_manager.js
#[derive(Clone)]
pub struct ContextManager {
    pub memory: MemorySystem,
    pub summary_path: PathBuf,
    pub max_raw_items: usize,
}

impl ContextManager {
    pub fn new(state: &AppState) -> Self {
        let memory = MemorySystem::new(state);
        let summary_path = memory.memory_root.join("context").join("session_summary.md");

        Self {
            memory,
            summary_path,
            max_raw_items: 4,
        }
    }

    pub fn clone_manager(&self) -> Self {
        self.clone()
    }

    /// Get working context (lines 11-37 in JS)
    pub async fn get_working_context(&self) -> Result<WorkingContext> {
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

        // TODO: If older_items.len() > 0, compress them into summary (requires Brain integration)
        // For now, we just return without compression (matching the minimal viable port)

        Ok(WorkingContext {
            summary,
            recent: recent_items.iter().map(|m| m.content.clone()).collect::<Vec<_>>().join("\n\n"),
            older_items,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct WorkingContext {
    pub summary: String,
    pub recent: String,
    pub older_items: Vec<MemoryItem>,
}
