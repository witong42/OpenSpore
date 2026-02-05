use crate::{MemorySystem, types::{MemoryItem, SearchResult}};
use walkdir::WalkDir;
use anyhow::Result;
use std::path::Path;

impl MemorySystem {
    /// Get all memories from a category (lines 211-228 in JS)
    pub fn get_memories(&self, category: &str) -> Vec<MemoryItem> {
        let dir = self.memory_root.join(category);
        if !dir.exists() {
            return vec![];
        }

        let mut memories = vec![];
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        memories.push(MemoryItem {
                            filename: path.file_name().unwrap().to_string_lossy().to_string(),
                            content,
                        });
                    }
                }
            }
        }

        // Sort by filename to ensure chronological order (for Exchange_TIMESTAMP.md)
        memories.sort_by(|a, b| a.filename.cmp(&b.filename));
        memories
    }

    /// Wrapper for semantic search (placeholdered by keyword search for now)
    pub async fn search_memories(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        self.search(query, limit).await.unwrap_or_default()
    }

    /// Search within a specific path
    pub async fn search_in_path(&self, query: &str, base_path: &Path, limit: usize) -> Result<Vec<SearchResult>> {
        if query.is_empty() {
            return Ok(vec![]);
        }

        let keywords: Vec<String> = query
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .map(|s| s.to_string())
            .collect();

        if keywords.is_empty() {
            return Ok(vec![]);
        }

        let mut results: Vec<SearchResult> = vec![];

        // Ensure base_path exists
        if !base_path.exists() {
             return Err(anyhow::anyhow!("Search path does not exist: {}", base_path.display()));
        }

        for entry in WalkDir::new(base_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip ignored directories (only check leaf name)
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    if self.ignore_dirs.contains(name.to_string_lossy().as_ref()) {
                        continue;
                    }
                }
            }

            if !path.is_file() {
                continue;
            }

            // Check extension
            let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy().to_lowercase())).unwrap_or_default();
            if !self.allowed_extensions.contains(&ext.as_str()) {
                continue;
            }

            // Skip large files (100KB)
            if let Ok(meta) = std::fs::metadata(path) {
                if meta.len() > 100 * 1024 {
                    continue;
                }
            }

            if let Ok(content) = std::fs::read_to_string(path) {
                let lower_content = content.to_lowercase();
                let filename = path.file_name().unwrap().to_string_lossy().to_lowercase();

                let mut score = 0;
                for kw in &keywords {
                    if filename.contains(kw) {
                        score += 50; // Boost filename matches
                    }
                    score += lower_content.matches(kw).count().min(20);
                }

                if score > 0 {
                    results.push(SearchResult {
                        title: path.file_name().unwrap().to_string_lossy().to_string(),
                        content,
                        score,
                        path: path.to_path_buf(),
                    });
                }
            }
        }

        results.sort_by(|a, b| b.score.cmp(&a.score));
        Ok(results.into_iter().take(limit).collect())
    }

    /// Search across workspace (lines 114-152 in JS)
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.search_in_path(query, &self.project_root, limit).await
    }
}
