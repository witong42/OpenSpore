//! OpenSpore Watchman - Modular Filesystem Observer
//!
//! This module is organized into:
//! - types: Core data structures (WatchEvent)
//! - ignore: Ignore rule management
//! - queue: Event queue management
//! - processing: Event processing and learning
//! - watcher: Filesystem watching

mod types;
mod ignore;
mod queue;
mod processing;
mod watcher;

// Re-export public types
pub use types::WatchEvent;

use std::path::PathBuf;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use openspore_core::config::AppConfig;
use openspore_memory::MemorySystem;
use openspore_brain::Brain;

/// Watchman - filesystem observer that triggers learning
pub struct Watchman {
    pub project_root: PathBuf,
    pub memory: MemorySystem,
    pub brain: Brain,
    pub ignore_rules: HashSet<String>,
    pub allowed_extensions: HashSet<String>,
    pub queue: Arc<Mutex<Vec<WatchEvent>>>,
}

impl Watchman {
    pub fn new(config: AppConfig, brain: Brain, memory: MemorySystem) -> Self {

        let mut watchman = Self {
            project_root: config.project_root.clone(),
            memory,
            brain,
            ignore_rules: HashSet::from([
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ]),
            allowed_extensions: HashSet::from([
                ".md".to_string(),
                ".txt".to_string(),
                ".json".to_string(),
            ]),
            queue: Arc::new(Mutex::new(Vec::new())),
        };

        watchman.load_ignore_rules();
        info!("👀 Watchman initialized");
        watchman
    }
}
