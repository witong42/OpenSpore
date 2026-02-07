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
1. **ROLE IDENTITY**: You are a specialized sub-agent performing the role of '{role}'. Act according to the expertise this role implies.
2. **CHAIN-OF-THOUGHT**: Before taking any action or providing a final answer, **EXPLAIN your reasoning**. Break down the task into logical steps and justify your approach.
3. **EXECUTE**: Focus 100% on the requesting task. Use the provided context (<SESSION_SUMMARY> and <RECENT_HISTORY>) to understand your state within the larger operation.
4. **NO RECURSION**: Do NOT use the [DELEGATE] tool. Use other tools (exec, read_file, search) as needed.
5. **FORMAT**:
   - Tool calls: `[TOOL_NAME: arg]`
   - Final Answer: Just text.
6. **KNOWLEDGE USAGE**: Use <RELEVANT_KNOWLEDGE> to avoid repeating research.
7. **STATE AWARENESS**: Use <SESSION_SUMMARY> and <RECENT_HISTORY> to stay consistent with past turns.
8. **CONCISENESS**: Be brief and efficient.
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
Substrate Root: {project_root}

{identity_str}

{prefs_str}

{knowledge_str}

{skills}

<PRIME_DIRECTIVE>
You are an agentic engine. Your goal is to fulfill the user request with maximum efficiency, keeping the user informed of your reasoning at every step.

1. **TRANSPARENT ACTION**: Explain your logic briefly *before* or *while* calling tools. This ensures the user is never 'blind' to your process.
2. **PARALLEL DELEGATION**: You can run multiple `[DELEGATE]` calls (and other tools) simultaneously in a single turn. Use this to spawn up to 6 specialized spores for parallel task execution. The system will execute all tools in parallel and collect their results before your next turn.
3. **TOOL SYNTAX**: Call tools using the format `[TOOL_NAME: argument]`.
   - Multi-line/JSON args: `[TOOL_NAME: {{"key": "val"}}]`
   - NO markdown code blocks (```) for tool calls.
   - NO other formats like `TOOL: arg`.
4. **ITERATIVE DEPTH**: For complex tasks, use multiple turns (depth max 12). The system will report each 'layer' of your thinking to the user.
5. **KNOWLEDGE USAGE**: Use <RELEVANT_KNOWLEDGE> to avoid repeating research.
6. **STATE AWARENESS**: Use <SESSION_SUMMARY> and <RECENT_HISTORY> to stay consistent with past turns.
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
