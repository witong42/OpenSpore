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
        "Replace targeted text in a file. Returns JSON with success, message, and path. Usage: [EDIT_FILE: \"/path\" --target=\"old text\" --replacement=\"new text\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let (path_raw, target, replacement) = if let Some(json_args) = crate::utils::try_parse_json(args) {
            let p = crate::utils::get_str_field(&json_args, "path").ok_or("JSON missing 'path'")?;
            let t = crate::utils::get_str_field(&json_args, "target").ok_or("JSON missing 'target'")?;
            let r = crate::utils::get_str_field(&json_args, "replacement").unwrap_or_default();
            (p, t, r)
        } else {
            let target_marker = "--target=";
            let replacement_marker = "--replacement=";

            let path_idx = if let Some(idx) = args.find(target_marker) { idx } else {
                let res = serde_json::json!({ "success": false, "error": "Missing --target=" });
                return Ok(res.to_string());
            };
            let target_idx = if let Some(idx) = args.find(replacement_marker) { idx } else {
                let res = serde_json::json!({ "success": false, "error": "Missing --replacement=" });
                return Ok(res.to_string());
            };

            let raw_path = args[..path_idx].trim().trim_matches('"').trim_matches('\'').trim();
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

        let path = crate::utils::sanitize_path(&path_raw);
        if path.is_empty() {
            let res = serde_json::json!({ "success": false, "error": "Empty path" });
            return Ok(res.to_string());
        }

        let content = match fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => {
                let res = serde_json::json!({ "success": false, "error": format!("Could not read {}: {}", path, e), "path": path });
                return Ok(res.to_string());
            }
        };

        let target_unescaped = crate::utils::unescape(&target);
        let replacement_unescaped = crate::utils::unescape(&replacement);

        if !content.contains(&target_unescaped) {
             let res = serde_json::json!({ "success": false, "error": format!("Target text not found in {}", path), "path": path });
             return Ok(res.to_string());
        }

        let new_content = content.replace(&target_unescaped, &replacement_unescaped);
        match fs::write(&path, new_content).await {
            Ok(_) => {
                let res = serde_json::json!({ "success": true, "message": format!("Successfully edited {}", path), "path": path });
                Ok(res.to_string())
            },
            Err(e) => {
                let res = serde_json::json!({ "success": false, "error": format!("Could not write {}: {}", path, e), "path": path });
                Ok(res.to_string())
            }
        }
    }
}
