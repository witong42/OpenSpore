//! Search Skill (Core) - Search memory/workspace

use super::Skill;
use async_trait::async_trait;
use std::path::Path;

pub struct SearchSkill;

#[async_trait]
impl Skill for SearchSkill {
    fn name(&self) -> &'static str { "search" }

    fn description(&self) -> &'static str {
        "Search the workspace/memory for relevant files. Returns JSON with success and results (title/path/score). Usage: [SEARCH: \"query\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let args = args.trim();
        let mut query_text = args.to_string();
        let mut search_path = None;

        let path_marker = "--path=";
        if let Some(idx) = args.find(path_marker) {
            let start = idx + path_marker.len();
            let remainder = &args[start..];
            let end = remainder.find(' ').unwrap_or(remainder.len());
            let raw_p = remainder[..end].trim().trim_matches('"').trim_matches('\'');
            search_path = Some(openspore_core::path_utils::expand_tilde(raw_p));
            query_text = format!("{} {}", &args[..idx], &remainder[end..]).trim().to_string();
        }

        let query = query_text.trim().trim_matches('"').trim_matches('\'');

        let config = openspore_core::config::AppConfig::load()
            .map_err(|e| format!("Config error: {}", e))?;
        let state = openspore_core::state::AppState::new(config);
        let memory = openspore_memory::MemorySystem::new(&state);

        let results_res = if let Some(p) = search_path {
            let path = Path::new(&p);
            memory.search_in_path(query, path, 10).await
        } else {
            memory.search(query, 10).await
        };

        match results_res {
            Ok(results) => {
                let items: Vec<_> = results.into_iter().map(|r| {
                    serde_json::json!({
                        "title": r.title,
                        "path": r.path.to_string_lossy(),
                        "score": r.score
                    })
                }).collect();

                let res = serde_json::json!({
                    "success": true,
                    "query": query,
                    "results": items
                });
                Ok(res.to_string())
            },
            Err(e) => {
                let res = serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                    "query": query
                });
                Ok(res.to_string())
            }
        }
    }
}
