//! OpenSpore Memory - Modular Memory System
//!
//! This module is organized into:
//! - types: Core data structures (MemoryItem, SearchResult)
//! - git: Version control operations
//! - structure: Directory initialization
//! - storage: Saving memories and journal entries
//! - retrieval: Searching and retrieving memories
//! - context: Context management (already modular)

mod types;
mod git;
mod structure;
mod storage;
mod retrieval;

pub mod context;

// Re-export public types
pub use types::{MemoryItem, SearchResult};

use openspore_core::state::AppState;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Exact port of opensporejs/src/memory.js MemorySystem class
#[derive(Clone)]
pub struct MemorySystem {
    pub project_root: PathBuf,
    pub memory_root: PathBuf,
    pub categories: Vec<&'static str>,
    pub allowed_extensions: Vec<&'static str>,
    pub ignore_dirs: HashSet<&'static str>,
    pub recently_written: Arc<Mutex<HashSet<PathBuf>>>,
}

impl MemorySystem {
    pub fn new(state: &AppState) -> Self {
        let root = state.config.project_root.clone();
        let memory_root = root.join("workspace");
        let categories = vec!["preferences", "identity", "knowledge", "context", "memory"];

        let mem = Self {
            project_root: root,
            memory_root,
            categories,
            allowed_extensions: vec![
                ".md", ".txt", ".json", ".yaml", ".yml",
                ".js", ".ts", ".py", ".rs", ".go", ".c", ".cpp", ".h", ".sh"
            ],
            ignore_dirs: HashSet::from([
                "node_modules", "target", ".git", "dist", "build", "coverage", "__pycache__", ".next", "bin", "lib"
            ]),
            recently_written: Arc::new(Mutex::new(HashSet::new())),
        };
        mem.init_git();
        mem
    }

    pub async fn mark_as_internal_write(&self, path: PathBuf) {
        let mut written = self.recently_written.lock().await;
        written.insert(path.clone());

        let written_clone = Arc::clone(&self.recently_written);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let mut written = written_clone.lock().await;
            written.remove(&path);
        });
    }

    pub async fn is_internal_write(&self, path: &Path) -> bool {
        let written = self.recently_written.lock().await;
        written.contains(path)
    }

    pub fn clone_memory(&self) -> Self {
        self.clone()
    }
}
