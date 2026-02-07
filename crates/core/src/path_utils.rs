//! Path utilities for OpenSpore
//!
//! Handles tilde expansion and path normalization.

use std::path::PathBuf;

/// Expands tilde (~) in paths to the user's home directory.
/// Examples:
/// "~/.openspore" -> "/Users/william/.openspore"
/// "~/archive" -> "/Users/william/archive"
/// "/tmp/foo" -> "/tmp/foo" (no change)
pub fn expand_tilde(path: &str) -> String {
    if path == "~" {
        return std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    }

    if path.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        return path.replacen('~', &home, 1);
    }

    path.to_string()
}

/// Helper to convert a potentially tilde-containing string into a PathBuf.
pub fn get_path(path: &str) -> PathBuf {
    PathBuf::from(expand_tilde(path))
}

/// Robustly resolves the OpenSpore project root using OPENSPORE_ROOT env var.
/// Handles absolute paths, tilde expansion, and relative names.
pub fn get_app_root() -> PathBuf {
    let root_name = std::env::var("OPENSPORE_ROOT").unwrap_or_else(|_| ".openspore".to_string());

    if root_name.starts_with('/') {
        PathBuf::from(root_name)
    } else if root_name.starts_with('~') {
        get_path(&root_name)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(root_name)
    }
}
