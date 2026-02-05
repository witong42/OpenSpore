use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    // Placeholder for future memory and autonomy state
    pub memory_path: String,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        // Determine memory path (default to ~/.openspore/workspace)
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let root_name = std::env::var("OPENSPORE_ROOT").unwrap_or_else(|_| ".openspore".to_string());

        let memory_path = format!("{}/{}/workspace", home, root_name);

        Self {
            config,
            memory_path,
        }
    }
}
