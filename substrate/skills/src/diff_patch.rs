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
        "Apply a GNU-style diff/patch to a file. Usage: [DIFF_PATCH: \"/path/to/file|||patch_text\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let separator = "|||";
        let parts: Vec<&str> = args.split(separator).collect();

        if parts.len() < 2 {
            return Err("Usage: [DIFF_PATCH: \"/path/to/file|||patch_text\"]".to_string());
        }

        let raw_path = parts[0].trim().trim_matches('"').trim_matches('\'').trim();
        let path_str = openspore_core::path_utils::expand_tilde(raw_path);
        let path = Path::new(&path_str);

        let joined_patch = parts[1..].join(separator);
        let patch_text = joined_patch.trim();

        if !path.exists() {
            return Err(format!("File not found: {}", path.display()));
        }

        let old_content = fs::read_to_string(path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let patch = Patch::from_str(patch_text)
            .map_err(|e| format!("Invalid patch format: {}", e))?;

        match diffy::apply(&old_content, &patch) {
            Ok(new_content) => {
                fs::write(path, new_content)
                    .await
                    .map_err(|e| format!("Failed to write patched file: {}", e))?;
                Ok(format!("✅ Successfully patched {}", path.display()))
            },
            Err(e) => {
                Err(format!("❌ Failed to apply patch: {:?}", e))
            }
        }
    }
}
