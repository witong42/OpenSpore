//! Delegate Skill (Core) - Spawn sub-spores

use super::Skill;
use async_trait::async_trait;
use tracing::info;

pub struct DelegateSkill;

#[async_trait]
impl Skill for DelegateSkill {
    fn name(&self) -> &'static str { "delegate" }

    fn description(&self) -> &'static str {
        "Spawn a specialized sub-spore for parallel task execution. Usage: [DELEGATE: \"task description\" --role=\"ExpertRole\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let parts: Vec<&str> = args.splitn(2, "--role=").collect();
        let task = parts[0].trim().trim_matches('"').trim_matches('\'').trim();
        let role = parts.get(1).map(|r| r.trim().trim_matches('"').trim_matches('\'')).unwrap_or("GeneralExpert");

        use tokio::process::Command;

        // Find the openspore binary
        let binary = if std::path::Path::new("/usr/local/bin/openspore").exists() {
            "/usr/local/bin/openspore".to_string()
        } else if let Ok(home) = std::env::var("HOME") {
            let p = format!("{}/.local/bin/openspore", home);
            if std::path::Path::new(&p).exists() {
                p
            } else {
                "openspore".to_string()
            }
        } else {
            "openspore".to_string()
        };

        info!("🧵 Swarm: Delegating task with role {}: {}", role, task);

        let output = Command::new(binary)
            .arg("think")
            .arg(task)
            .arg("--role")
            .arg(role)
            .output()
            .await
            .map_err(|e| format!("Delegation process failed: {}", e))?;

        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if output.status.success() {
            Ok(format!(
                "\n=== Result from Sub-Agent [{}] ===\n{}\n====================================\n",
                role, result
            ))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Delegation Failed ({}):\n{}\n{}", output.status, result, stderr))
        }
    }
}
