//! List Directory Skill (Core)

use super::Skill;
use async_trait::async_trait;
use tokio::fs;

pub struct ListDirSkill;

#[async_trait]
impl Skill for ListDirSkill {
    fn name(&self) -> &'static str { "list_dir" }

    fn description(&self) -> &'static str {
        "List contents of a directory. Returns JSON with success, path, and items (name/type). Usage: [LIST_DIR: \"/path/to/dir\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let path = crate::utils::sanitize_path(args);

        match fs::read_dir(&path).await {
            Ok(mut entries) => {
                let mut items = Vec::new();
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
                    items.push(serde_json::json!({
                        "name": name,
                        "type": if is_dir { "directory" } else { "file" }
                    }));
                }
                let res = serde_json::json!({
                    "success": true,
                    "path": path,
                    "items": items
                });
                Ok(res.to_string())
            },
            Err(e) => {
                let res = serde_json::json!({
                    "success": false,
                    "error": format!("Failed to read dir {}: {}", path, e),
                    "path": path
                });
                Ok(res.to_string())
            }
        }
    }
}
