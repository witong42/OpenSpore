use super::IoError;
use tokio::process::Command;

pub async fn exec(command: &str, args: &[&str]) -> Result<String, IoError> {
    let output = Command::new(command)
        .args(args)
        .output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(IoError::CommandError(String::from_utf8_lossy(&output.stderr).to_string()))
    }
}
