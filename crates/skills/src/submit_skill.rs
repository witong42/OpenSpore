//! Submit Skill (Core)
//! Port of submit_code.js - Deploys modules to autonomy sandbox.

use super::Skill;
use async_trait::async_trait;
use tokio::fs;

pub struct SubmitSkill;

const FORBIDDEN_PATTERNS: &[&str] = &[
    "child_process",
    "exec(",
    "spawn(",
    "rm -rf",
    ".env",
    "process.exit",
    "process.kill",
];

const ALLOWED_MODULES: &[&str] = &[
    "fs", "path", "os", "crypto", "util", "url", "querystring",
    "http", "https", "buffer", "stream", "zlib", "axios"
];

#[async_trait]
impl Skill for SubmitSkill {
    fn name(&self) -> &'static str { "submit_skill" }

    fn description(&self) -> &'static str {
        "Deploy a self-generated logic module (JS/Shell) to the skills directory for immediate use. Returns JSON with success and message. Usage: [SUBMIT_SKILL: \"filename.js|||literal_code_content\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let (filename, code) = if let Some(json_args) = crate::utils::try_parse_json(args) {
             let f = crate::utils::get_str_field(&json_args, "filename").ok_or("JSON missing 'filename'")?;
             let c = crate::utils::get_str_field(&json_args, "code").ok_or("JSON missing 'code'")?;
             (f, c)
        } else {
            let separator = "|||";
            let parts: Vec<&str> = args.split(separator).collect();

            if parts.len() < 2 {
                let res = serde_json::json!({ "success": false, "error": "Usage: [SUBMIT_SKILL: {\"filename\": \"...\", \"code\": \"...\"}] or [SUBMIT_SKILL: \"filename|||code\"]" });
                return Ok(res.to_string());
            }

            let filename = crate::utils::sanitize_path(parts[0]);
            let code = parts[1..].join(separator);
            (filename, code)
        };

        // Path-pointing detection (common LLM mistake)
        let trimmed_code = code.trim();
        if (trimmed_code.starts_with('/') || trimmed_code.starts_with('~') || trimmed_code.starts_with("./"))
           && !trimmed_code.contains('\n') && !trimmed_code.contains(' ') && !trimmed_code.contains('{') {
                let res = serde_json::json!({ "success": false, "error": format!("Blocked by Self-Defense: SUBMIT_SKILL requires the literal content of the code, not a path to it. You provided: '{}'", trimmed_code) });
                return Ok(res.to_string());
        }

        // Validation
        let mut issues = Vec::new();

        // 1. Forbidden Patterns
        for pattern in FORBIDDEN_PATTERNS {
            if code.contains(pattern) {
                issues.push(format!("Forbidden pattern: {}", pattern));
            }
        }

        // 2. Simple require check (heuristic)
        if code.contains("require(") {
             let mut cursor = 0;
             while let Some(start) = code[cursor..].find("require(") {
                 let start_idx = cursor + start + 8;
                 let quote = code.chars().nth(start_idx);
                 if let Some(q) = quote {
                     if q == '"' || q == '\'' {
                         if let Some(end) = code[start_idx+1..].find(q) {
                             let module_name = &code[start_idx+1..start_idx+1+end];
                             if !ALLOWED_MODULES.contains(&module_name) && !module_name.starts_with("./") {
                                 issues.push(format!("Unauthorized module: {}", module_name));
                             }
                         }
                     }
                 }
                 cursor = start_idx;
             }
        }

        if !issues.is_empty() {
             let res = serde_json::json!({ "success": false, "error": format!("Blocked by Self-Defense: {}", issues.join(", ")) });
             return Ok(res.to_string());
        }

        let root = openspore_core::path_utils::get_app_root();
        let target_dir = root.join("skills");
        let file_path = target_dir.join(&filename);

        // Security check: ensure path is within skills directory
        if !file_path.starts_with(&target_dir) {
             let res = serde_json::json!({ "success": false, "error": "Error: Skill deployment is restricted to ~/.openspore/skills/" });
             return Ok(res.to_string());
        }

        if !target_dir.exists() {
            if let Err(e) = fs::create_dir_all(&target_dir).await {
                let res = serde_json::json!({ "success": false, "error": format!("Failed to create skills directory: {}", e) });
                return Ok(res.to_string());
            }
        }

        // Unescape code (only if it wasn't valid JSON, as the JSON parser handles escapes)
        let final_code = if crate::utils::try_parse_json(args).is_some() {
            code
        } else {
            crate::utils::unescape(&code)
        };

        // NEW: Sanitize file content (Remove Markdown Blocks)
        let mut content_to_write = final_code.trim();
        // Check for common markdown block patterns at START
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

        match fs::write(&file_path, content_to_write).await {
            Ok(_) => {
                let res = serde_json::json!({
                    "success": true,
                    "filename": filename,
                    "message": format!("Skill {} deployed to {} and validated.", filename, target_dir.display())
                });
                Ok(res.to_string())
            },
            Err(e) => {
                let res = serde_json::json!({ "success": false, "error": format!("Failed to write skill file: {}", e) });
                Ok(res.to_string())
            }
        }
    }
}
