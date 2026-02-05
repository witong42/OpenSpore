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
