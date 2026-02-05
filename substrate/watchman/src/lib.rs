//! OpenSpore Watchman (v3.0 Autonomous Swarm)
//!
//! Monitors the substrate for changes and triggers cognitive updates.
//! Exact port of opensporejs/src/watchman.js

use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use tracing::{info, warn, error};
use openspore_core::config::AppConfig;
use openspore_core::state::AppState;
use openspore_memory::MemorySystem;
use openspore_brain::Brain;

/// Watchman - filesystem observer that triggers learning
pub struct Watchman {
    project_root: PathBuf,
    memory: MemorySystem,
    brain: Brain,
    ignore_rules: HashSet<String>,
    allowed_extensions: Vec<String>,
    queue: Arc<Mutex<Vec<WatchEvent>>>,
}

#[derive(Debug, Clone)]
pub struct WatchEvent {
    pub event_type: String,
    pub file_path: PathBuf,
}

impl Watchman {
    pub fn new(config: AppConfig) -> Self {
        let state = AppState::new(config.clone());
        let memory = MemorySystem::new(&state);
        let brain = Brain::new(config);

        // Default ignore rules (matching memory.ignoreDirs + workspace)
        let ignore_rules: HashSet<String> = [
            "node_modules", "target", ".git", "dist", "build", "coverage",
            "__pycache__", ".next", "bin", "lib", "workspace"
        ].iter().map(|s| s.to_string()).collect();

        let allowed_extensions: Vec<String> = vec![
            ".md", ".txt", ".json", ".yaml", ".yml",
            ".js", ".ts", ".py", ".rs", ".go", ".c", ".cpp", ".h", ".sh"
        ].iter().map(|s| s.to_string()).collect();

        Self {
            project_root: memory.project_root.clone(),
            memory,
            brain,
            ignore_rules,
            allowed_extensions,
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Load additional ignore rules from .watchmanignore
    pub fn load_ignore_rules(&mut self) {
        let ignore_file = self.project_root.join(".watchmanignore");
        if ignore_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&ignore_file) {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        self.ignore_rules.insert(line.to_string());
                    }
                }
            }
        }
        info!("👀 Watchman Ignore Rules: {:?}", self.ignore_rules);
    }

    /// Check if a file path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        let rel_path = path.strip_prefix(&self.project_root).unwrap_or(path);

        for component in rel_path.components() {
            if let std::path::Component::Normal(name) = component {
                let name_str = name.to_string_lossy();
                if self.ignore_rules.contains(name_str.as_ref()) {
                    return true;
                }
            }
        }

        // Check extension
        if let Some(ext) = path.extension() {
            let ext_str = format!(".{}", ext.to_string_lossy().to_lowercase());
            if !self.allowed_extensions.contains(&ext_str) {
                return true;
            }
        } else {
            return true; // No extension = ignore
        }

        false
    }

    /// Enqueue a file event for processing
    async fn enqueue(&self, event_type: &str, file_path: PathBuf) {
        if self.memory.is_internal_write(&file_path).await {
            info!("👀 Watchman: Ignoring internal write to {:?}", file_path);
            return;
        }

        if self.should_ignore(&file_path) {
            return;
        }

        info!("👀 Watchman detected {}: {:?}", event_type, file_path);

        let mut queue = self.queue.lock().await;
        queue.push(WatchEvent {
            event_type: event_type.to_string(),
            file_path,
        });
    }

    /// Process queued events - trigger learning
    pub async fn process_queue(&self) {
        let events: Vec<WatchEvent> = {
            let mut queue = self.queue.lock().await;
            std::mem::take(&mut *queue)
        };

        for event in events {
            if let Err(e) = self.process_event(&event).await {
                error!("Watchman Error processing {:?}: {}", event.file_path, e);
            }
        }
    }

    /// Process a single event - read file and trigger learn()
    async fn process_event(&self, event: &WatchEvent) -> anyhow::Result<()> {
        let content = tokio::fs::read_to_string(&event.file_path).await?;

        // Truncate content preview
        let preview = if content.len() > 1000 {
            format!("{}...", &content[..1000])
        } else {
            content.clone()
        };

        // Create context for learning
        let context = format!(
            "[System Event]: File {} detected at {:?}.\nContent Preview:\n{}",
            event.event_type,
            event.file_path,
            preview
        );

        // Use brain to analyze and extract knowledge
        // This matches memory.learn() in JS
        let analysis_prompt = format!(
            r#"Analyze this file change for new knowledge:

{}

If this contains:
1. User preferences (likes, dislikes, workflow habits)
2. New factual knowledge
3. Critical context for future tasks

Output JSON:
{{"should_save": true/false, "category": "preferences"|"knowledge"|"context", "title": "Short Title", "content": "...", "tags": ["tag1"]}}

If nothing worth saving, set "should_save": false."#,
            context
        );

        let response = self.brain.think_simple(&analysis_prompt).await;

        // Try to parse JSON response
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&response) {
            if data["should_save"].as_bool().unwrap_or(false) {
                let category = data["category"].as_str().unwrap_or("context");
                let title = data["title"].as_str().unwrap_or("untitled");
                let save_content = data["content"].as_str().unwrap_or(&content);
                let tags: Vec<String> = data["tags"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();

                if let Ok(Some(path)) = self.memory.save_memory(category, title, save_content, tags, Some("learned")).await {
                    info!("🧠 Watchman Learned: {} -> {:?}", title, path);
                }
            }
        }

        Ok(())
    }

    /// Start the filesystem watcher
    pub async fn start(self: Arc<Self>) -> anyhow::Result<()> {
        info!("👀 Watchman: Starting filesystem observation at {:?}", self.project_root);

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let watchman = self.clone();

        // Spawn watcher in blocking thread
        let project_root = self.project_root.clone();
        std::thread::spawn(move || {
            let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            }).expect("Failed to create watcher");

            watcher.watch(&project_root, RecursiveMode::Recursive)
                .expect("Failed to watch directory");

            // Keep watcher alive
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            }
        });

        info!("👀 Watchman: Ready and watching.");

        // Process events
        while let Some(event) = rx.recv().await {
            match event.kind {
                EventKind::Create(_) => {
                    for path in event.paths {
                        watchman.enqueue("add", path).await;
                    }
                }
                EventKind::Modify(_) => {
                    for path in event.paths {
                        watchman.enqueue("change", path).await;
                    }
                }
                _ => {}
            }

            // Process queue after each batch
            watchman.process_queue().await;
        }

        Ok(())
    }
}
