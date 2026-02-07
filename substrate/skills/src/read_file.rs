//! Read File Skill (Core)

use super::Skill;
use async_trait::async_trait;
use tokio::fs;

pub struct ReadFileSkill;

#[async_trait]
impl Skill for ReadFileSkill {
    fn name(&self) -> &'static str { "read_file" }

    fn description(&self) -> &'static str {
        "Read contents of a file. Returns JSON with success, content, and path. Usage: [READ_FILE: \"/path/to/file.txt\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let path = crate::utils::sanitize_path(args);

        match fs::read_to_string(&path).await {
            Ok(content) => {
                let result = serde_json::json!({
                    "success": true,
                    "content": content,
                    "path": path
                });
                Ok(result.to_string())
            },
            Err(e) => {
                let result = serde_json::json!({
                    "success": false,
                    "error": format!("Failed to read {}: {}", path, e),
                    "path": path
                });
                Ok(result.to_string())
            }
        }
    }
}
