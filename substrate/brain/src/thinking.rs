use crate::{Brain, Message};
use tracing::{info, warn, error};

impl Brain {
    /// The Core Thinking Loop: Minimalist & Robust
    pub async fn think(&self, user_prompt: &str) -> String {
        let start_time = std::time::Instant::now();
        info!("🧠 Thinking: {}", user_prompt);

        // 1. Build Context & System Prompt
        let (system_prompt, session_ctx) = crate::context_assembler::ContextAssembler::build_system_prompt(self, user_prompt).await;

        let mut messages = vec![
            Message { role: "system".to_string(), content: system_prompt },
            Message { role: "user".to_string(), content: user_prompt.to_string() }
        ];

        let mut content = match self.complete(&messages).await {
            Ok(c) => c,
            Err(e) => return format!("Errors: {}", e),
        };

        // 3. Tool Loop
        let max_depth = 12;
        let mut depth = 0;

        loop {
            if depth >= max_depth {
                warn!("⚠️ Depth limit hit ({}). Terminating tool loop to prevent infinite recursion.", max_depth);
                content.push_str("\n\n[SYSTEM: Maximum thinking depth reached. Please summarize your findings.]");
                break;
            }

            info!("Raw LLM Output (Depth {}): {:?}", depth, content);

            // Robust Parser (State Machine) to handle nested brackets/JSON
            let tools_to_run = self.extract_tools(&content);

            if tools_to_run.is_empty() {
                break;
            }

            // Execute Tools
            let mut tool_outputs = String::from("\n<TOOL_OUTPUTS>\n");
            for (name, arg) in tools_to_run {
                if let Some(skill) = self.skill_loader.get(&name) {
                    info!("⚙️ Executing: [{} : {}]", name, arg);
                    println!("⚙️ Executing: [{} : {}]", name, arg); // Force visual output in TUI
                    match skill.execute(&arg).await {
                        Ok(output) => {
                            let preview = output.chars().take(100).collect::<String>();
                            info!("✅ Result: {}...", preview);
                            tool_outputs.push_str(&format!("\n--- Output from {} ---\n{}\n", name, output));
                        },
                        Err(e) => {
                            error!("❌ Error executing {}: {}", name, e);
                            tool_outputs.push_str(&format!("\n--- Error from {} ---\n{}\n", name, e));
                        }
                    }
                } else {
                    warn!("⚠️ Unknown tool: {}", name);
                    tool_outputs.push_str(&format!("\n--- Error ---\nUnknown tool '{}'\n", name));
                }
            }
            tool_outputs.push_str("\n</TOOL_OUTPUTS>\n");

            // Feedback Loop
            messages.push(Message { role: "assistant".to_string(), content: content.clone() });
            messages.push(Message {
                role: "user".to_string(),
                content: format!("{}\n\nProcess the results. If more actions needed, use tools. If done, provide final answer.", tool_outputs)
            });

            match self.complete(&messages).await {
                Ok(new_content) => content = new_content,
                Err(e) => {
                    error!("Re-think error: {}", e);
                    break;
                }
            }
            depth += 1;
        }

        // Save interaction for Watchman to analyze
        let exchange = format!("**User**: {}\n\n**Assistant**: {}", user_prompt, content);
        let _ = self.context_manager.save_interaction(
            &exchange,
            vec!["conversation".to_string()],
            Some("exchange")
        ).await;

        // Learn from interaction (extract preferences/knowledge)
        tokio::spawn({
            let brain = self.clone_brain();
            let prompt = user_prompt.to_string();
            let resp = content.clone();
            let older_items = session_ctx.older_items.clone();
            async move {
                // 1. Learn from exchange
                brain.learn(&prompt, &resp).await;

                // 2. Compress context if needed (deferred)
                if !older_items.is_empty() {
                    if let Err(e) = brain.context_manager.compress_older_items(older_items, &brain).await {
                         // Gracefully log but don't fail, as a parallel task might have already compressed
                         warn!("Context compression notice (might be parallel task): {}", e);
                    }
                }
            }
        });

        // Log significant events to LOGS.md
        // Log FULL raw interaction to LOGS.md
        let log_entry = format!("\n[{}]\nUser: {}\n\nAI: {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            user_prompt,
            content
        );
        let _ = self.memory.save_journal(&log_entry).await;

        info!("✅ Cycle finished in {:?}", start_time.elapsed());
        content
    }

    fn extract_tools(&self, content: &str) -> Vec<(String, String)> {
        let mut tools = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            // Find start: '[' followed by 'NAME:'
            if chars[i] == '[' {
                // Scan forward for ':' to check if this is a tool candidate
                let mut j = i + 1;
                while j < len && (chars[j].is_ascii_uppercase() || chars[j].is_numeric() || chars[j] == '_') {
                    j += 1;
                }

                // Must be at least 3 chars name and followed by ':'
                if j > i + 3 && j < len && chars[j] == ':' {
                    let name: String = chars[i+1..j].iter().collect(); // Extract NAME

                    // Start parsing ARG after ':'
                    let arg_start = j + 1;
                    let mut current = arg_start;
                    let mut depth = 1; // We rely on the initial '[' as depth 1
                    let mut in_quote = false;
                    let mut quote_char = '\0';
                    let mut escape = false;

                    while current < len {
                        let c = chars[current];

                        if escape {
                            escape = false;
                        } else if c == '\\' {
                            escape = true;
                        } else if in_quote {
                            if c == quote_char {
                                in_quote = false;
                            }
                        } else {
                            match c {
                                '"' | '\'' | '`' => {
                                    in_quote = true;
                                    quote_char = c;
                                }
                                '[' => depth += 1,
                                ']' => {
                                    depth -= 1;
                                    if depth == 0 {
                                        // Found end of tool
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                        current += 1;
                    }

                    if depth == 0 {
                        // Found valid tool
                        let raw_arg: String = chars[arg_start..current].iter().collect();
                        let mut arg = raw_arg.trim().to_string();

                         // Cleanup markdown/quotes logic from before
                        if arg.contains("```") {
                             arg = arg.replace("```tool_code", "").replace("```json", "").replace("```", "").trim().to_string();
                        }
                        if (arg.starts_with('"') && arg.ends_with('"')) || (arg.starts_with('\'') && arg.ends_with('\'')) {
                             if arg.len() >= 2 {
                                 arg = arg[1..arg.len()-1].to_string();
                             }
                        }

                        // Validate against SkillLoader
                        if self.skill_loader.get(&name).is_some() {
                            // info!("🔎 Detected Tool: [{} : {}]", name, arg);
                            tools.push((name, arg));
                        }

                        i = current; // Advance main loop
                        continue;
                    }
                }
            }
            i += 1;
        }
        tools
    }
}
