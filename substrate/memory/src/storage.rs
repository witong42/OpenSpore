use crate::MemorySystem;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use chrono::Utc;
use anyhow::Result;

impl MemorySystem {
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

        // Normalize category to lowercase
        let category_clean = category.trim().to_lowercase();

        // Fallback to 'context' if category not recognized
        let target_category = if self.categories.contains(&category_clean.as_str()) {
            category_clean
        } else {
            "context".to_string()
        };

        let dir = self.memory_root.join(&target_category);
        if !dir.exists() {
            fs::create_dir_all(&dir).await?;
        }

        // v3.5: Protect core identity and operational log files
        let core_identity_files = ["USER", "SOUL", "AGENTS", "SKILLS", "LOGS", "SESSION_SUMMARY"];
        let normalized_title = title.to_uppercase().trim().to_string();
        if (target_category == "identity" || target_category == "context")
            && core_identity_files.contains(&normalized_title.as_str())
        {
            tracing::warn!("🛡️ Save Blocked: Attempt to clobber protected substrate file \"{}.md\" via save_memory.", normalized_title);
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
        if ["preferences", "identity", "knowledge", "memory"].contains(&target_category.as_str()) {
            self.commit(&format!("Auto-save: {}/{}", target_category, title));
        }

        Ok(Some(file_path))
    }

    /// Append to LOGS.md (non-blocking journal, lines 192-209 in JS)
    pub async fn save_journal(&self, entry: &str) -> Result<()> {
        let path = self.memory_root.join("context").join("LOGS.md");

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }

        self.mark_as_internal_write(path.clone()).await;

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        file.write_all(entry.as_bytes()).await?;
        Ok(())
    }
}
