pub mod config;
pub mod state;
pub mod path_utils;

use tracing::{info, warn};

pub fn init() {
    info!("🍄 Spore Core Initialized");
}
