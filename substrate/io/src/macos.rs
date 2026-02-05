use super::{NativeBridge, IoError};
use async_trait::async_trait;
use tokio::process::Command;

pub struct MacBridge;

impl MacBridge {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl NativeBridge for MacBridge {
    async fn say(&self, text: &str) -> Result<(), IoError> {
        Command::new("say")
            .arg(text)
            .output()
            .await?;
        Ok(())
    }

    async fn notify(&self, title: &str, message: &str) -> Result<(), IoError> {
        let script = format!("display notification \"{}\" with title \"{}\"", message, title);
        self.tell("System Events", &script).await.map(|_| ())
    }

    async fn tell(&self, app: &str, command: &str) -> Result<String, IoError> {
        // Safe AppleScript execution
        // tell application "App" to ...
        let script = if app == "System Events" {
            command.to_string()
        } else {
             format!("tell application \"{}\" to {}", app, command)
        };

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(IoError::CommandError(String::from_utf8_lossy(&output.stderr).to_string()))
        }
    }

    async fn get_active_app(&self) -> Result<String, IoError> {
        let script = "tell application \"System Events\" to get name of first application process whose frontmost is true";
        self.tell("System Events", script).await
    }

    async fn get_spotify_status(&self) -> Result<String, IoError> {
        let script = r#"
            if application "Spotify" is running then
                tell application "Spotify"
                    return (get artist of current track) & " - " & (get name of current track)
                end tell
            else
                return ""
            end if
        "#;
        self.tell("Spotify", script).await.or(Ok("".to_string()))
    }
}
