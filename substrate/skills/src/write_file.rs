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
        "Write content to a file. Usage: [WRITE_FILE: \"/path\" --content=\"content\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        // Robust splitting: find the first --content=
        let content_marker = "--content=";
        let (path_part, content_part) = if let Some(idx) = args.find(content_marker) {
            (&args[..idx], &args[idx + content_marker.len()..])
        } else {
            return Err("Usage: [WRITE_FILE: \"/path\" --content=\"content\"]".to_string());
        };

        let raw_path = path_part.trim().trim_matches('"').trim_matches('\'').trim();
        let path = openspore_core::path_utils::expand_tilde(raw_path);

        // Content might be wrapped in quotes by the LLM
        let mut content = content_part.trim();
        if (content.starts_with('"') && content.ends_with('"')) ||
           (content.starts_with('\'') && content.ends_with('\'')) {
            if content.len() >= 2 {
                content = &content[1..content.len()-1];
            }
        }

        if path.is_empty() {
            return Err("Empty file path.".to_string());
        }

        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).await.ok();
        }

        // Unescape newlines
        let final_content = content.replace("\\n", "\n");

        fs::write(&path, final_content)
            .await
            .map_err(|e| format!("Failed to write {}: {}", path, e))?;

        Ok(format!("✅ Written {} bytes to {}", content.len(), path))
    }
}
