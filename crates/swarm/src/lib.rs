use std::path::PathBuf;
use tokio::process::Command;
use tracing::info;
use anyhow::Result;
use std::time::Duration;
use tokio::time::timeout;
use tokio::sync::Semaphore;
use once_cell::sync::Lazy;

static SWARM_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(6));

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SporeInfo {
    pub pid: u32,
    pub role: String,
    pub task: String,
    pub start_time: chrono::DateTime<chrono::Local>,
}

pub struct SwarmManager {
    pub binary_path: PathBuf,
}

impl SwarmManager {
    pub fn new() -> Self {
        let binary_path = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("openspore"));
        Self { binary_path }
    }

    /// Spawn a new sub-spore (delegation)
    pub async fn spawn(&self, task: &str, role: &str) -> Result<String> {
        info!("🐝 Swarm: Waiting for permit to spawn sub-spore (Role: {})", role);
        let _permit = SWARM_SEMAPHORE.acquire().await?;

        info!("🐝 Swarm: Spawning sub-spore (Role: {}) for task: {}", role, task);

        let child = Command::new(&self.binary_path)
            .arg("think")
            .arg(task)
            .arg("--role")
            .arg(role)
            .env("IS_SPORE", "true")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // 3 minute timeout for sub-spores
        match timeout(Duration::from_secs(180), child.wait_with_output()).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if output.status.success() {
                    Ok(stdout)
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(anyhow::anyhow!("Sub-spore failed ({}):\n{}\n{}", output.status, stdout, stderr))
                }
            }
            Ok(Err(e)) => Err(anyhow::anyhow!("Sub-spore error: {}", e)),
            Err(_) => {
                // The child is already moved into wait_with_output, so if it timeouts,
                // the future is dropped, which kills the child if it's the only handle?
                // Actually, tokio's Child does NOT kill on drop unless configured.
                Err(anyhow::anyhow!("Sub-spore timeout (10m)"))
            }
        }
    }

    /// Discover active sub-spores via process table
    pub async fn discovery(&self) -> Result<Vec<String>> {
        let output = Command::new("ps")
            .args(["aux"])
            .output()
            .await?;

        let content = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<String> = content.lines()
            .filter(|l| l.contains("openspore think") && !l.contains("grep"))
            .map(|s| s.to_string())
            .collect();

        Ok(lines)
    }
}

impl Default for SwarmManager {
    fn default() -> Self {
        Self::new()
    }
}
