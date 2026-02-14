//! Write File Skill (Core)
//! Supports both --content= syntax and heredoc <<<EOF blocks for multi-line content.

use super::Skill;
use async_trait::async_trait;
use tokio::fs;
use std::path::Path;

pub struct WriteFileSkill;

#[async_trait]
impl Skill for WriteFileSkill {
    fn name(&self) -> &'static str { "write_file" }

    fn description(&self) -> &'static str {
        "Write content to a file. Supports two modes:\n\
         1. Inline: [WRITE_FILE: \"/path\" --content=\"content\"]\n\
         2. Heredoc (recommended for code): [WRITE_FILE: \"/path\" <<<EOF\ncontent here\nEOF]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        // Try heredoc syntax first: "/path" <<<EOF\ncontent\nEOF
        if let Some((path_raw, content)) = try_parse_heredoc(args) {
            return write_content(&path_raw, &content).await;
        }

        // Fallback: --content= syntax
        let content_marker = "--content=";
        let (path_part, content_part) = if let Some(idx) = args.find(content_marker) {
            (&args[..idx], &args[idx + content_marker.len()..])
        } else {
            // Last resort: try JSON
            if let Some(json_args) = crate::utils::try_parse_json(args) {
                let p = crate::utils::get_str_field(&json_args, "path")
                    .or_else(|| crate::utils::get_str_field(&json_args, "file"))
                    .unwrap_or_default();
                let c = crate::utils::get_str_field(&json_args, "content").unwrap_or_default();
                return write_content(&p, &c).await;
            }

            let res = serde_json::json!({
                "success": false,
                "error": "Usage: [WRITE_FILE: \"/path\" <<<EOF\ncontent\nEOF] or [WRITE_FILE: \"/path\" --content=\"content\"]"
            });
            return Ok(res.to_string());
        };

        let path = path_part.trim().trim_matches('"').trim_matches('\'').trim();

        let mut content = content_part.trim();
        if (content.starts_with('"') && content.ends_with('"')) ||
           (content.starts_with('\'') && content.ends_with('\'')) {
            if content.len() >= 2 {
                content = &content[1..content.len()-1];
            }
        }

        let final_content = crate::utils::unescape(content);
        write_content(path, &final_content).await
    }
}

/// Parse heredoc syntax: "/path" <<<MARKER\ncontent\nMARKER
fn try_parse_heredoc(args: &str) -> Option<(String, String)> {
    let heredoc_marker = "<<<";
    let idx = args.find(heredoc_marker)?;

    let path_part = args[..idx].trim().trim_matches('"').trim_matches('\'').trim();
    if path_part.is_empty() {
        return None;
    }

    let after_marker = &args[idx + heredoc_marker.len()..];

    // Find the delimiter word (e.g., EOF, END, CONTENT)
    let first_newline = after_marker.find('\n')?;
    let delimiter = after_marker[..first_newline].trim();
    if delimiter.is_empty() {
        return None;
    }

    let content_start = first_newline + 1;
    let remaining = &after_marker[content_start..];

    // Find the closing delimiter (must be on its own line)
    let end_pattern = format!("\n{}", delimiter);
    let content = if let Some(end_idx) = remaining.find(&end_pattern) {
        &remaining[..end_idx]
    } else if remaining.ends_with(delimiter) && remaining[..remaining.len()-delimiter.len()].ends_with('\n') {
        &remaining[..remaining.len() - delimiter.len() - 1]
    } else if remaining.trim_end().ends_with(delimiter) {
        let trimmed = remaining.trim_end();
        &trimmed[..trimmed.len() - delimiter.len()]
    } else {
        // No closing delimiter found, treat everything as content
        remaining.trim_end()
    };

    Some((path_part.to_string(), content.to_string()))
}

async fn write_content(path_raw: &str, content: &str) -> Result<String, String> {
    let path = crate::utils::sanitize_path(path_raw);

    if path.is_empty() {
        return Ok(serde_json::json!({ "success": false, "error": "Empty file path." }).to_string());
    }

    // SAFE MODE CHECK
    if crate::utils::is_safe_mode_active() && crate::utils::is_path_protected(&path) {
        return Ok(serde_json::json!({
            "success": false,
            "error": "SAFE_MODE_ENABLED: Modifying engine core is forbidden."
        }).to_string());
    }

    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent).await.ok();
    }

    let bytes_written = content.len();

    match fs::write(&path, content).await {
        Ok(_) => {
            Ok(serde_json::json!({
                "success": true,
                "bytes_written": bytes_written,
                "path": path
            }).to_string())
        },
        Err(e) => {
            Ok(serde_json::json!({
                "success": false,
                "error": format!("Failed to write {}: {}", path, e),
                "path": path
            }).to_string())
        }
    }
}
