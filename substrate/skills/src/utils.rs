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
