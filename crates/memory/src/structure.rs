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
