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
