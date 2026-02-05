use openspore_core::state::AppState;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use chrono::{Local, Utc};
use anyhow::{Result, Context, bail};
use walkdir::WalkDir;
use serde::{Serialize, Deserialize};

pub mod context;

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

    pub fn init_git(&self) {
        if !self.memory_root.join(".git").exists() {
            let _ = std::process::Command::new("git")
                .arg("init")
                .current_dir(&self.memory_root)
                .output();
            let _ = std::process::Command::new("git")
                .args(["add", "."])
                .current_dir(&self.memory_root)
                .output();
            let _ = std::process::Command::new("git")
                .args(["commit", "-m", "Initial memory snapshot"])
                .current_dir(&self.memory_root)
                .output();
        }
    }

    pub fn commit(&self, message: &str) {
        let _ = std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&self.memory_root)
            .output();
        let _ = std::process::Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.memory_root)
            .output();
    }

    /// Ensure the workspace directory structure exists (lines 30-62 in JS)
    pub async fn ensure_structure(&self) -> Result<()> {
        // Create memory root
        if !self.memory_root.exists() {
            fs::create_dir_all(&self.memory_root).await?;
        }

        // Create category directories
        for cat in &self.categories {
            let dir = self.memory_root.join(cat);
            if !dir.exists() {
                fs::create_dir_all(&dir).await?;
            }
        }

        // Create identity core files if missing
        let identity_dir = self.memory_root.join("identity");
        let core_files = [
            ("AGENTS.md", "# Agents\nList of specialized agents and their roles."),
            ("SKILLS.md", "# Available Tools\nDocumentation for the skills available to OpenSpore."),
            ("SOUL.md", "# Agent Soul & Personality\nCore values, tone of voice, and personality traits."),
            ("USER.md", "# User Profile\nInformation about William and his preferences."),
        ];

        for (name, template) in core_files {
            let file_path = identity_dir.join(name);
            if !file_path.exists() {
                fs::write(&file_path, template).await?;
            }
        }

        // Ensure autonomy directories
        for dir in ["autonomy", "autonomy/state", "autonomy/proposals"] {
            let full_path = self.memory_root.join(dir);
            if !full_path.exists() {
                fs::create_dir_all(&full_path).await?;
            }
        }

        Ok(())
    }

    /// Save memory with YAML frontmatter (lines 154-189 in JS)
    /// Exact replication of saveMemory(category, title, content, metadata)
    pub async fn save_memory(
        &self,
        category: &str,
        title: &str,
        content: &str,
        tags: Vec<String>,
        memory_type: Option<&str>,
    ) -> Result<Option<PathBuf>> {
        self.ensure_structure().await?;

        // Fallback to 'context' if category not recognized
        let target_category = if self.categories.contains(&category) {
            category
        } else {
            "context"
        };

        let dir = self.memory_root.join(target_category);
        if !dir.exists() {
            fs::create_dir_all(&dir).await?;
        }

        // v3.5: Protect core identity and operational log files
        let core_identity_files = ["USER", "SOUL", "AGENTS", "SKILLS", "LOGS", "SESSION_SUMMARY"];
        let normalized_title = title.to_uppercase().trim().to_string();
        if (target_category == "identity" || target_category == "context")
            && core_identity_files.contains(&normalized_title.as_str())
        {
            println!("🛡️ Save Blocked: Attempt to clobber protected substrate file \"{}.md\" via save_memory.", normalized_title);
            return Ok(None);
        }

        // Sanitize filename
        let filename = format!(
            "{}.md",
            title.to_lowercase().chars().filter(|c| c.is_alphanumeric() || *c == ' ').collect::<String>().replace(' ', "_")
        );
        let file_path = dir.join(&filename);

        // Build frontmatter (exact format from JS line 177)
        let tags_str = tags.join(", ");
        let mem_type = memory_type.unwrap_or("memory");
        let created = Utc::now().to_rfc3339();

        let file_content = format!(
            "---\ntype: {}\ncreated: {}\ntags: {}\n---\n\n# {}\n\n{}\n",
            mem_type, created, tags_str, title, content
        );

        self.mark_as_internal_write(file_path.clone()).await;
        fs::write(&file_path, &file_content).await?;

        // Versioning: Commit important changes
        if ["preferences", "identity", "knowledge", "memory"].contains(&target_category) {
            self.commit(&format!("Auto-save: {}/{}", target_category, title));
        }

        Ok(Some(file_path))
    }

    /// Append to LOGS.md (non-blocking journal, lines 192-209 in JS)
    pub async fn save_journal(&self, entry: &str) -> Result<()> {
        let path = self.memory_root.join("context").join("LOGS.md");
        self.mark_as_internal_write(path.clone()).await;

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        file.write_all(entry.as_bytes()).await?;
        Ok(())
    }

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

    /// Search across workspace (lines 114-152 in JS)
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
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

        for entry in WalkDir::new(&self.project_root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip ignored directories
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
                        score += 10;
                    }
                    score += lower_content.matches(kw).count().min(10);
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
}

#[derive(Debug, Clone)]
pub struct MemoryItem {
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub content: String,
    pub score: usize,
    pub path: PathBuf,
}
