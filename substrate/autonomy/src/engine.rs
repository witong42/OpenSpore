use openspore_brain::Brain;
use regex::Regex;
use openspore_memory::MemorySystem;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, error};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct Idea {
    pub title: String,
    pub description: String,
    #[serde(rename = "type")]
    pub idea_type: String,
    pub implementation_plan: Option<String>,
    pub code_snippet: Option<String>,
}

pub struct AutonomyEngine;

impl AutonomyEngine {
    pub async fn run(brain: &Brain, memory: &MemorySystem) -> Result<Option<PathBuf>> {
        info!("💡 Autonomy: Scanning for improvement opportunities...");

        // 1. Get recent context
        let logs = memory.get_memories("context");
        let recent_logs = logs.iter().rev().take(5).map(|m| m.content.clone()).collect::<Vec<_>>().join("\n\n");

        let prompt = format!(r#"
You are the 'Idea Spore'. Your job is to look at the agent's recent logs and codebase structure and propose ONE concrete improvement or feature.

<RECENT_LOGS>
{recent_logs}
</RECENT_LOGS>

Criteria:
1. Must be code-related (refactor, new script, bugfix).
2. Must be SAFE (no deletion of data).
3. Must be valuable to the user.
4. **NO REPETITION**: If the logs show a similar proposal was just made or failed, DO NOT propose it again. Suggest something completely different (e.g., UI, performance, documentation).

Output JSON ONLY:
{{
    "title": "Short Title",
    "description": "Why this is good...",
    "type": "feature" | "refactor" | "fix",
    "implementation_plan": "Step 1... Step 2...",
    "code_snippet": "The actual code (if applicable)"
}}
"#);

        let raw = brain.think(&prompt).await;
        let trimmed = raw.trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();

        // Use regex to extract the JSON object if there's surrounding text
        let re = Regex::new(r"(?s).*?(\{.*\}).*").unwrap();
        let clean_json = match re.captures(trimmed) {
            Some(caps) => caps.get(1).map_or(trimmed, |m| m.as_str()),
            None => trimmed,
        };

        let idea: Idea = match serde_json::from_str(clean_json) {
            Ok(i) => i,
            Err(e) => {
                error!("Failed to parse idea JSON: {}. Raw: {}", e, clean_json);
                return Ok(None);
            }
        };

        // 2. Create Proposal
        let proposal_path = Self::create_proposal(&idea, memory)?;
        info!("✨ NEW PROPOSAL GENERATED: {}", proposal_path.display());

        Ok(Some(proposal_path))
    }

    fn create_proposal(idea: &Idea, memory: &MemorySystem) -> Result<PathBuf> {
        let id = format!("proposal_{}", chrono::Local::now().format("%Y%m%d_%H%M%S"));
        let filename = format!("{}.md", id);
        let proposals_dir = memory.project_root.join("workspace/autonomy/proposals");

        if !proposals_dir.exists() {
            std::fs::create_dir_all(&proposals_dir)?;
        }

        let content = format!(r#"---
id: {id}
title: {title}
type: {idea_type}
status: PENDING
created: {created}
---

# {title}

## Description
{description}

## Implementation Plan
{plan}

## Proposed Code
```javascript
{code}
```
"#,
            id = id,
            title = idea.title,
            idea_type = idea.idea_type,
            created = chrono::Local::now().to_rfc3339(),
            description = idea.description,
            plan = idea.implementation_plan.as_deref().unwrap_or("N/A"),
            code = idea.code_snippet.as_deref().unwrap_or("// No code provided")
        );

        let file_path = proposals_dir.join(filename);
        std::fs::write(&file_path, content)?;

        Ok(file_path)
    }
}
