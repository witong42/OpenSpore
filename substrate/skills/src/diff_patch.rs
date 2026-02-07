//! Diff/Patch Skill (Core)
//! Uses diffy for GNU-style patching.

use super::Skill;
use async_trait::async_trait;
use tokio::fs;
use std::path::Path;
use diffy::Patch;

pub struct DiffPatchSkill;

#[async_trait]
impl Skill for DiffPatchSkill {
    fn name(&self) -> &'static str { "diff_patch" }

    fn description(&self) -> &'static str {
        "Apply a GNU-style diff/patch to a file. Returns JSON with success, message, and path. Usage: [DIFF_PATCH: \"/path/to/file|||patch_text\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let separator = "|||";
        let parts: Vec<&str> = args.split(separator).collect();

        if parts.len() < 2 {
            let res = serde_json::json!({
                "success": false,
                "error": "Usage: [DIFF_PATCH: \"/path/to/file|||patch_text\"]"
            });
            return Ok(res.to_string());
        }

        let raw_path = parts[0].trim().trim_matches('"').trim_matches('\'').trim();
        let path_str = openspore_core::path_utils::expand_tilde(raw_path);

        // SAFE MODE CHECK
        if crate::utils::is_safe_mode_active() && crate::utils::is_path_protected(&path_str) {
            let res = serde_json::json!({
                "success": false,
                "error": "SAFE_MODE_ENABLED: Modifying the crates (logic) is forbidden."
            });
            return Ok(res.to_string());
        }

        let path = Path::new(&path_str);

        let joined_patch = parts[1..].join(separator);
        let mut patch_text = crate::utils::unescape(joined_patch.trim());

        // Sanitize: Remove Markdown code blocks if present
        if patch_text.starts_with("```") {
            let lines: Vec<&str> = patch_text.lines().collect();
            if lines.len() >= 2 {
                // remove first and last line if they look like markers
                let start = if lines[0].starts_with("```") { 1 } else { 0 };
                let end = if lines.last().unwrap_or(&"").starts_with("```") { lines.len() - 1 } else { lines.len() };
                patch_text = lines[start..end].join("\n");
            }
        }
        patch_text = patch_text.trim().to_string();

        // Ensure trailing newline for strict diff parsers
        if !patch_text.ends_with('\n') {
            patch_text.push('\n');
        }

        if !path.exists() {
            let res = serde_json::json!({
                "success": false,
                "error": format!("File not found: {}", path_str),
                "path": path_str
            });
            return Ok(res.to_string());
        }

        let old_content = match fs::read_to_string(path).await {
            Ok(c) => c,
            Err(e) => {
                let res = serde_json::json!({ "success": false, "error": format!("Failed to read file: {}", e), "path": path_str });
                return Ok(res.to_string());
            }
        };

        let patch = match Patch::from_str(&patch_text) {
            Ok(p) => p,
            Err(e) => {
                let res = serde_json::json!({ "success": false, "error": format!("Invalid patch format: {}", e), "path": path_str });
                return Ok(res.to_string());
            }
        };

        match diffy::apply(&old_content, &patch) {
            Ok(new_content) => {
                if let Err(e) = fs::write(path, new_content).await {
                    let res = serde_json::json!({ "success": false, "error": format!("Failed to write patched file: {}", e), "path": path_str });
                    return Ok(res.to_string());
                }
                let res = serde_json::json!({
                    "success": true,
                    "message": format!("Successfully patched {}", path_str),
                    "path": path_str
                });
                Ok(res.to_string())
            },
            Err(e) => {
                let res = serde_json::json!({
                    "success": false,
                    "error": format!("Failed to apply patch: {:?}", e),
                    "path": path_str
                });
                Ok(res.to_string())
            }
        }
    }
}
