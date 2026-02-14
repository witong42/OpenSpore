//! Grep Skill (Core)
//! Search for text patterns in files using system grep.

use super::Skill;
use async_trait::async_trait;
use tokio::process::Command;

pub struct GrepSkill;

#[async_trait]
impl Skill for GrepSkill {
    fn name(&self) -> &'static str { "grep" }

    fn description(&self) -> &'static str {
        "Search for text patterns in files recursively. Returns matching lines with file paths and line numbers. Usage: [GREP: \"pattern\" --path=\"/search/dir\"] or [GREP: \"pattern\"] (searches from CWD). Supports --include=\"*.ext\" for filtering."
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let args_str = args.trim();

        // Parse arguments
        let pattern;
        let mut search_path = crate::utils::get_virtual_cwd().to_string_lossy().to_string();
        let mut include_glob = String::new();

        // Extract --path= if present
        let path_marker = "--path=";
        let include_marker = "--include=";
        let mut remaining = args_str.to_string();

        if let Some(idx) = remaining.find(path_marker) {
            let start = idx + path_marker.len();
            let rest = &remaining[start..];
            let (val, end_offset) = extract_quoted_or_word(rest);
            search_path = crate::utils::sanitize_path(&val);
            remaining = format!("{}{}", &remaining[..idx], &rest[end_offset..]);
        }

        if let Some(idx) = remaining.find(include_marker) {
            let start = idx + include_marker.len();
            let rest = &remaining[start..];
            let (val, end_offset) = extract_quoted_or_word(rest);
            include_glob = val;
            remaining = format!("{}{}", &remaining[..idx], &rest[end_offset..]);
        }

        // Remaining text is the pattern
        pattern = remaining.trim().trim_matches('"').trim_matches('\'').to_string();

        if pattern.is_empty() {
            return Ok(serde_json::json!({
                "success": false,
                "error": "Usage: [GREP: \"pattern\" --path=\"/dir\"]"
            }).to_string());
        }

        // Build grep command
        let mut cmd = Command::new("grep");
        cmd.arg("-rn")           // recursive + line numbers
           .arg("--color=never") // no ANSI codes
           .arg("-I");           // skip binary files

        if !include_glob.is_empty() {
            cmd.arg(format!("--include={}", include_glob));
        }

        // Limit output to prevent context flooding
        cmd.arg("-m").arg("50"); // max 50 matches per file

        cmd.arg("--").arg(&pattern).arg(&search_path);

        let output = cmd.output().await
            .map_err(|e| format!("Failed to run grep: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Parse results into structured format
        let mut matches: Vec<serde_json::Value> = Vec::new();
        for line in stdout.lines().take(100) { // Hard cap at 100 results
            // Format: filepath:linenum:content
            if let Some(first_colon) = line.find(':') {
                let file = &line[..first_colon];
                let rest = &line[first_colon + 1..];
                if let Some(second_colon) = rest.find(':') {
                    let line_num = &rest[..second_colon];
                    let content = &rest[second_colon + 1..];
                    matches.push(serde_json::json!({
                        "file": file,
                        "line": line_num.parse::<u64>().unwrap_or(0),
                        "content": content.trim()
                    }));
                }
            }
        }

        let exit_code = output.status.code().unwrap_or(-1);
        // grep returns 1 for "no match" which is not an error
        let success = exit_code == 0 || exit_code == 1;

        Ok(serde_json::json!({
            "success": success,
            "pattern": pattern,
            "search_path": search_path,
            "match_count": matches.len(),
            "matches": matches,
            "stderr": if stderr.is_empty() { None } else { Some(stderr) }
        }).to_string())
    }
}

/// Extract a quoted value or a whitespace-delimited word from the start of a string.
/// Returns (value, chars_consumed).
fn extract_quoted_or_word(s: &str) -> (String, usize) {
    let s = s.trim_start();

    if s.starts_with('"') || s.starts_with('\'') {
        let quote = s.chars().next().unwrap();
        if let Some(end) = s[1..].find(quote) {
            return (s[1..1 + end].to_string(), 1 + end + 1);
        }
    }

    // No quotes, take until whitespace
    let end = s.find(char::is_whitespace).unwrap_or(s.len());
    (s[..end].to_string(), end)
}
