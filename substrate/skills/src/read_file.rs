//! Read File Skill (Core)

use super::Skill;
use async_trait::async_trait;
use tokio::fs;

pub struct ReadFileSkill;

#[async_trait]
impl Skill for ReadFileSkill {
    fn name(&self) -> &'static str { "read_file" }

    fn description(&self) -> &'static str {
        "Read contents of a file. Usage: [READ_FILE: \"/path/to/file.txt\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let path = args.trim().trim_matches('"').trim_matches('\'');
        fs::read_to_string(path)
            .await
            .map_err(|e| format!("Failed to read {}: {}", path, e))
    }
}
