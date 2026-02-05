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
        let (path, target, replacement) = if let Some(json_args) = crate::utils::try_parse_json(args) {
            let p = crate::utils::get_str_field(&json_args, "path").ok_or("JSON missing 'path'")?;
            let t = crate::utils::get_str_field(&json_args, "target").ok_or("JSON missing 'target'")?;
            let r = crate::utils::get_str_field(&json_args, "replacement").unwrap_or_default();
            (p, t, r)
        } else {
            // Parse args: path --target="..." --replacement="..."
            let target_marker = "--target=";
            let replacement_marker = "--replacement=";

            let path_idx = args.find(target_marker).ok_or("Missing --target=")?;
            let target_idx = args.find(replacement_marker).ok_or("Missing --replacement=")?;

            let raw_path = args[..path_idx].trim().trim_matches('"').trim_matches('\'').trim();
            // path expansion happens later

            let target_part = &args[path_idx + target_marker.len()..target_idx].trim();
            let replacement_part = &args[target_idx + replacement_marker.len()..].trim();

            let t = if (target_part.starts_with('"') && target_part.ends_with('"')) ||
                            (target_part.starts_with('\'') && target_part.ends_with('\'')) {
                target_part[1..target_part.len()-1].to_string()
            } else {
                target_part.to_string()
            };

            let r = if (replacement_part.starts_with('"') && replacement_part.ends_with('"')) ||
                                (replacement_part.starts_with('\'') && replacement_part.ends_with('\'')) {
                replacement_part[1..replacement_part.len()-1].to_string()
            } else {
                replacement_part.to_string()
            };
            (raw_path.to_string(), t, r)
        };

        let path = openspore_core::path_utils::expand_tilde(&path);

        if path.is_empty() { return Err("Empty path".to_string()); }

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Could not read {}: {}", path, e))?;

        // Unescape newlines for target and replacement
        let target_unescaped = target.replace("\\n", "\n");
        let replacement_unescaped = replacement.replace("\\n", "\n");

        if !content.contains(&target_unescaped) {
            return Err(format!("Target text not found in {}", path));
        }

        let new_content = content.replace(&target_unescaped, &replacement_unescaped);
        fs::write(&path, new_content)
            .await
            .map_err(|e| format!("Could not write {}: {}", path, e))?;

        Ok(format!("✅ Successfully edited {}", path))
    }
}
