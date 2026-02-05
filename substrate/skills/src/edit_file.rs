//! Edit File Skill (Core)
//! Allows targeted replacement of text in a file.

use super::Skill;
use async_trait::async_trait;
use tokio::fs;

pub struct EditFileSkill;

#[async_trait]
impl Skill for EditFileSkill {
    fn name(&self) -> &'static str { "edit_file" }

    fn description(&self) -> &'static str {
        "Replace targeted text in a file. Usage: [EDIT_FILE: \"/path\" --target=\"old text\" --replacement=\"new text\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        // Parse args: path --target="..." --replacement="..."
        let target_marker = "--target=";
        let replacement_marker = "--replacement=";

        let path_idx = args.find(target_marker).ok_or("Missing --target=")?;
        let target_idx = args.find(replacement_marker).ok_or("Missing --replacement=")?;

        let path = args[..path_idx].trim().trim_matches('"').trim_matches('\'').trim();
        let target_part = &args[path_idx + target_marker.len()..target_idx].trim();
        let replacement_part = &args[target_idx + replacement_marker.len()..].trim();

        let target = if (target_part.starts_with('"') && target_part.ends_with('"')) ||
                        (target_part.starts_with('\'') && target_part.ends_with('\'')) {
            &target_part[1..target_part.len()-1]
        } else {
            target_part
        };

        let replacement = if (replacement_part.starts_with('"') && replacement_part.ends_with('"')) ||
                             (replacement_part.starts_with('\'') && replacement_part.ends_with('\'')) {
            &replacement_part[1..replacement_part.len()-1]
        } else {
            replacement_part
        };

        if path.is_empty() { return Err("Empty path".to_string()); }

        let content = fs::read_to_string(path)
            .await
            .map_err(|e| format!("Could not read {}: {}", path, e))?;

        if !content.contains(target) {
            return Err(format!("Target text not found in {}", path));
        }

        let new_content = content.replace(target, replacement);
        fs::write(path, new_content)
            .await
            .map_err(|e| format!("Could not write {}: {}", path, e))?;

        Ok(format!("✅ Successfully edited {}", path))
    }
}
