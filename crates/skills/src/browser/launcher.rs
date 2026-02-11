use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use std::str::FromStr;
use serde_json::Value;
use tokio::time::sleep;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserType {
    Chrome,
    Brave,
    Edge,
    Arc,
    Chromium,
}

impl BrowserType {
    pub fn name(&self) -> &'static str {
        match self {
            BrowserType::Chrome => "Google Chrome",
            BrowserType::Brave => "Brave",
            BrowserType::Edge => "Microsoft Edge",
            BrowserType::Arc => "Arc",
            BrowserType::Chromium => "Chromium",
        }
    }
}

impl FromStr for BrowserType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "chrome" | "google-chrome" => Ok(BrowserType::Chrome),
            "brave" | "brave-browser" => Ok(BrowserType::Brave),
            "edge" | "microsoft-edge" => Ok(BrowserType::Edge),
            "arc" => Ok(BrowserType::Arc),
            "chromium" => Ok(BrowserType::Chromium),
            _ => Err(anyhow!("Unknown browser type: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BrowserInfo {
    pub browser_type: BrowserType,
    pub path: PathBuf,
    pub version: Option<String>,
}

impl BrowserInfo {
    pub fn new(browser_type: BrowserType, path: PathBuf) -> Self {
        Self {
            browser_type,
            path,
            version: None,
        }
    }

    pub fn with_version(mut self) -> Self {
        self.version = self.detect_version();
        self
    }

    fn detect_version(&self) -> Option<String> {
        let output = Command::new(&self.path).arg("--version").output().ok()?;
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.trim();
            if let Some(idx) = version.rfind(' ') {
                return Some(version[idx + 1..].to_string());
            }
            Some(version.to_string())
        } else {
            None
        }
    }
}

pub fn discover_browser(preferred: Option<BrowserType>) -> Result<BrowserInfo> {
    tracing::info!("Discovering browser. Preference: {:?}", preferred);
    let browsers = discover_all_browsers();
    tracing::info!("Found installed browsers: {:?}", browsers.iter().map(|b| b.browser_type).collect::<Vec<_>>());

    if let Some(pref) = preferred {
        if let Some(info) = browsers.iter().find(|b| b.browser_type == pref) {
            tracing::info!("Selected preferred browser: {:?}", info.browser_type);
            return Ok(info.clone());
        } else {
            tracing::warn!("Preferred browser {:?} not found in installed list.", pref);
        }
    }

    let selected = browsers.into_iter().next().ok_or_else(|| anyhow!("No supported browser found"))?;
    tracing::info!("Falling back to/Selecting default browser: {:?}", selected.browser_type);
    Ok(selected)
}

pub fn discover_all_browsers() -> Vec<BrowserInfo> {
    let candidates = get_browser_candidates();
    let mut found = Vec::new();

    for (browser_type, paths) in candidates {
        for path in paths {
            let p = PathBuf::from(shellexpand::tilde(&path).to_string());
            if p.exists() {
                found.push(BrowserInfo::new(browser_type, p).with_version());
                break;
            }
        }
    }
    found
}

fn get_browser_candidates() -> Vec<(BrowserType, Vec<String>)> {
    #[cfg(target_os = "macos")]
    {
        let candidates = vec![
            (BrowserType::Chrome, vec!["com.google.Chrome", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "~/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"]),
            (BrowserType::Brave, vec!["com.brave.Browser", "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser", "~/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"]),
            (BrowserType::Edge, vec!["com.microsoft.edgemac", "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"]),
            (BrowserType::Arc, vec!["company.thebrowser.Browser", "/Applications/Arc.app/Contents/MacOS/Arc"]),
            (BrowserType::Chromium, vec!["org.chromium.Chromium", "/Applications/Chromium.app/Contents/MacOS/Chromium"]),
        ];

        let mut final_candidates = Vec::new();

        for (b_type, paths) in candidates {
            let mut resolved_paths = Vec::new();

            // 1. Try mdfind first (bundle ID is the first element)
            if let Some(bundle_id) = paths.first() {
                if !bundle_id.starts_with('/') && !bundle_id.starts_with('~') {
                   if let Some(p) = find_mdfind_browser(bundle_id) {
                       resolved_paths.push(p.to_string_lossy().to_string());
                   }
                }
            }

            // 2. Add hardcoded paths
            for p in paths {
                if p.starts_with('/') || p.starts_with('~') {
                    resolved_paths.push(p.to_string());
                }
            }

            final_candidates.push((b_type, resolved_paths));
        }

        // Convert inner Vec<String> to Vec<&'static str> is hard because of ownership.
        // We need to change the return type signature of get_browser_candidates to Vec<(BrowserType, Vec<String>)>
        final_candidates
    }
    #[cfg(target_os = "linux")]
    {
        let candidates = vec![
            (BrowserType::Chrome, vec!["/usr/bin/google-chrome", "/usr/bin/google-chrome-stable", "/opt/google/chrome/google-chrome"]),
            (BrowserType::Brave, vec!["/usr/bin/brave-browser", "/usr/bin/brave"]),
            (BrowserType::Edge, vec!["/usr/bin/microsoft-edge", "/usr/bin/microsoft-edge-stable"]),
            (BrowserType::Chromium, vec!["/usr/bin/chromium", "/usr/bin/chromium-browser", "/snap/bin/chromium"]),
        ];

        let mut final_candidates = Vec::new();
        for (b_type, paths) in candidates {
            let mut resolved_paths = Vec::new();
            for p in paths {
                resolved_paths.push(p.to_string());
            }
            final_candidates.push((b_type, resolved_paths));
        }
        final_candidates
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        vec![]
    }
}

/// Use macOS Spotlight (mdfind) to locate a browser by Bundle ID
#[cfg(target_os = "macos")]
fn find_mdfind_browser(bundle_id: &str) -> Option<PathBuf> {
    let output = Command::new("mdfind")
        .arg(format!("kMDItemCFBundleIdentifier == '{}'", bundle_id))
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = stdout.lines().next() {
            let app_path = PathBuf::from(line);
            // Append binary path inside .app
            let binary_name = match bundle_id {
                "com.google.Chrome" => "Google Chrome",
                "com.brave.Browser" => "Brave Browser",
                "com.microsoft.edgemac" => "Microsoft Edge",
                "company.thebrowser.Browser" => "Arc",
                "org.chromium.Chromium" => "Chromium",
                _ => return None,
            };
            let full_path = app_path.join("Contents/MacOS").join(binary_name);
            if full_path.exists() {
                tracing::info!("mdfind resolved {} to {:?}", bundle_id, full_path);
                return Some(full_path);
            }
        }
    }
    None
}

pub struct BrowserLauncher {
    pub browser_info: BrowserInfo,
    pub user_data_dir: PathBuf,
    pub cdp_port: u16,
    pub headless: bool,
}

impl BrowserLauncher {
    pub fn new(preferred: Option<BrowserType>) -> Result<Self> {
        let browser_info = discover_browser(preferred)?;
        let root = openspore_core::path_utils::get_app_root();

        // --- Legacy Cleanup ---
        let legacy_profile = root.join("browser_profile");
        if legacy_profile.exists() && legacy_profile.is_dir() {
            tracing::warn!("Cleaning up legacy browser_profile at {:?}", legacy_profile);
            let _ = std::fs::remove_dir_all(&legacy_profile);
        }
        // ----------------------

        tracing::info!("Resolved App Root: {:?}", root);
        let user_data_dir = root
            .join("workspace")
            .join("browser_profile");

        Ok(Self {
            browser_info,
            user_data_dir,
            cdp_port: 9222,
            headless: false,
        })
    }

    pub fn launch(&self) -> Result<Child> {

        tracing::info!("Launching browser with user data dir: {:?}", self.user_data_dir);
        std::fs::create_dir_all(&self.user_data_dir)?;

        let mut args = vec![
            format!("--remote-debugging-port={}", self.cdp_port),
            format!("--user-data-dir={}", self.user_data_dir.display()),
            "--no-first-run".to_string(),
            "--no-default-browser-check".to_string(),
            "--disable-blink-features=AutomationControlled".to_string(),
            "--disable-infobars".to_string(),
            "--window-size=1920,1080".to_string(),
        ];

        if self.headless {
            args.push("--headless=new".to_string());
        }

        Command::new(&self.browser_info.path)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow!("Failed to launch browser: {}", e))
    }

    pub async fn wait_for_cdp(&self) -> Result<String> {
        let url = format!("http://127.0.0.1:{}/json/version", self.cdp_port);
        let client = reqwest::Client::builder().no_proxy().build()?;

        for _ in 0..20 {
            sleep(Duration::from_millis(500)).await;
            if let Ok(resp) = client.get(&url).send().await {
                if resp.status().is_success() {
                    let json: Value = resp.json().await?;
                    if let Some(ws_url) = json.get("webSocketDebuggerUrl").and_then(|v| v.as_str()) {
                        return Ok(ws_url.to_string());
                    }
                }
            }
        }
        Err(anyhow!("Timeout waiting for CDP"))
    }

    pub async fn launch_and_wait(&self) -> Result<(Child, String)> {
        let child = self.launch()?;
        let ws_url = self.wait_for_cdp().await?;
        Ok((child, ws_url))
    }
}
