pub mod config;
pub mod state;
pub mod path_utils;

use tracing::{info};

pub fn init() {
    info!("🍄 Spore Core Initialized");
}
