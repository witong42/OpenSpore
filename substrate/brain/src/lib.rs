use openspore_core::config::AppConfig;
use openspore_skills::SkillLoader;
use openspore_memory::{MemorySystem, context::ContextManager};
use reqwest::Client;
use serde_json::json;
use serde::{Serialize, Deserialize};
use tracing::{info, error, warn};
use std::collections::HashMap;
use regex::Regex;
use std::sync::Arc;
use openspore_io::{NativeBridge, get_bridge};

// ============================================================================
// VECTOR PROFILES - Exact port of brain.js lines 8-35
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorProfile {
    pub horizon: f32,         // Long-term vs immediate focus
    pub abstraction: f32,     // First-principles vs concrete
    pub rigor: f32,           // MECE/proof vs heuristic
    pub velocity: f32,        // Rapid vs deliberate
    pub divergence: f32,      // Lateral vs convergent
    pub description: String,
    pub model_override: Option<ModelType>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModelType {
    Fast,
    Reasoning,
    Default,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventType {
    Heartbeat,
    Optimization,
    DailyJournal,
    Acknowledgment,
    General,
}

impl EventType {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().trim() {
            "HEARTBEAT" => Self::Heartbeat,
            "OPTIMIZATION" => Self::Optimization,
            "DAILY_JOURNAL" => Self::DailyJournal,
            "ACKNOWLEDGMENT" => Self::Acknowledgment,
            _ => Self::General,
        }
    }
}

fn get_vector_profiles() -> HashMap<EventType, VectorProfile> {
    let mut profiles = HashMap::new();

    profiles.insert(EventType::Heartbeat, VectorProfile {
        horizon: 0.2, abstraction: 0.1, rigor: 0.9, velocity: 0.8, divergence: 0.1,
        description: "rapid validation and system health check".to_string(),
        model_override: Some(ModelType::Fast),
        max_tokens: None,
    });

    profiles.insert(EventType::Optimization, VectorProfile {
        horizon: 0.8, abstraction: 0.7, rigor: 0.9, velocity: 0.3, divergence: 0.6,
        description: "deep architecture refinement".to_string(),
        model_override: Some(ModelType::Reasoning),
        max_tokens: None,
    });

    profiles.insert(EventType::DailyJournal, VectorProfile {
        horizon: 0.7, abstraction: 0.4, rigor: 0.8, velocity: 0.5, divergence: 0.3,
        description: "structured reflection".to_string(),
        model_override: None,
        max_tokens: None,
    });

    profiles.insert(EventType::Acknowledgment, VectorProfile {
        horizon: 0.1, abstraction: 0.1, rigor: 0.3, velocity: 0.9, divergence: 0.1,
        description: "minimal processing for simple affirmations".to_string(),
        model_override: Some(ModelType::Fast),
        max_tokens: Some(50),
    });

    profiles.insert(EventType::General, VectorProfile {
        horizon: 0.5, abstraction: 0.5, rigor: 0.7, velocity: 0.6, divergence: 0.4,
        description: "balanced synthesis".to_string(),
        model_override: None,
        max_tokens: None,
    });

    profiles
}

// ============================================================================
// MODELS - Exact port of brain.js lines 37-41
// ============================================================================

fn get_model(model_type: ModelType, _config: &AppConfig) -> String {
    match model_type {
        ModelType::Fast => std::env::var("OPENROUTER_MODEL_FAST")
            .unwrap_or_else(|_| "google/gemini-2.0-flash-001".to_string()),
        ModelType::Reasoning => std::env::var("OPENROUTER_MODEL_REASONING")
            .unwrap_or_else(|_| "anthropic/claude-3.5-sonnet".to_string()),
        ModelType::Default => std::env::var("OPENROUTER_MODEL")
            .unwrap_or_else(|_| "google/gemini-2.0-flash-001".to_string()),
    }
}

// ============================================================================
// BRAIN - Exact port of brain.js Brain class
// ============================================================================

pub struct Brain {
    client: Client,
    config: AppConfig,
    profiles: HashMap<EventType, VectorProfile>,
    skill_loader: Arc<SkillLoader>,
    memory: MemorySystem,
    context_manager: ContextManager,
    io: Arc<Box<dyn NativeBridge + Send + Sync>>,
}

impl Brain {
    pub fn new(config: AppConfig) -> Self {
        let state = openspore_core::state::AppState::new(config.clone());
        Self {
            client: Client::new(),
            memory: MemorySystem::new(&state),
            context_manager: ContextManager::new(&state),
            config,
            profiles: get_vector_profiles(),
            skill_loader: Arc::new(SkillLoader::new()),
            io: Arc::new(get_bridge()),
        }
    }

    /// Classify event type (brain.js lines 48-71)
    pub async fn classify_event_type(&self, query: &str) -> EventType {
        let prompt = format!(r#"
Categorize the following user request into one of these cognitive profiles:
- ACKNOWLEDGMENT: Simple affirmations (ok, thanks, roger).
- HEARTBEAT: System health, status checks, uptime, "are you there?".
- DAILY_JOURNAL: Personal reflections, recap of the day, logging thoughts.
- OPTIMIZATION: Architecture changes, refactoring code, improving logic, design blueprints.
- GENERAL: Everything else, research, coding, chatting.

Request: "{}"

Output ONLY the category name."#, query);

        let fast_profile = VectorProfile {
            horizon: 0.0, abstraction: 0.0, rigor: 0.0, velocity: 1.0, divergence: 0.0,
            description: "classification".to_string(),
            model_override: Some(ModelType::Fast),
            max_tokens: Some(10),
        };

        match self.complete(&[Message { role: "user".to_string(), content: prompt }], Some(&fast_profile)).await {
            Ok(response) => EventType::from_str(&response),
            Err(_) => EventType::General,
        }
    }

    /// Context Compression (lines 40-67 in JS context_manager.js)
    pub async fn compress_context(&self, existing_summary: &str, new_items: Vec<openspore_memory::MemoryItem>) -> Result<String, String> {
        let items_content = new_items.iter().map(|m| m.content.clone()).collect::<Vec<_>>().join("\n---\n");
        let prompt = format!(r#"
Summarize the following conversation history into the existing session summary.
Maintain all key decisions, code pointers, and user preferences established.

<EXISTING_SUMMARY>
{}
</EXISTING_SUMMARY>

<NEW_ITEMS_TO_COMPRESS>
{}
</NEW_ITEMS_TO_COMPRESS>

Provide a concise, updated summary of the session so far."#, existing_summary, items_content);

        let fast_profile = self.profiles.get(&EventType::Acknowledgment).unwrap();
        match self.complete(&[Message { role: "system".to_string(), content: prompt }], Some(fast_profile)).await {
            Ok(summary) => {
                // Save the new summary and delete older items
                let _ = tokio::fs::write(&self.context_manager.summary_path, &summary).await;
                for item in new_items {
                    let path = self.memory.memory_root.join("context").join(item.filename);
                    let _ = tokio::fs::remove_file(path).await;
                }
                Ok(summary)
            }
            Err(e) => Err(format!("Compression Error: {}", e)),
        }
    }

    /// Autonomous Learning (lines 286-341 in JS memory.js)
    pub async fn learn(&self, prompt: &str, response: &str) -> bool {
        let extraction_prompt = format!(r#"
You are an insight extractor. Analyze the following interaction.
User: "{}"
AI: "{}"

If the interaction contains:
1. User preferences (likes, dislikes, workflow habits)
2. New factual knowledge
3. Critical context for future tasks

Format the output EXACTLY as JSON:
{{
    "should_save": true,
    "category": "preferences" | "knowledge" | "context",
    "title": "Short Title",
    "content": "The actual content to save...",
    "tags": ["tag1", "tag2"]
}}

If nothing worth saving, set "should_save": false."#, prompt, response);

        let fast_profile = self.profiles.get(&EventType::Acknowledgment).unwrap();
        if let Ok(raw_json) = self.complete(&[Message { role: "user".to_string(), content: extraction_prompt }], Some(fast_profile)).await {
            let clean_json = raw_json.trim().trim_start_matches("```json").trim_end_matches("```").trim();
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(clean_json) {
                if data["should_save"].as_bool().unwrap_or(false) {
                    let category = data["category"].as_str().unwrap_or("knowledge");
                    let title = data["title"].as_str().unwrap_or("Insight");
                    let content = data["content"].as_str().unwrap_or("");
                    let tags: Vec<String> = data["tags"].as_array()
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();

                    if !content.is_empty() {
                        let _ = self.memory.save_memory(category, title, content, tags, Some("learned")).await;
                        info!("🧠 Learned new insight: {}", title);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Apply vector context (brain.js lines 73-97)
    fn apply_vector_context(&self, base_prompt: &str, event_type: EventType, vectors: &VectorProfile) -> String {
        let vector_instructions = format!(r#"
<VECTOR_ANALYSIS>
Event Type: {:?}
Cognitive Profile: {}
- Horizon: {} ({})
- Abstraction: {} ({})
- Rigor: {} ({})
- Velocity: {} ({})
- Divergence: {} ({})
</VECTOR_ANALYSIS>

<THINKING_PROTOCOL>
Based on current vector profile, modulate your response:
{}{}{}{}{}{}
- SPORE-FIRST: If a task can be parallelized, it MUST be delegated.
</THINKING_PROTOCOL>
"#,
            event_type,
            vectors.description,
            vectors.horizon, if vectors.horizon > 0.6 { "long-term architecture focus" } else { "immediate execution focus" },
            vectors.abstraction, if vectors.abstraction > 0.5 { "first-principles thinking" } else { "concrete implementation" },
            vectors.rigor, if vectors.rigor > 0.7 { "MECE/logical proof required" } else { "heuristic approach acceptable" },
            vectors.velocity, if vectors.velocity > 0.7 { "rapid execution, minimal deliberation" } else { "deep recursive deliberation" },
            vectors.divergence, if vectors.divergence > 0.5 { "lateral exploration encouraged" } else { "convergent accuracy focus" },
            if vectors.velocity > 0.8 { "- ULTRA-CONCISE: Prioritize speed. One sentence max.\n" } else { "" },
            if vectors.velocity < 0.4 { "- DEEP DELIBERATION: Step through reasoning explicitly. DELEGATE sub-tasks to Spores.\n" } else { "" },
            if vectors.rigor > 0.8 { "- PROFIT-LEVEL: Validate assumptions. Use [DELEGATE] for rigorous verification.\n" } else { "" },
            if vectors.abstraction > 0.6 { "- FIRST-PRINCIPLES: Question fundamentals. Use [DELEGATE] to research basics.\n" } else { "" },
            if vectors.horizon > 0.6 { "- ARCHITECTURAL VIEW: Consider technical debt. DELEGATE implementation chores to Spores.\n" } else { "" },
            if vectors.divergence > 0.5 { "- EXPLORE: Consider alternatives. DELEGATE comparative analysis tasks.\n" } else { "- FOCUSED: Direct answer.\n" },
        );

        format!("{}{}", base_prompt, vector_instructions)
    }

    /// Complete API call (brain.js lines 99-128)
    pub async fn complete(&self, messages: &[Message], profile: Option<&VectorProfile>) -> Result<String, String> {
        let api_key = &self.config.openrouter_api_key;
        if api_key.is_empty() {
            return Err("⚠️ OpenRouter API Key missing.".to_string());
        }

        let model_type = profile
            .and_then(|p| p.model_override)
            .unwrap_or(ModelType::Default);
        let selected_model = get_model(model_type, &self.config);

        let mut request_body = json!({
            "model": selected_model,
            "messages": messages.iter().map(|m| json!({"role": m.role, "content": m.content})).collect::<Vec<_>>()
        });

        if let Some(p) = profile {
            if let Some(max_tokens) = p.max_tokens {
                request_body["max_tokens"] = json!(max_tokens);
            }
        }

        match self.client.post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
        {
            Ok(res) => {
                let status = res.status();
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                        return Ok(content.to_string());
                    }
                    if let Some(err) = json["error"]["message"].as_str() {
                        return Err(format!("OpenRouter Error: {}", err));
                    }
                    return Err(format!("LLM Error ({}): {}", status, json));
                }
                Err(format!("Unexpected LLM response format (Status: {}).", status))
            }
            Err(e) => {
                error!("Request failed: {}", e);
                Err(format!("Request failed: {}", e))
            }
        }
    }

    /// Stitch context (brain.js lines 130-202)
    pub async fn stitch_context(&self, user_query: &str, event_type: EventType, vectors: &VectorProfile) -> String {
        let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let active_app = self.io.get_active_app().await.unwrap_or_else(|_| "Unknown".to_string());
        let spotify = self.io.get_spotify_status().await.unwrap_or_else(|_| "".to_string());

        let skills_prompt = self.skill_loader.get_system_prompt();

        // Fire parallel fetches (exact JS port lines 135-140)
        let search_count = if vectors.velocity > 0.8 { 2 } else { 5 };

        let prefs = self.memory.get_memories("preferences");
        let relevant = self.memory.search(user_query, search_count).await.unwrap_or_default();
        let is_spore = std::env::var("IS_SPORE").map(|v| v == "true").unwrap_or(false);
        let mut working_context = if !is_spore && vectors.velocity < 0.9 {
            self.context_manager.get_working_context().await.unwrap_or_default()
        } else {
            openspore_memory::context::WorkingContext { summary: "".to_string(), recent: "".to_string(), older_items: vec![] }
        };

        // Trigger compression if needed (v3.0 logic)
        if !working_context.older_items.is_empty() {
             info!("📉 Substrate: Compressing older turns into session summary...");
             if let Ok(new_summary) = self.compress_context(&working_context.summary, working_context.older_items).await {
                 working_context.summary = new_summary;
             }
        }

        let preferences_str = prefs.iter().map(|m| m.content.clone()).collect::<Vec<_>>().join("\n\n");
        let active_knowledge = if !relevant.is_empty() {
            relevant.iter().map(|r| format!("--- [File: {}] ---\n{}", r.title, r.content)).collect::<Vec<_>>().join("\n\n")
        } else {
            "No specific local knowledge relevant to this query.".to_string()
        };

        let mut base_prompt = format!(r#"
You are **OpenSpore**, a high-performance, proactive AI agent.
Time: {}
Active App: {}
{}
"#, time, active_app, if !spotify.is_empty() { format!("Music: {}", spotify) } else { "".to_string() });

        if is_spore {
            let role = std::env::var("SPORE_ROLE").unwrap_or_else(|_| "Expert".to_string());
            base_prompt.push_str(&format!(r#"
<SPORE_MISSION>
You are a SUB-SPORE with the role: {}.
You have been delegated a specific task. Focus ONLY on this task and return your findings concisely.
Do NOT attempt to manage the host or the session—the Manager Spore handles that.
</SPORE_MISSION>
"#, role));
        }

        base_prompt.push_str(&format!(r#"
<CORE_OBJECTIVES>
1. **Act, Don't Just Talk**: If you have a tool to solve a problem, USE IT immediately.
2. **Think Horizontally**: You are the **Manager Spore**. For any complex, multi-part, or time-consuming task, use the `delegate` tool to spawn sub-spores.
3. **Be Concise**: Answer directly. Avoid fluff.
4. **Use Memory**: You have access to a vast filesystem memory.
5. **Learn & adapt**: If the user corrects me, update my memory.
</CORE_OBJECTIVES>

<DELEGATION_STRATEGY>
Parallelize execution by default. If a task involves:
- Multi-step research or coding
- Concurrent operations (e.g., "Refactor X AND Search Y")
- Deep validation or "Red Teaming"
Spawn a specialized Spore with `[DELEGATE: "task" --role="ExpertRole"]`.
</DELEGATION_STRATEGY>

<SYSTEM_PREFERENCES>
{}
</SYSTEM_PREFERENCES>
"#, preferences_str));

        base_prompt = self.apply_vector_context(&base_prompt, event_type, vectors);

        base_prompt.push_str(&format!(r#"
<RELEVANT_KNOWLEDGE_FROM_FILESYSTEM>
{}
</RELEVANT_KNOWLEDGE_FROM_FILESYSTEM>

<AVAILABLE_TOOLS>
{}
Use tools efficiently. You can chain multiple tools in one response.
Format: [TOOL_NAME: "argument"]
</AVAILABLE_TOOLS>
"#, active_knowledge, skills_prompt));

        if vectors.velocity < 0.9 {
            base_prompt.push_str(&format!(r#"
<SESSION_SUMMARY>
{}
</SESSION_SUMMARY>

<RECENT_CONVERSATION>
{}
</RECENT_CONVERSATION>
"#, working_context.summary, working_context.recent));
        }

        base_prompt
    }

    /// Main think function with tool execution loop (brain.js lines 204-290)
    pub async fn think(&self, user_prompt: &str) -> String {
        let start_time = std::time::Instant::now();
        info!("🧠 Thinking: {}", user_prompt);

        // 1. Classify event type
        let event_type = self.classify_event_type(user_prompt).await;
        let vectors = self.profiles.get(&event_type).unwrap();

        info!("📊 Event Type: {:?}, Velocity: {}", event_type, vectors.velocity);

        // Auto-Journaling (JS lines 212-216)
        let is_spore = std::env::var("IS_SPORE").map(|v| v == "true").unwrap_or(false);
        if !is_spore {
            let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
            let journal_entry = format!("\n[{}] User: {} (Profile: {:?})\n", timestamp, user_prompt, event_type);
            let _ = self.memory.save_journal(&journal_entry).await;
        }

        // 2. Build system context
        let system_context = self.stitch_context(user_prompt, event_type, vectors).await;

        // 3. Initial completion
        let mut messages = vec![
            Message { role: "system".to_string(), content: system_context },
            Message { role: "user".to_string(), content: user_prompt.to_string() },
        ];

        let mut content = match self.complete(&messages, Some(vectors)).await {
            Ok(c) => c,
            Err(e) => return format!("Error: {}", e),
        };

        // 4. Tool execution loop (brain.js lines 228-274)
        if vectors.velocity < 0.9 {
            let max_depth = if vectors.velocity > 0.6 { 4 } else { 8 };
            let mut depth = 0;

            let mut i = 0;
            let chars: Vec<char> = content.chars().collect();
            let mut detected_tools = Vec::new();

            while i < chars.len() {
                if chars[i] == '[' {
                    let start = i;
                    let mut bracket_depth = 0;
                    let mut found_end = false;
                    let mut j = i;

                    while j < chars.len() {
                        if chars[j] == '[' {
                            bracket_depth += 1;
                        } else if chars[j] == ']' {
                            bracket_depth -= 1;
                            if bracket_depth == 0 {
                                let potential_tool = chars[start..=j].iter().collect::<String>();
                                // Validate if it's a tool call [NAME: args]
                                if let Some(colon_idx) = potential_tool.find(':') {
                                    let name_part = &potential_tool[1..colon_idx].trim();
                                    if !name_part.is_empty() && name_part.chars().all(|c| c.is_alphanumeric() || c == '_') {
                                        detected_tools.push((name_part.to_string(), potential_tool[colon_idx+1..potential_tool.len()-1].to_string()));
                                        i = j;
                                        found_end = true;
                                        break;
                                    }
                                }
                            }
                        }
                        j += 1;
                    }
                    if found_end {
                        i += 1;
                        continue;
                    }
                }
                i += 1;
            }

            while depth < max_depth {
                if detected_tools.is_empty() {
                    break;
                }

                let current_batch = std::mem::take(&mut detected_tools);
                let mut tool_outputs = String::from("\n<TOOL_OUTPUTS>\n");

                for (tool_name, mut tool_args_raw) in current_batch {
                    let mut tool_args = tool_args_raw.trim();

                    // Strip outer quotes if they wrap the entire args block
                    if (tool_args.starts_with('"') && tool_args.ends_with('"')) ||
                       (tool_args.starts_with('\'') && tool_args.ends_with('\'')) {
                        if tool_args.len() >= 2 {
                            tool_args = &tool_args[1..tool_args.len()-1];
                        }
                    }

                    let unescaped_args = tool_args.replace("\\\"", "\"").replace("\\'", "'");
                    let tool_args = unescaped_args.as_str();

                    if let Some(skill) = self.skill_loader.get(&tool_name) {
                        info!("⚙️ Executing skill [{}]: {}", tool_name, tool_args);

                        match skill.execute(tool_args).await {
                            Ok(result) => {
                                info!("✅ Skill result: {}", result.trim());
                                let log_ts = chrono::Local::now().format("%H:%M:%S").to_string();
                                let tool_log = format!("\n[{}] 🛠️ Executed {}: {}\nResult: {}...\n", log_ts, tool_name, tool_args, result.chars().take(200).collect::<String>());
                                let _ = self.memory.save_journal(&tool_log).await;

                                tool_outputs.push_str(&format!("\n--- Output from {} ---\n{}\n", tool_name, result));
                            }
                            Err(e) => {
                                error!("❌ Skill error: {}", e);
                                tool_outputs.push_str(&format!("\n--- Error from {} ---\n{}\n", tool_name, e));
                            }
                        }
                    } else {
                        warn!("⚠️ Unknown skill: {}", tool_name);
                    }
                }

                tool_outputs.push_str("\n</TOOL_OUTPUTS>\n");

                messages.push(Message { role: "assistant".to_string(), content: content.clone() });
                messages.push(Message {
                    role: "user".to_string(),
                    content: format!("{}\n\nBased on these outputs, do you need to take further action? If yes, use another tool. If you have the final answer, provide it clearly.", tool_outputs)
                });

                content = match self.complete(&messages, Some(vectors)).await {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Tool loop error: {}", e);
                        break;
                    }
                };

                // Re-detect tools in the new content
                let chars: Vec<char> = content.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    if chars[i] == '[' {
                        let start = i;
                        let mut bracket_depth = 0;
                        let mut found_end = false;
                        let mut j = i;
                        while j < chars.len() {
                            if chars[j] == '[' { bracket_depth += 1; }
                            else if chars[j] == ']' {
                                bracket_depth -= 1;
                                if bracket_depth == 0 {
                                    let potential_tool = chars[start..=j].iter().collect::<String>();
                                    if let Some(colon_idx) = potential_tool.find(':') {
                                        let name_part = &potential_tool[1..colon_idx].trim();
                                        if !name_part.is_empty() && name_part.chars().all(|c| c.is_alphanumeric() || c == '_') {
                                            detected_tools.push((name_part.to_string(), potential_tool[colon_idx+1..potential_tool.len()-1].to_string()));
                                            i = j;
                                            found_end = true;
                                            break;
                                        }
                                    }
                                }
                            }
                            j += 1;
                        }
                        if found_end { i += 1; continue; }
                    }
                    i += 1;
                }

                depth += 1;
            }
        }

        if !is_spore {
            // Persistence (JS lines 276-281)
            let ai_log_ts = chrono::Local::now().format("%H:%M:%S").to_string();
            let ai_log = format!("\n[{}] AI: {}\n\n---", ai_log_ts, content);
            let _ = self.memory.save_journal(&ai_log).await;

            if event_type != EventType::Acknowledgment {
                let _ = self.memory.save_memory("context", &format!("Exchange_{}", chrono::Local::now().timestamp()), &format!("User: {}\nAI: {}", user_prompt, content), vec![], None).await;
            }
        }

        if !is_spore {
            // Autonomous learning (lines 286-341 in memory.js)
            let brain_clone = self.clone_brain(); // We need a way to call learn without blocking
            let prompt_snapshot = user_prompt.to_string();
            let content_snapshot = content.clone();

            tokio::spawn(async move {
                brain_clone.learn(&prompt_snapshot, &content_snapshot).await;
            });
        }

        let elapsed = start_time.elapsed();
        info!("✅ Thought Cycle Complete in {:?}", elapsed);
        content
    }

    fn clone_brain(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
            profiles: self.profiles.clone(),
            skill_loader: Arc::clone(&self.skill_loader),
            memory: self.memory.clone_memory(),
            context_manager: self.context_manager.clone_manager(),
            io: Arc::clone(&self.io),
        }
    }

    /// Simple think without classification (for quick responses)
    pub async fn think_simple(&self, prompt: &str) -> String {
        let messages = vec![Message { role: "user".to_string(), content: prompt.to_string() }];
        match self.complete(&messages, None).await {
            Ok(content) => content,
            Err(e) => format!("Error: {}", e),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}
