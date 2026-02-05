use crate::{Brain, Message};
use regex::Regex;
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

        // Robust tool regex:
        // - Supports [NAME: ARG] with interior spaces
        // - Supports bare NAME: ARG
        // - Non-greedy arg capture to avoid consuming multiple tools in one line
        let tool_regex = Regex::new(r"(?m)(?:\[\s*)?(?P<name>[A-Z0-9_]{3,})\s*:\s*(?P<arg>.*?)(?:\s*\]|$)").unwrap();

        loop {
            if depth >= max_depth {
                warn!("⚠️ Depth limit hit ({}). Terminating tool loop to prevent infinite recursion.", max_depth);
                content.push_str("\n\n[SYSTEM: Maximum thinking depth reached. Please summarize your findings.]");
                break;
            }

            info!("Raw LLM Output (Depth {}): {:?}", depth, content);

            let mut tools_to_run = Vec::new();
            // Process the content Once. Redundant markdown extraction removed to prevent double execution.
            for cap in tool_regex.captures_iter(&content) {
                let name = cap["name"].trim().to_uppercase();
                let mut arg = cap["arg"].trim().to_string();

                // 1. Cleanup: Strip trailing bracket if it was part of the capture (bare-tool format match)
                if arg.ends_with(']') {
                    arg = arg[..arg.len()-1].trim().to_string();
                }

                // 2. Cleanup: Strip markdown markers if present inside arg
                if arg.contains("```") {
                    arg = arg.replace("```tool_code", "").replace("```json", "").replace("```", "").trim().to_string();
                }

                // 3. Quote cleanup
                if (arg.starts_with('"') && arg.ends_with('"')) || (arg.starts_with('\'') && arg.ends_with('\'')) {
                    if arg.len() >= 2 {
                        arg = arg[1..arg.len()-1].to_string();
                    }
                }

                // Validate if it's actually a registered tool to avoid false positives with normal text
                if self.skill_loader.get(&name).is_some() {
                    info!("🔎 Detected Tool: [{} : {}]", name, arg);
                    tools_to_run.push((name, arg));
                }
            }

            if tools_to_run.is_empty() {
                break;
            }

            // Execute Tools
            let mut tool_outputs = String::from("\n<TOOL_OUTPUTS>\n");
            for (name, arg) in tools_to_run {
                if let Some(skill) = self.skill_loader.get(&name) {
                    info!("⚙️ Executing: [{} : {}]", name, arg);
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
        if user_prompt.len() > 100 || content.len() > 200 {
            let log_entry = format!("\n[{}] 💬 Interaction: {} chars in, {} chars out\n",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                user_prompt.len(),
                content.len()
            );
            let _ = self.memory.save_journal(&log_entry).await;
        }

        info!("✅ Cycle finished in {:?}", start_time.elapsed());
        content
    }
}
