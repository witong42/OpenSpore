pub mod config;
pub mod state;
pub mod path_utils;

use tracing::{info, warn};

pub fn init() {
    info!("🍄 Spore Core Initialized");
    match config::AppConfig::load() {
        Ok(cfg) => info!("✅ Config loaded: Autonomy={}", cfg.autonomy_enabled),
        Err(e) => warn!("⚠️ Config warning: {}", e),
    }
}
