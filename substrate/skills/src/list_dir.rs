//! List Directory Skill (Core)

use super::Skill;
use async_trait::async_trait;
use tokio::fs;

pub struct ListDirSkill;

#[async_trait]
impl Skill for ListDirSkill {
    fn name(&self) -> &'static str { "list_dir" }

    fn description(&self) -> &'static str {
        "List contents of a directory. Usage: [LIST_DIR: \"/path/to/dir\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let raw_path = args.trim().trim_matches('"').trim_matches('\'');
        let path = openspore_core::path_utils::expand_tilde(raw_path);

        let mut entries = fs::read_dir(&path)
            .await
            .map_err(|e| format!("Failed to read dir {}: {}", path, e))?;

        let mut result = String::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let file_type = if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                "📁"
            } else {
                "📄"
            };
            result.push_str(&format!("{} {}\n", file_type, entry.file_name().to_string_lossy()));
        }

        Ok(result)
    }
}
