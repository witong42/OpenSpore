use crate::MemorySystem;
use tokio::fs;
use anyhow::Result;

impl MemorySystem {
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
        for dir in ["autonomy/proposals"] {
            let full_path = self.memory_root.join(dir);
            if !full_path.exists() {
                fs::create_dir_all(&full_path).await?;
            }
        }

        Ok(())
    }
}
