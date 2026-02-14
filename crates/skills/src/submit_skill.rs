//! Submit Skill (Core)
//! Port of submit_code.js - Deploys modules to autonomy sandbox.

use super::Skill;
use async_trait::async_trait;
use tokio::fs;

pub struct SubmitSkill;

#[async_trait]
impl Skill for SubmitSkill {
    fn name(&self) -> &'static str { "submit_skill" }

    fn description(&self) -> &'static str {
        "Deploy a logic module to the skills directory in OpenClaw format (folder + SKILL.md). Returns JSON with success and message. Usage: [SUBMIT_SKILL: {\"name\": \"skill_name\", \"description\": \"...\", \"instructions\": \"...\", \"code\": \"...\"}]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let (name, description, instructions, code) = if let Some(json_args) = crate::utils::try_parse_json(args) {
             let n = crate::utils::get_str_field(&json_args, "name").or_else(|| crate::utils::get_str_field(&json_args, "filename")).ok_or("JSON missing 'name' or 'filename'")?;
             let d = crate::utils::get_str_field(&json_args, "description").unwrap_or_else(|| format!("Self-generated skill: {}", n));
             let i = crate::utils::get_str_field(&json_args, "instructions").unwrap_or_else(|| "No special instructions provided.".to_string());
             let c = crate::utils::get_str_field(&json_args, "code").ok_or("JSON missing 'code'")?;
             (n, d, i, c)
        } else {
            let separator = "|||";
            let parts: Vec<&str> = args.split(separator).collect();

            if parts.len() < 2 {
                let res = serde_json::json!({ "success": false, "error": "Usage: [SUBMIT_SKILL: {\"name\": \"...\", \"description\": \"...\", \"instructions\": \"...\", \"code\": \"...\"}]" });
                return Ok(res.to_string());
            }

            let name = crate::utils::sanitize_path(parts[0]);
            let code = parts[1..].join(separator);
            let description = format!("Self-generated skill: {}", name);
            let instructions = "No special instructions provided.".to_string();
            (name, description, instructions, code)
        };

        // Path-pointing detection
        let trimmed_code = code.trim();
        if (trimmed_code.starts_with('/') || trimmed_code.starts_with('~') || trimmed_code.starts_with("./"))
           && !trimmed_code.contains('\n') && !trimmed_code.contains(' ') && !trimmed_code.contains('{') {
                let res = serde_json::json!({ "success": false, "error": format!("Blocked by Self-Defense: SUBMIT_SKILL requires the literal content of the code, not a path to it. You provided: '{}'", trimmed_code) });
                return Ok(res.to_string());
        }

        // Validation (Refined for real-world utility)
        let mut issues = Vec::new();
        let dangerous_keywords = ["process.exit", "process.kill", ".env", "rm -rf", "chown", "chmod"];
        for kw in dangerous_keywords {
            if code.contains(kw) {
                issues.push(format!("Dangerous pattern: {}", kw));
            }
        }

        if !issues.is_empty() {
             let res = serde_json::json!({ "success": false, "error": format!("Blocked by Self-Defense: {}", issues.join(", ")) });
             return Ok(res.to_string());
        }

        let root = openspore_core::path_utils::get_app_root();
        let skills_dir = root.join("skills");
        let skill_folder = skills_dir.join(&name);

        // Security check: ensure path is within skills directory
        if !skill_folder.starts_with(&skills_dir) {
             let res = serde_json::json!({ "success": false, "error": "Error: Skill deployment is restricted to ~/.openspore/skills/" });
             return Ok(res.to_string());
        }

        if let Err(e) = fs::create_dir_all(&skill_folder).await {
            let res = serde_json::json!({ "success": false, "error": format!("Failed to create skill directory: {}", e) });
            return Ok(res.to_string());
        }

        // 1. Write SKILL.md
        let skill_md_content = format!("---\nname: {}\ndescription: {}\n---\n\n{}\n", name, description, instructions);
        let skill_md_path = skill_folder.join("SKILL.md");
        fs::write(&skill_md_path, skill_md_content).await.map_err(|e| format!("Failed to write SKILL.md: {}", e))?;

        // 2. Write logic script (main.js by default)
        // Heuristic extension detection
        let ext = if code.contains("module.exports") || code.contains("require(") { "js" } else if code.contains("#!/bin/bash") || code.contains("#!/bin/sh") { "sh" } else { "js" };
        let script_filename = format!("main.{}", ext);
        let script_path = skill_folder.join(script_filename);

        // Unescape code if needed
        let final_code = if crate::utils::try_parse_json(args).is_some() {
            code
        } else {
            crate::utils::unescape(&code)
        };

        // NEW: Sanitize file content (Remove Markdown Blocks)
        let mut content_to_write = final_code.trim();
        let mut clean_content = content_to_write.to_string();
        if content_to_write.starts_with("```") {
            let lines: Vec<&str> = content_to_write.lines().collect();
            if lines.len() >= 2 {
                let start = if lines[0].starts_with("```") { 1 } else { 0 };
                let end = if lines.last().unwrap_or(&"").starts_with("```") { lines.len() - 1 } else { lines.len() };
                clean_content = lines[start..end].join("\n");
            }
        }
        content_to_write = &clean_content;

        match fs::write(&script_path, content_to_write).await {
            Ok(_) => {
                let res = serde_json::json!({
                    "success": true,
                    "name": name,
                    "path": skill_folder.display().to_string(),
                    "message": format!("OpenClaw Skill '{}' deployed to {} and validated.", name, skill_folder.display())
                });
                Ok(res.to_string())
            },
            Err(e) => {
                let res = serde_json::json!({ "success": false, "error": format!("Failed to write script file: {}", e) });
                Ok(res.to_string())
            }
        }
    }
}
