//! Purge Skill (Core)
//! Cleans up old logs and raw context to keep the substrate slim.

use super::Skill;
use async_trait::async_trait;
use tokio::fs;
use chrono::{Duration, Utc, DateTime};

pub struct PurgeSkill;

#[async_trait]
impl Skill for PurgeSkill {
    fn name(&self) -> &'static str { "purge" }

    fn description(&self) -> &'static str {
        "Clean up old context logs. Returns JSON with success and deleted_count. Usage: [PURGE: \"days\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let sanitized = crate::utils::sanitize_path(args);
        let days: i64 = sanitized.parse().unwrap_or(7);
        let cutoff = Utc::now() - Duration::days(days);

        // Get context directory
        let root = openspore_core::path_utils::get_app_root();
        let context_dir = root.join("workspace/context");

        if !context_dir.exists() {
            let res = serde_json::json!({ "success": true, "deleted_count": 0, "message": "No context directory found to purge." });
            return Ok(res.to_string());
        }

        let mut deleted_count = 0;
        let mut entries = match fs::read_dir(&context_dir).await {
            Ok(e) => e,
            Err(e) => {
                let res = serde_json::json!({ "success": false, "error": e.to_string() });
                return Ok(res.to_string());
            }
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() {
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

        let res = serde_json::json!({
            "success": true,
            "deleted_count": deleted_count
        });
        Ok(res.to_string())
    }
}
