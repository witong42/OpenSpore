use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct WatchEvent {
    pub event_type: String,
    pub file_path: PathBuf,
}
