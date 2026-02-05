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

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(stdout.to_string())
        } else {
            Err(format!("Command failed:\n{}\n{}", stdout, stderr))
        }
    }
}
