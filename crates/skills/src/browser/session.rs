use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};
use chromiumoxide::browser::Browser;
use chromiumoxide::handler::Handler;
use anyhow::{anyhow, Result};
use super::launcher::{BrowserLauncher, BrowserType};

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionState {
    pub cdp_port: u16,
    pub cdp_url: String,
}

pub struct SessionManager {
    sessions_dir: PathBuf,
    preferred_browser: Option<BrowserType>,
}

impl SessionManager {
    pub fn new(preferred_browser: Option<BrowserType>) -> Self {
        let sessions_dir = openspore_core::path_utils::get_app_root()
            .join("workspace")
            .join("browser_sessions");
        Self { sessions_dir, preferred_browser }
    }

    fn session_file(&self) -> PathBuf {
        self.sessions_dir.join("active_session.json")
    }

    fn load_session_state(&self) -> Option<SessionState> {
        let content = fs::read_to_string(self.session_file()).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save_session_state(&self, state: &SessionState) -> Result<()> {
        fs::create_dir_all(&self.sessions_dir)?;
        let content = serde_json::to_string_pretty(state)?;
        fs::write(self.session_file(), content)?;
        Ok(())
    }

    pub fn remove_session_state(&self) {
        let _ = fs::remove_file(self.session_file());
    }

    pub async fn is_alive(&self, state: &SessionState) -> bool {
        let url = format!("http://127.0.0.1:{}/json/version", state.cdp_port);
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        client.get(&url).send().await.is_ok()
    }

    pub async fn get_or_create_session(&self) -> Result<(Browser, Handler)> {
        if let Some(state) = self.load_session_state() {
            if self.is_alive(&state).await {
                if let Ok((browser, handler)) = Browser::connect(&state.cdp_url).await {
                    return Ok((browser, handler));
                }
            }
            self.remove_session_state();
        }

        let launcher = BrowserLauncher::new(self.preferred_browser)?;
        let (_child, cdp_url) = launcher.launch_and_wait().await?;

        let state = SessionState {
            cdp_port: launcher.cdp_port,
            cdp_url: cdp_url.clone(),
        };
        self.save_session_state(&state)?;

        Browser::connect(&cdp_url).await.map_err(|e| anyhow!("Failed to connect: {}", e))
    }
}
