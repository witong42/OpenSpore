//! Exec Skill - Run shell commands (Core)

use super::Skill;
use async_trait::async_trait;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::process::Stdio;
use tokio::time::Duration;

pub struct ExecSkill;

#[async_trait]
impl Skill for ExecSkill {
    fn name(&self) -> &'static str { "exec" }

    fn description(&self) -> &'static str {
        "Execute a shell command with virtual statefulness and optional pattern waiting. Usage: [EXEC: \"command\", \"optional_pattern\", \"optional_timeout_sec\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let project_root = openspore_core::path_utils::get_app_root();

        // 1. Virtual Statefulness: Get current CWD
        let current_cwd = crate::utils::get_virtual_cwd();

        // 2. Argument Parsing
        let args_list = crate::utils::split_arguments(args);
        let raw_cmd = args_list.get(0).cloned().unwrap_or_default();
        let wait_for = args_list.get(1).cloned();
        let timeout_secs = args_list.get(2).and_then(|s| s.parse::<u64>().ok()).unwrap_or(30);

        if raw_cmd.is_empty() {
             return Ok(serde_json::json!({ "success": true, "stdout": "", "stderr": "" }).to_string());
        }

        // 3. Handle 'cd' (Virtual Statefulness update)
        let trimmed_cmd = raw_cmd.trim();
        if trimmed_cmd.starts_with("cd ") {
            let target_path_str = trimmed_cmd[3..].trim().trim_matches('"').trim_matches('\'');
            let target_path = openspore_core::path_utils::ensure_absolute(target_path_str);

            if target_path.exists() && target_path.is_dir() {
                let _ = crate::utils::set_virtual_cwd(&target_path);
                // If it's JUST a cd, return success immediately
                if (!trimmed_cmd.contains("&&") && !trimmed_cmd.contains(';')) || trimmed_cmd.len() == target_path_str.len() + 3 {
                    return Ok(serde_json::json!({
                        "success": true,
                        "cwd": target_path.to_string_lossy(),
                        "message": format!("CWD updated to {}", target_path.display())
                    }).to_string());
                }
            }
        }

        // 4. Intelligent Binary & PATH setup
        let engine_bin = project_root.join("crates/target/release");
        let path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", engine_bin.to_string_lossy(), path);

        // 5. SAFE MODE CHECK
        if crate::utils::is_safe_mode_active() {
            let dangerous_keywords = ["rm ", "mv ", "sed ", "cargo build", "git checkout", "git reset", "git clean", "chmod ", "chown "];
            let lower_cmd = raw_cmd.to_lowercase();

            if dangerous_keywords.iter().any(|kw| lower_cmd.contains(kw)) {
                let protected_targets = ["crates/", "/crates", ".env", "cargo.toml", "cargo.lock", "install.sh", "readme.md"];
                if protected_targets.iter().any(|t| lower_cmd.contains(t.to_lowercase().as_str())) {
                    return Ok(serde_json::json!({
                        "success": false,
                        "error": "SAFE_MODE_ENABLED: Command blocked targeting engine core."
                    }).to_string());
                }
            }
        }

        // 6. Execution Path
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&raw_cmd)
           .envs(std::env::vars())
           .env("PATH", new_path)
           .current_dir(&current_cwd)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("Command Spawn Error: {}", e))?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut captured_stdout = Vec::new();
        let mut captured_stderr = Vec::new();
        let mut pattern_found = false;

        let start = tokio::time::Instant::now();
        let duration = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() >= duration { break; }

            tokio::select! {
                line = stdout_reader.next_line() => {
                    if let Ok(Some(l)) = line {
                        if let Some(ref p) = wait_for {
                            if l.contains(p) { pattern_found = true; }
                        }
                        captured_stdout.push(l);
                        if pattern_found { break; }
                    } else {
                        break;
                    }
                }
                line = stderr_reader.next_line() => {
                    if let Ok(Some(l)) = line {
                         if let Some(ref p) = wait_for {
                            if l.contains(p) { pattern_found = true; }
                        }
                        captured_stderr.push(l);
                        if pattern_found { break; }
                    }
                }
                status = child.wait() => {
                    let _ = status;
                    break;
                }
            }
        }

        let full_stdout = captured_stdout.join("\n");
        let full_stderr = captured_stderr.join("\n");

        Ok(serde_json::json!({
            "success": true,
            "wait_condition_met": pattern_found,
            "stdout": full_stdout,
            "stderr": full_stderr,
            "cwd": current_cwd.to_string_lossy()
        }).to_string())
    }
}
