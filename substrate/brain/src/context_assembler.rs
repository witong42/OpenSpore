use crate::Brain;
use tracing::info;

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

        info!("📊 Cognition: Standard Mode");

        // 2. Format Context
        let skills = brain.skill_loader.get_system_prompt();
        let time = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

        let session_str = if !session_ctx.recent.is_empty() {
            format!("<SESSION_HISTORY>\n{}\n</SESSION_HISTORY>", session_ctx.recent)
        } else { "".to_string() };

        let knowledge_str = if !relevant.is_empty() {
             let items = relevant.iter().map(|s| format!("--- File: {} ---\n{}", s.title, s.content)).collect::<Vec<_>>().join("\n\n");
             format!("<RELEVANT_KNOWLEDGE>\n{}\n</RELEVANT_KNOWLEDGE>", items)
        } else { "".to_string() };

        let prefs_str = if !prefs.is_empty() {
            let items = prefs.iter().take(5).map(|m| format!("- {}", m.content)).collect::<Vec<_>>().join("\n");
            format!("<USER_PREFERENCES>\n{}\n</USER_PREFERENCES>", items)
        } else { "".to_string() };

        let mut identity_str = if !identity.is_empty() {
            let items = identity.iter().map(|m| m.content.clone()).collect::<Vec<_>>().join("\n\n");
            format!("<IDENTITY>\n{}\n</IDENTITY>", items)
        } else { "".to_string() };

        // 2e. Swarm Identity Overlays
        if std::env::var("IS_SPORE").is_ok() {
            let role = std::env::var("SPORE_ROLE").unwrap_or_else(|_| "Sub-Agent".to_string());
            identity_str.push_str(&format!(
                "\n<SPORE_IDENTITY>\nYou are a specialized sub-spore with the role: {}.\nFocus EXCLUSIVELY on your assigned task. Your output will be consumed by the Parent Spore.\n</SPORE_IDENTITY>",
                role
            ));
        }

        let prompt = format!(r#"
You are OpenSpore, an autonomous AI system.
Time: {time}

<PRIME_DIRECTIVE>
1. **Action over Talk**: Use tools immediately to solve the request.
2. **Mandatory Format**: Call tools EXACTLY like this: `[TOOL_NAME: argument]`.
   - Correct: [SYS_INFO: ]
   - Correct: [WRITE_FILE: file.txt --content="hello"]
3. **Complex Args**: Use JSON for code/multiline: `[SUBMIT_SKILL: {{"filename": "f.js", "code": "..."}}]`.
4. **CRITICAL**: Do NOT use markdown code blocks (e.g., ```tool_code) for tool calls.
5. **CRITICAL**: Do NOT use any other format like `TOOL_NAME: argument`. ONLY use `[TOOL_NAME: argument]`.
</PRIME_DIRECTIVE>

{identity_str}

{prefs_str}

{session_str}

{knowledge_str}

{skills}

<USER_REQUEST>
{user_prompt}
</USER_REQUEST>
"#);
        (prompt, session_ctx)
    }
}
