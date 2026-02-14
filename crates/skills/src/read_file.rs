//! Read File Skill (Core)
//! Supports optional line-range reading to save context tokens.

use super::Skill;
use async_trait::async_trait;
use tokio::fs;

pub struct ReadFileSkill;

#[async_trait]
impl Skill for ReadFileSkill {
    fn name(&self) -> &'static str { "read_file" }

    fn description(&self) -> &'static str {
        "Read contents of a file. Supports optional line range to save context. Usage:\n\
         - Full: [READ_FILE: \"/path/to/file\"]\n\
         - Range: [READ_FILE: \"/path/to/file\" --lines=50-80]\n\
         Returns JSON with success, content, path, total_lines, and shown range."
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let args_str = args.trim();

        // Extract --lines=START-END if present
        let lines_marker = "--lines=";
        let (path_raw, line_range) = if let Some(idx) = args_str.find(lines_marker) {
            let path_part = &args_str[..idx];
            let range_part = &args_str[idx + lines_marker.len()..];
            let range_str = range_part.trim().trim_matches('"').trim_matches('\'');

            let range = if let Some(dash) = range_str.find('-') {
                let start = range_str[..dash].parse::<usize>().unwrap_or(1);
                let end = range_str[dash + 1..].parse::<usize>().unwrap_or(usize::MAX);
                Some((start, end))
            } else if let Ok(single) = range_str.parse::<usize>() {
                Some((single, single))
            } else {
                None
            };
            (path_part, range)
        } else {
            (args_str, None)
        };

        let path = crate::utils::sanitize_path(path_raw);

        match fs::read_to_string(&path).await {
            Ok(content) => {
                let all_lines: Vec<&str> = content.lines().collect();
                let total_lines = all_lines.len();

                let (output_content, shown_start, shown_end) = if let Some((start, end)) = line_range {
                    let s = start.saturating_sub(1).min(total_lines); // 1-indexed to 0-indexed
                    let e = end.min(total_lines);
                    let selected: Vec<String> = all_lines[s..e]
                        .iter()
                        .enumerate()
                        .map(|(i, line)| format!("{}: {}", s + i + 1, line))
                        .collect();
                    (selected.join("\n"), s + 1, e)
                } else {
                    // Cap output at 500 lines to prevent context flooding
                    if total_lines > 500 {
                        let selected: Vec<String> = all_lines[..500]
                            .iter()
                            .enumerate()
                            .map(|(i, line)| format!("{}: {}", i + 1, line))
                            .collect();
                        let mut result = selected.join("\n");
                        result.push_str(&format!("\n\n[... {} more lines. Use --lines= to view specific ranges]", total_lines - 500));
                        (result, 1, 500)
                    } else {
                        (content.clone(), 1, total_lines)
                    }
                };

                let result = serde_json::json!({
                    "success": true,
                    "content": output_content,
                    "path": path,
                    "total_lines": total_lines,
                    "shown_range": format!("{}-{}", shown_start, shown_end)
                });
                Ok(result.to_string())
            },
            Err(e) => {
                let result = serde_json::json!({
                    "success": false,
                    "error": format!("Failed to read {}: {}", path, e),
                    "path": path
                });
                Ok(result.to_string())
            }
        }
    }
}
