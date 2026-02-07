//! Exec Skill - Run shell commands (Core)

use super::Skill;
use async_trait::async_trait;
use tokio::process::Command;

pub struct ExecSkill;

#[async_trait]
impl Skill for ExecSkill {
    fn name(&self) -> &'static str { "exec" }

    fn description(&self) -> &'static str {
        "Execute a shell command. Returns JSON with success, exit_code, stdout, and stderr. Usage: [EXEC: \"ls -la\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let project_root = openspore_core::path_utils::get_app_root();

        // Sanitize arguments centrally
        let sanitized_args = crate::utils::sanitize_path(args);
        if sanitized_args.is_empty() {
             return Ok(serde_json::json!({ "success": true, "stdout": "", "stderr": "" }).to_string());
        }

        // Intelligent Binary Discovery
        let engine_bin = project_root.join("crates/target/release");
        let path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", engine_bin.to_string_lossy(), path);

        let mut cmd_parts = sanitized_args.split_whitespace();
        let first_word = cmd_parts.next().unwrap_or("");

        let final_cmd = if first_word == "openspore" {
            let local_bin = engine_bin.join("openspore");
            if local_bin.exists() {
                sanitized_args.replacen("openspore", &local_bin.to_string_lossy(), 1)
            } else {
                sanitized_args.to_string()
            }
        } else {
            sanitized_args.to_string()
        };

        // SAFE MODE CHECK (EXEC)
        if crate::utils::is_safe_mode_active() {
            let dangerous_keywords = ["rm ", "mv ", "sed ", "cargo build", "git checkout", "git reset", "git clean", "chmod ", "chown "];
            let lower_cmd = final_cmd.to_lowercase();

            if dangerous_keywords.iter().any(|kw| lower_cmd.contains(kw)) {
                // Determine if any part of the command targets the protected engine paths
                let protected_targets = ["crates/", "/crates", ".env", "cargo.toml", "cargo.lock", "install.sh", "readme.md"];
                let targets_protected = protected_targets.iter().any(|t| lower_cmd.contains(t.to_lowercase().as_str()));

                if targets_protected {
                     let res = serde_json::json!({
                        "success": false,
                        "error": "SAFE_MODE_ENABLED: This command is blocked because it targets protected engine crates or configuration."
                    });
                    return Ok(serde_json::to_string_pretty(&res).unwrap_or_default());
                }
            }
        }

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(&final_cmd)
            .envs(std::env::vars())
            .env("PATH", new_path)
            .current_dir(&project_root);

        let output = cmd.output()
            .await
            .map_err(|e| format!("System Error: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        // Structured Response for AI Agents
        let result = serde_json::json!({
            "success": output.status.success(),
            "exit_code": output.status.code().unwrap_or(-1),
            "stdout": stdout,
            "stderr": stderr
        });

        // Always return Ok for a completed command.
        // The LLM will use the "success" boolean and exit_code to judge the result.
        Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()))
    }
}
