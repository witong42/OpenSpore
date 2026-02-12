use crate::{Brain, Message};
use tracing::{info, warn, error, debug};

impl Brain {
    /// The Core Thinking Loop: Minimalist & Robust
    pub(crate) async fn think_internal(&self, user_prompt: &str, tx: Option<tokio::sync::mpsc::Sender<crate::events::BrainEvent>>) -> String {
        let start_time = std::time::Instant::now();
        info!("🧠 Thinking: {}", user_prompt);

        // Immediate logging to LOGS.md (Start of turn)
        let start_log = format!("\n[{}] User: {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), user_prompt);
        let _ = self.memory.save_journal(&start_log).await;

        // 1. Build Context & System Prompt
        let (system_prompt, session_ctx) = crate::context_assembler::ContextAssembler::build_system_prompt(self, user_prompt).await;

        let mut messages = vec![
            Message { role: "system".to_string(), content: system_prompt.clone() },
            Message { role: "user".to_string(), content: user_prompt.to_string() }
        ];

        // 2. Initial Completion
        let mut content = match self.complete(&messages).await {
            Ok(c) => c,
            Err(e) => {
                if let Some(t) = &tx { let _ = t.send(crate::events::BrainEvent::Error(e.to_string())).await; }
                return format!("Errors: {}", e);
            }
        };

        // 3. Tool Loop
        let max_depth = 24;
        let mut depth = 0;

        loop {
            if depth >= max_depth {
                warn!("⚠️ Depth limit hit ({}). Terminating tool loop to prevent infinite recursion.", max_depth);
                content.push_str("\n\n[SYSTEM: Maximum thinking depth reached. Please summarize your findings.]");
                break;
            }

            debug!("Raw LLM Output (Depth {}): {:?}", depth, content);

            // Notify observer of the thinking layer
            if let Some(t) = &tx {
                let _ = t.send(crate::events::BrainEvent::ThoughtLayer {
                    depth,
                    content: content.clone(),
                }).await;
            } else {
                // Layer Visibility (Only if no observer is active, e.g. CLI one-shot)
                debug!("\n<details>\n<summary>▶ [Layer {}] Thinking Process</summary>\n\n{}\n\n</details>", depth, content);
            }


            // Robust Parser (State Machine) to handle nested brackets/JSON
            let tools_to_run = crate::parser::ToolParser::extract_tools(&content, &self.skill_loader);

            // Self-Correction: Check for common hallucinated tool formats (Markdown blocks)
            // We check this BEFORE deciding to break, because if the model tried to run a tool via markdown,
            // tools_to_run WILL be empty, and we want to catch it.
            if tools_to_run.is_empty() && (content.contains("```tool_code") || content.contains("```python") || content.contains("```javascript") || content.contains("```bash")) {
                 warn!("⚠️ Detected invalid markdown tool usage. Triggering self-correction.");

                 messages.push(Message { role: "assistant".into(), content: content.clone() });
                 messages.push(Message {
                     role: "user".into(),
                     content: "SYSTEM ERROR: You attempted to use a tool using Markdown code blocks (```). THIS IS INVALID. \n\nREQUIRED SYNTAX: `[TOOL_NAME: argument]`\n\nExample: `[DELEGATE: \"task\"]`\n\nPlease retry immediately with the correct syntax.".into()
                 });

                 match self.complete(&messages).await {
                    Ok(new_content) => {
                        content = new_content;
                        depth += 1;
                        continue;
                    },
                    Err(e) => {
                         error!("Re-think error during self-correction: {}", e);
                         break;
                    }
                 }
            }

            if tools_to_run.is_empty() {
                break;
            }

            // Execute Tools in Parallel
            use futures::stream::{FuturesUnordered, StreamExt};
            use std::pin::Pin;
            use futures::Future;

            let mut tool_tasks: FuturesUnordered<Pin<Box<dyn Future<Output = (String, Result<String, String>)> + Send>>> = FuturesUnordered::new();

            // State Verification: Track seen files in this turn's history
            let history_so_far = messages.iter().map(|m| m.content.clone()).collect::<Vec<_>>().join("\n");
            let destructive_tools = ["edit_file", "write_file", "diff_patch", "delegate"];

            for (name, arg) in tools_to_run {
                // Autonomous Safety Guard: Check if file was read before modification
                if destructive_tools.contains(&name.to_lowercase().as_str()) {
                    let path_to_verify = if name.to_lowercase() == "delegate" {
                        // For delegate, we just want to ensure it has SOME context, but it's less direct.
                        None
                    } else {
                        // Extract path from arg (might be JSON or raw command string)
                        if let Ok(json_arg) = serde_json::from_str::<serde_json::Value>(&arg) {
                            json_arg.get("path")
                                .or_else(|| json_arg.get("TargetFile"))
                                .or_else(|| json_arg.get("filename"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        } else {
                            // Fallback for non-JSON skills (e.g., "path" --content="content")
                            // We take the first part before any space or flag
                            let path = arg.split_whitespace().next().unwrap_or("").trim_matches('"').to_string();
                            if path.is_empty() { None } else { Some(path) }
                        }
                    };

                    if let Some(path) = path_to_verify {
                        let absolute_path = openspore_core::path_utils::ensure_absolute(&path).to_string_lossy().to_string();

                        // SKIP VERIFICATION for internal system files that the AI is expected to manage
                        // or that are already partially present in the system prompt context.
                        let is_internal = absolute_path.ends_with("session_summary.md") ||
                                         absolute_path.ends_with("LOGS.md") ||
                                         absolute_path.contains("/workspace/context/");

                        if !is_internal && !history_so_far.contains(&absolute_path) && !system_prompt.contains(&absolute_path) {
                             warn!("🛑 State Verification Failure: AI tried to modify {} without reading it first.", absolute_path);
                             tool_tasks.push(Box::pin(async move {
                                 (name, Err(format!("ERROR: State Verification Refused. You must use `READ_FILE` or `LIST_DIR` on '{}' to verify its current state before attempting to modify it. Blind writes are forbidden for safety. (Tip: Use full absolute paths)", absolute_path)))
                             }));
                             continue;
                        }
                    }
                }

                let skill_loader = &self.skill_loader;
                let tx = tx.clone();

                tool_tasks.push(Box::pin(async move {
                    if let Some(skill) = skill_loader.get(&name) {
                        info!("⚙️ Executing: [{} : {}]", name, arg);

                        if let Some(t) = &tx {
                            let _ = t.send(crate::events::BrainEvent::ToolExecution {
                                name: name.clone(),
                                arg: arg.clone(),
                            }).await;
                        }

                        match skill.execute(&arg).await {
                            Ok(output) => {
                                if let Some(t) = &tx {
                                    let _ = t.send(crate::events::BrainEvent::ToolResult {
                                        name: name.clone(),
                                        output: output.clone(),
                                        success: true,
                                    }).await;
                                }
                                (name, Ok(output))
                            },
                            Err(e) => {
                                if let Some(t) = &tx {
                                    let _ = t.send(crate::events::BrainEvent::ToolResult {
                                        name: name.clone(),
                                        output: e.clone(),
                                        success: false,
                                    }).await;
                                }
                                (name, Err(e))
                            }
                        }
                    } else {
                        (name.clone(), Err(format!("Unknown tool '{}'", name)))
                    }
                }));
            }

            let mut tool_outputs = String::from("\n<TOOL_OUTPUTS>\n");
            while let Some((name, result)) = tool_tasks.next().await {
                match result {
                    Ok(output) => {
                        tool_outputs.push_str(&format!("\n--- Output from {} ---\n{}\n", name, output));
                    },
                    Err(e) => {
                        error!("❌ Error executing {}: {}", name, e);
                        tool_outputs.push_str(&format!("\n--- Error from {} ---\n{}\n", name, e));
                    }
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
                    if let Some(t) = &tx { let _ = t.send(crate::events::BrainEvent::Error(e.to_string())).await; }
                    break;
                }
            }
            depth += 1;
        }

        // Final answer notification
        if let Some(t) = &tx {
            let _ = t.send(crate::events::BrainEvent::FinalAnswer(content.clone())).await;
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
                         warn!("Context compression notice (might be parallel task): {}", e);
                    }
                }
            }
        });

        // Log FULL raw interaction to LOGS.md (Result of turn)
        let end_log = format!("\nAI: {}\n", content);
        if let Err(e) = self.memory.save_journal(&end_log).await {
            error!("❌ Failed to save journal entry to LOGS.md: {}", e);
        }

        info!("✅ Cycle finished in {:?}", start_time.elapsed());
        content
    }
}
