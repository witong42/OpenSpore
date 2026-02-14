use openspore_skills::SkillLoader;

pub struct ToolParser;

impl ToolParser {
    pub fn extract_tools(content: &str, skill_loader: &SkillLoader) -> Vec<(String, String)> {
        let mut tools = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let mut start_marker_len = 0;
            let mut is_markdown = false;

            // 1. Detect tool start marker
            if chars[i] == '[' {
                start_marker_len = 1;
            } else if i + 12 <= len && chars[i..i+12].iter().collect::<String>() == "```tool_code" {
                start_marker_len = 12;
                is_markdown = true;
            } else if i + 7 <= len && chars[i..i+7].iter().collect::<String>() == "```json" {
                // Surgical Check: Only treat ```json as a tool if it contains a NAME: pattern
                let peek_start = i + 7;
                let mut peek = peek_start;
                while peek < len && chars[peek].is_whitespace() { peek += 1; }

                let name_start = peek;
                while peek < len && (chars[peek].is_ascii_uppercase() || chars[peek].is_numeric() || chars[peek] == '_') {
                    peek += 1;
                }

                if peek > name_start + 1 && peek < len && chars[peek] == ':' {
                    start_marker_len = 7;
                    is_markdown = true;
                }
            }

            if start_marker_len > 0 {
                let mut j = i + start_marker_len;
                while j < len && chars[j].is_whitespace() { j += 1; }

                let name_start = j;
                while j < len && (chars[j].is_ascii_uppercase() || chars[j].is_numeric() || chars[j] == '_') {
                    j += 1;
                }

                if j > name_start + 1 && j < len && chars[j] == ':' {
                    let name: String = chars[name_start..j].iter().collect();
                    let arg_start = j + 1;
                    let mut current = arg_start;
                    let mut depth = 1;
                    let mut in_quote = false;
                    let mut quote_char = '\0';
                    let mut escape = false;
                    let mut found_end = false;

                    while current < len {
                        if !is_markdown {
                            let c = chars[current];
                            if escape { escape = false; }
                            else if c == '\\' { escape = true; }
                            else if in_quote { if c == quote_char { in_quote = false; } }
                            else {
                                match c {
                                    '"' | '\'' | '`' => { in_quote = true; quote_char = c; }
                                    '[' => depth += 1,
                                    ']' => { depth -= 1; if depth == 0 { found_end = true; break; } }
                                    _ => {}
                                }
                            }
                        } else {
                            if current + 3 <= len && chars[current..current+3].iter().collect::<String>() == "```" {
                                if !in_quote {
                                    found_end = true;
                                    break;
                                }
                            }
                            let c = chars[current];
                            if escape { escape = false; }
                            else if c == '\\' { escape = true; }
                            else if in_quote { if c == quote_char { in_quote = false; } }
                            else if c == '"' || c == '\'' || c == '`' { in_quote = true; quote_char = c; }
                        }
                        current += 1;
                    }

                    if found_end {
                        let raw_arg: String = chars[arg_start..current].iter().collect();
                        let arg = raw_arg.trim().to_string();

                        if skill_loader.get(&name).is_some() {
                            tools.push((name, arg));
                        }

                        i = if is_markdown { current + 3 } else { current + 1 };
                        continue;
                    }
                }
            }
            i += 1;
        }
        tools
    }

    /// Robustly splits arguments, respecting quotes and escapes.
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
}
