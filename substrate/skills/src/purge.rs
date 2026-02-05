//! Purge Skill (Core)
//! Cleans up old logs and raw context to keep the substrate slim.

use super::Skill;
use async_trait::async_trait;
use tokio::fs;
use std::path::Path;
use chrono::{Duration, Utc, DateTime};

pub struct PurgeSkill;

#[async_trait]
impl Skill for PurgeSkill {
    fn name(&self) -> &'static str { "purge" }

    fn description(&self) -> &'static str {
        "Clean up old context logs. Usage: [PURGE: \"days\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let days: i64 = args.trim().trim_matches('"').trim_matches('\'').parse().unwrap_or(7);
        let cutoff = Utc::now() - Duration::days(days);

        // Get context directory
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let context_dir = Path::new(&home).join(".openspore/workspace/context");

        if !context_dir.exists() {
            return Ok("No context directory found to purge.".to_string());
        }

        let mut deleted_count = 0;
        let mut entries = fs::read_dir(&context_dir).await.map_err(|e| e.to_string())?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            if path.is_file() {
                // Don't purge critical files
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name == "LOGS.md" || name == "session_summary.md" {
                    continue;
                }

                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        let dt: DateTime<Utc> = modified.into();
                        if dt < cutoff {
                            let _ = fs::remove_file(path).await;
                            deleted_count += 1;
                        }
                    }
                }
            }
        }

        Ok(format!("♻️ Purge complete. Removed {} old context items.", deleted_count))
    }
}
