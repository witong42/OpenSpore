//! Write File Skill (Core)
//! Robust version with better handle of content and quotes.

use super::Skill;
use async_trait::async_trait;
use tokio::fs;
use std::path::Path;

pub struct WriteFileSkill;

#[async_trait]
impl Skill for WriteFileSkill {
    fn name(&self) -> &'static str { "write_file" }

    fn description(&self) -> &'static str {
        "Write content to a file. Returns JSON with success, bytes_written, and path. Usage: [WRITE_FILE: \"/path\" --content=\"content\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let content_marker = "--content=";
        let (path_part, content_part) = if let Some(idx) = args.find(content_marker) {
            (&args[..idx], &args[idx + content_marker.len()..])
        } else {
            let res = serde_json::json!({
                "success": false,
                "error": "Usage: [WRITE_FILE: \"/path\" --content=\"content\"]"
            });
            return Ok(res.to_string());
        };

        let path = crate::utils::sanitize_path(path_part);

        let mut content = content_part.trim();
        if (content.starts_with('"') && content.ends_with('"')) ||
           (content.starts_with('\'') && content.ends_with('\'')) {
            if content.len() >= 2 {
                content = &content[1..content.len()-1];
            }
        }

        if path.is_empty() {
            let res = serde_json::json!({ "success": false, "error": "Empty file path." });
            return Ok(res.to_string());
        }

        // SAFE MODE CHECK
        if crate::utils::is_safe_mode_active() && crate::utils::is_path_protected(&path) {
            let res = serde_json::json!({
                "success": false,
                "error": "SAFE_MODE_ENABLED: Modifying the crates (logic) is forbidden."
            });
            return Ok(res.to_string());
        }

        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).await.ok();
        }

        let final_content = crate::utils::unescape(content);
        let bytes_written = final_content.len();

        match fs::write(&path, final_content).await {
            Ok(_) => {
                let res = serde_json::json!({
                    "success": true,
                    "bytes_written": bytes_written,
                    "path": path
                });
                Ok(res.to_string())
            },
            Err(e) => {
                let res = serde_json::json!({
                    "success": false,
                    "error": format!("Failed to write {}: {}", path, e),
                    "path": path
                });
                Ok(res.to_string())
            }
        }
    }
}
