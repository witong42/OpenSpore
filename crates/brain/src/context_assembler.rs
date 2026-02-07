use crate::Brain;

pub struct ContextAssembler;

use openspore_memory::context::WorkingContext;

impl ContextAssembler {
    pub async fn build_system_prompt(brain: &Brain, user_prompt: &str) -> (String, WorkingContext) {
        // 1. Context Loading
        let ctx_fut = brain.context_manager.get_working_context(Some(brain));
        let memory_fut = brain.memory.search_memories(user_prompt, 3);
        let prefs_fut = async { brain.memory.get_memories("preferences") };
        let identity_fut = async { brain.memory.get_memories("identity") };

        let (session_ctx_res, relevant, prefs, identity) = tokio::join!(
            ctx_fut,
            memory_fut,
            prefs_fut,
            identity_fut
        );

        let session_ctx = session_ctx_res.unwrap_or_default();
        let project_root = brain.config.project_root.display().to_string();

        // 2. Format Context
        let skills = if std::env::var("IS_SPORE").is_ok() {
            // Sub-spores get all tools EXCEPT 'delegate' to prevent recursion
            brain.skill_loader.get_system_prompt(&["delegate"])
        } else {
             // Parent spore gets everything
            brain.skill_loader.get_system_prompt(&[])
        };
        let time = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

        let summary_str = if !session_ctx.summary.is_empty() && session_ctx.summary != "No session summary available." {
            format!("<SESSION_SUMMARY>\n{}\n</SESSION_SUMMARY>", session_ctx.summary)
        } else { "".to_string() };

        let recent_str = if !session_ctx.recent.is_empty() {
            format!("<RECENT_HISTORY>\n{}\n</RECENT_HISTORY>", session_ctx.recent)
        } else { "".to_string() };

        let knowledge_str = if !relevant.is_empty() {
             let items = relevant.iter().map(|s| format!("--- File: {} ---\n{}", s.title, s.content)).collect::<Vec<_>>().join("\n\n");
             format!("<RELEVANT_KNOWLEDGE>\n{}\n</RELEVANT_KNOWLEDGE>", items)
        } else { "".to_string() };

        let prefs_str = if !prefs.is_empty() {
            let items = prefs.iter().take(5).map(|m| format!("- {}", m.content)).collect::<Vec<_>>().join("\n");
            format!("<USER_PREFERENCES>\n{}\n</USER_PREFERENCES>", items)
        } else { "".to_string() };

        let identity_str = if !identity.is_empty() {
            let items = identity.iter().map(|m| m.content.clone()).collect::<Vec<_>>().join("\n\n");
            format!("<IDENTITY>\n{}\n</IDENTITY>", items)
        } else { "".to_string() };

        // Swarm Identity Overlays
        if std::env::var("IS_SPORE").is_ok() {
            let role = std::env::var("SPORE_ROLE").unwrap_or_else(|_| "Sub-Agent".to_string());

            // Lean Spore Prompt
            let prompt = format!(r#"You are a specialized OpenSpore Sub-Agent.
Role: {role}
Root: {project_root}

{knowledge_str}

{skills}

<PRIME_DIRECTIVE>
1. **ROLE IDENTITY**: You are a specialized sub-agent performing the role of '{role}'.
2. **VALIDATION PULSE**: Never assume file content or directory state based on history alone. Use `READ_FILE` or `LIST_DIR` to verify reality before editing or executing scripts.
3. **CHAIN-OF-THOUGHT**: Explain your reasoning *before* taking action.
4. **NO RECURSION**: Do NOT use the [DELEGATE] tool.
5. **FORMAT**: Use `[TOOL_NAME: arg]`. Final Answer MUST be **Natural Language (Markdown)**. Never respond with raw JSON.
6. **SAFE MODE**: If `SAFE_MODE_ENABLED=true`, modifying `crates/` (engine) or root config is strictly forbidden. Modifying `skills/` and `workspace/` is permitted.
7. **STOPPING CRITERIA**: If the task is finished in history, stop and report.
</PRIME_DIRECTIVE>

{recent_str}

<TASK>
{user_prompt}
</TASK>"#);
            return (prompt, session_ctx);
        }

        // Standard Main Agent Prompt
        let prompt = format!(r#"You are OpenSpore, an autonomous AI system.
Current Time: {time}
Engine Root: {project_root}

{identity_str}

{prefs_str}

{knowledge_str}

{skills}

<PRIME_DIRECTIVE>
1. **ACTION FIRST**: Explain your logic briefly *before* calling tools. Stay focused on the immediate task.
2. **VALIDATION PULSE**: Never assume file content or directory state based on history alone. You MUST use `READ_FILE` or `LIST_DIR` to verify reality before editing or executing scripts you didn't create in the current turn.
3. **TOOL SYNTAX**: Use `[TOOL_NAME: arg]`. For JSON args: `[TOOL_NAME: {{"k": "v"}}]`. No markdown code blocks for tool calls.
4. **PARALLELISM**: Use up to 6 simultaneous `[DELEGATE]` or tool calls in one turn for maximum efficiency.
5. **SAFE MODE**: If `SAFE_MODE_ENABLED=true`, modifying `crates/` (engine) or root config is strictly forbidden. Modifying `skills/` and `workspace/` is permitted and encouraged.
6. **RESPONSE FORMAT**: Use **Natural Language (Markdown)**. Never respond with a raw JSON object. Use code blocks ONLY for file content.
7. **STOPPING CRITERIA**: If the task is clearly finished in the `<RECENT_HISTORY>`, do NOT re-run it. Provide a final summary and stop.
</PRIME_DIRECTIVE>

{summary_str}

{recent_str}

<USER_REQUEST>
{user_prompt}
</USER_REQUEST>
"#);
        (prompt, session_ctx)
    }
}
