use super::{NativeBridge, IoError};
use async_trait::async_trait;
use tokio::process::Command;

pub struct LinuxBridge;

impl LinuxBridge {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl NativeBridge for LinuxBridge {
    async fn say(&self, text: &str) -> Result<(), IoError> {
        // Try spd-say or espeak
        Command::new("spd-say")
            .arg(text)
            .output()
            .await?;
        Ok(())
    }

    async fn notify(&self, title: &str, message: &str) -> Result<(), IoError> {
        Command::new("notify-send")
            .arg(title)
            .arg(message)
            .output()
            .await?;
        Ok(())
    }

    async fn tell(&self, app: &str, command: &str) -> Result<String, IoError> {
        // Linux doesn't have a universal "tell".
        // We'll treat "app" as the binary and "command" as the args for now (shell exec style)
        // OR we can implement DBus logic here for specific apps like Spotify

        // Placeholder simple execution:
        let output = Command::new(app)
            .args(command.split_whitespace())
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
             Err(IoError::CommandError(String::from_utf8_lossy(&output.stderr).to_string()))
        }
    }

    async fn get_active_app(&self) -> Result<String, IoError> {
        Ok("Linux Desktop".to_string())
    }

    async fn get_spotify_status(&self) -> Result<String, IoError> {
        Ok("".to_string())
    }
}
