use async_trait::async_trait;
use thiserror::Error;

pub mod shell;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[derive(Error, Debug)]
pub enum IoError {
    #[error("Command failed: {0}")]
    CommandError(String),
    #[error("Platform not supported")]
    UnsupportedPlatform,
    #[error("IO Error: {0}")]
    StdIo(#[from] std::io::Error),
}

#[async_trait]
pub trait NativeBridge {
    /// Speak text using native TTS
    async fn say(&self, text: &str) -> Result<(), IoError>;

    /// Send a notification
    async fn notify(&self, title: &str, message: &str) -> Result<(), IoError>;

    /// Control an application (generic "tell")
    async fn tell(&self, app: &str, command: &str) -> Result<String, IoError>;

    /// Get active application name
    async fn get_active_app(&self) -> Result<String, IoError>;

    /// Get spotify current track info
    async fn get_spotify_status(&self) -> Result<String, IoError>;
}

// Factory function to get the platform-specific bridge
pub fn get_bridge() -> Box<dyn NativeBridge + Send + Sync> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacBridge::new())
    }

    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxBridge::new())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        panic!("Unsupported OS")
    }
}

pub fn init() {
    println!("🔌 Spore IO Initialized");
}
