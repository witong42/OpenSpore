//! Search Skill (Core) - Search memory/workspace

use super::Skill;
use async_trait::async_trait;

pub struct SearchSkill;

#[async_trait]
impl Skill for SearchSkill {
    fn name(&self) -> &'static str { "search" }

    fn description(&self) -> &'static str {
        "Search the workspace/memory for relevant files. Usage: [SEARCH: \"query\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let query = args.trim().trim_matches('"').trim_matches('\'');

        let config = openspore_core::config::AppConfig::load()
            .map_err(|e| format!("Config error: {}", e))?;
        let state = openspore_core::state::AppState::new(config);
        let memory = openspore_memory::MemorySystem::new(&state);

        let results = memory.search(query, 5)
            .await
            .map_err(|e| format!("Search error: {}", e))?;

        if results.is_empty() {
            return Ok("No results found.".to_string());
        }

        let mut output = format!("Found {} results:\n", results.len());
        for r in results {
            output.push_str(&format!("- {} (score: {})\n", r.title, r.score));
        }
        Ok(output)
    }
}
