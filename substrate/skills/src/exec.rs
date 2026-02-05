//! Exec Skill - Run shell commands (Core)

use super::Skill;
use async_trait::async_trait;
use tokio::process::Command;

pub struct ExecSkill;

#[async_trait]
impl Skill for ExecSkill {
    fn name(&self) -> &'static str { "exec" }

    fn description(&self) -> &'static str {
        "Execute a shell command. Usage: [EXEC: \"ls -la\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(args)
            .output()
            .await
            .map_err(|e| e.to_string())?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        let mut combined = stdout;
        if !stderr.is_empty() {
            if !combined.is_empty() {
                combined.push_str("\n");
            }
            combined.push_str(&format!("[STDERR]\n{}", stderr));
        }

        if output.status.success() {
            if combined.is_empty() {
                Ok("✅ Command executed successfully (no output).".to_string())
            } else {
                Ok(combined)
            }
        } else {
            Err(format!("Command failed (exit code {}):\n{}",
                output.status.code().unwrap_or(-1),
                combined))
        }
    }
}
