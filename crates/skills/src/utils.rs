//! Shared utilities for skills

use serde_json::Value;

/// Parses arguments that can be either a JSON string or a raw string with a delimiter.
///
/// If `args` is valid JSON, it returns the parsed Value.
/// If not, strict JSON parsing fails, and it returns None.
pub fn try_parse_json(args: &str) -> Option<Value> {
    serde_json::from_str(args).ok()
}

/// Helper to extract string field from JSON value
pub fn get_str_field(val: &Value, field: &str) -> Option<String> {
    val.get(field).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Robustly unescape a string coming from an LLM tool call.
/// Handles \n, \r, \t, \\, \", and \'.
pub fn unescape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                match next {
                    'n' => { result.push('\n'); chars.next(); }
                    'r' => { result.push('\r'); chars.next(); }
                    't' => { result.push('\t'); chars.next(); }
                    '\\' => { result.push('\\'); chars.next(); }
                    '"' => { result.push('"'); chars.next(); }
                    '\'' => { result.push('\''); chars.next(); }
                    _ => { result.push('\\'); }
                }
            } else {
                result.push('\\');
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Centralized path/string sanitization for all skills.
/// Trims quotes, expands tildes, and removes unneccessary whitespace.
pub fn sanitize_path(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'').trim();
    openspore_core::path_utils::expand_tilde(trimmed)
}

/// Check if global safe mode is enabled via environment variable
pub fn is_safe_mode_active() -> bool {
    std::env::var("SAFE_MODE_ENABLED")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// Check if a path is considered part of the "protected engine" (core logic/config)
pub fn is_path_protected(path_str: &str) -> bool {
    let root = openspore_core::path_utils::get_app_root();
    let root_str = root.to_string_lossy();

    // Sanitize and ensure absolute
    let path = std::path::Path::new(path_str);
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let abs_str = abs_path.to_string_lossy();

    // Protection Rules:
    // 1. Block anything inside the 'crates' directory (engine logic)
    if abs_str.contains(&format!("{}/crates", root_str)) {
        return true;
    }

    // 2. Block top-level configuration and installer files
    let protected_files = [".env", "Cargo.toml", "Cargo.lock", "install.sh", "README.md"];
    for file in protected_files {
        if abs_str == format!("{}/{}", root_str, file) {
            return true;
        }
    }

    false
}

/// A simple shell-word splitter that respects quotes and escapes.
pub fn split_arguments(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_quote = false;
    let mut quote_char = '\0';
    let mut escaped = false;

    for c in s.chars() {
        if escaped {
            word.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if in_quote {
            if c == quote_char {
                in_quote = false;
            } else {
                word.push(c);
            }
        } else if c == '"' || c == '\'' {
            in_quote = true;
            quote_char = c;
        } else if c.is_whitespace() {
            if !word.is_empty() {
                words.push(word.clone());
                word.clear();
            }
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}
