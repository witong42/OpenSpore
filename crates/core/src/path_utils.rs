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
    if !path.contains('~') {
        return path.to_string();
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());

    // Replace ~/ at start or after a space
    let mut result = path.to_string();
    if result.starts_with("~/") {
        result = result.replacen("~/", &format!("{}/", home), 1);
    }
    result.replace(" ~/", &format!(" {}/", home))
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

/// Ensures a path is absolute, resolving tilde and relative to app root.
pub fn ensure_absolute(path: &str) -> PathBuf {
    let p = get_path(path);
    if p.is_absolute() {
        p
    } else {
        get_app_root().join(p)
    }
}

/// Generates a lightweight, depth-limited string representation of the directory tree.
/// Skips hidden files, target/, and node_modules/ to keep the context slim.
pub fn get_directory_tree(path: &std::path::Path, max_depth: usize) -> String {
    let mut tree = String::new();
    let _ = build_tree_string(path, 0, max_depth, &mut tree);
    tree
}

fn build_tree_string(path: &std::path::Path, depth: usize, max_depth: usize, out: &mut String) -> std::io::Result<()> {
    if depth > max_depth {
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(path)?
        .flatten()
        .collect();

    // Sort for stable results
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip junk
        if name.starts_with('.') || name == "target" || name == "node_modules" || name == "dist" || name == "out" {
            continue;
        }

        for _ in 0..depth {
            out.push_str("  ");
        }

        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            out.push_str("📁 ");
            out.push_str(&name);
            out.push_str("/\n");
            let _ = build_tree_string(&entry.path(), depth + 1, max_depth, out);
        } else {
            out.push_str("📄 ");
            out.push_str(&name);
            out.push_str("\n");
        }
    }
    Ok(())
}
