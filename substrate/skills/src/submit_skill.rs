//! Submit Skill (Core)
//! Port of submit_code.js - Deploys modules to autonomy sandbox.

use super::Skill;
use async_trait::async_trait;
use tokio::fs;
use std::path::PathBuf;

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
        "Deploy a self-generated logic module (JS/Shell) to the skills directory for immediate use. Usage: [SUBMIT_SKILL: \"filename.js|||literal_code_content\"]"
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
                return Err("Usage: [SUBMIT_SKILL: {\"filename\": \"...\", \"code\": \"...\"}] or [SUBMIT_SKILL: \"filename|||code\"]".to_string());
            }

            let filename = parts[0].trim().trim_matches('"').trim_matches('\'').trim().to_string();
            let code = parts[1..].join(separator);
            (filename, code)
        };

        // Path-pointing detection (common LLM mistake)
        let trimmed_code = code.trim();
        if (trimmed_code.starts_with('/') || trimmed_code.starts_with('~') || trimmed_code.starts_with("./"))
           && !trimmed_code.contains('\n') && !trimmed_code.contains(' ') && !trimmed_code.contains('{') {
               return Err(format!("Blocked by Self-Defense: SUBMIT_SKILL requires the literal content of the code, not a path to it. You provided: '{}'", trimmed_code));
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
        // Note: Rust regex would be better but keeping it simple as per JS port for now.
        // We look for require("...") or require('...')
        if code.contains("require(") {
             // Basic extraction logic
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
            return Err(format!("Blocked by Self-Defense: {}", issues.join(", ")));
        }

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let target_dir = PathBuf::from(format!("{}/.openspore/skills", home));
        let file_path = target_dir.join(&filename);

        // Security check: ensure path is within skills directory
        if !file_path.starts_with(&target_dir) {
            return Err("Error: Skill deployment is restricted to ~/.openspore/skills/".to_string());
        }

        if !target_dir.exists() {
            fs::create_dir_all(&target_dir).await.map_err(|e| e.to_string())?;
        }

        // Unescape newlines (common LLM behavior to provide \n in one-line tool calls)
        let final_code = code.replace("\\n", "\n");

        fs::write(&file_path, final_code).await.map_err(|e| e.to_string())?;

        Ok(format!("Success: Skill {} deployed to {} and validated.", filename, target_dir.display()))
    }
}
