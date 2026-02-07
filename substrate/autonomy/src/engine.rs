use openspore_brain::Brain;
use openspore_memory::MemorySystem;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, error, warn};
use anyhow::Result;
use chrono;

#[derive(Debug, Serialize, Deserialize)]
pub struct Idea {
    pub title: String,
    pub description: String,
    #[serde(rename = "type")]
    pub idea_type: String,
    pub implementation_plan: serde_json::Value,
}

impl Idea {
    pub fn format_plan(&self) -> String {
        match &self.implementation_plan {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(arr) => {
                arr.iter()
                    .map(|v| v.as_str().unwrap_or("").to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            _ => self.implementation_plan.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AutonomyState {
    pub last_processed_log: chrono::DateTime<chrono::Local>,
    pub run_count: usize,
}

impl Default for AutonomyState {
    fn default() -> Self {
        Self {
            last_processed_log: chrono::Local::now() - chrono::Duration::hours(2),
            run_count: 0,
        }
    }
}

pub struct AutonomyEngine;

impl AutonomyEngine {
    pub async fn run(brain: &Brain, memory: &MemorySystem) -> Result<Option<PathBuf>> {
        info!("💡 Autonomy: Scanning for improvement opportunities (Tree of Thoughts mode)...");

        // 1. Load state
        let state_path = memory.project_root.join("workspace/autonomy/state.json");
        let mut state: AutonomyState = if state_path.exists() {
            match std::fs::read_to_string(&state_path) {
                Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
                Err(_) => AutonomyState::default(),
            }
        } else {
            AutonomyState::default()
        };

        // 2. Get recent context (Since last run, max 2 hours)
        let two_hours_ago = chrono::Local::now() - chrono::Duration::hours(2);
        let cutoff = if state.last_processed_log < two_hours_ago {
             two_hours_ago
        } else {
             state.last_processed_log
        };

        info!("📅 Fetching logs since: {}", cutoff.format("%Y-%m-%d %H:%M:%S"));

        let recent_logs = memory.get_logs_since(cutoff).unwrap_or_else(|e| {
            warn!("Failed to retrieve recent logs: {}. Falling back to recent memories.", e);
            memory.get_memories("context").iter().rev().take(5)
                .map(|m| m.content.clone()).collect::<Vec<_>>().join("\n\n")
        });

        if recent_logs.trim().is_empty() {
            info!("💤 No new logs since last scan. Skipping autonomy cycle.");
            return Ok(None);
        }

        // --- PHASE 1: BRAINSTORM (Branching) ---
        info!("🌳 ToT [Phase 1]: Brainstorming multiple reasoning paths...");
        let brainstorm_prompt = format!(r#"
You are the 'Explorer Spore'. Your job is to analyze the agent's recent logs and propose THREE (3) DIFFERENT reasoning paths (strategies) for system improvement.
Context: You are running in the project root: {root}

Unlike a single-threaded plan, you must explore different perspectives:
Path A: Conservative/Safe refactoring.
Path B: Feature expansion/New capability.
Path C: Optimization/Performance refinement.

Directive: Avoid over-engineering. If a problem can be fixed with 10 lines of code, do not propose a 4-task research-and-audit project.
SYSTEM DIRECTIVE: You are in a reasoning phase. DO NOT USE ANY TOOLS. DO NOT add any conversational text. Respond ONLY with the JSON object below.

<RECENT_LOGS>
{recent_logs}
</RECENT_LOGS>

Output JSON ONLY:
{{
    "title": "General Theme of Improvement",
    "paths": [
        {{
            "id": 1,
            "strategy": "Path A strategy",
            "reasoning": "...",
            "potential_risks": "..."
        }},
        {{
            "id": 2,
            "strategy": "Path B strategy",
            "reasoning": "...",
            "potential_risks": "..."
        }},
        {{
            "id": 3,
            "strategy": "Path C strategy",
            "reasoning": "...",
            "potential_risks": "..."
        }}
    ]
}}
"# , root = memory.project_root.display(), recent_logs = recent_logs);

        let brainstorm_raw = brain.think(&brainstorm_prompt).await;
        let brainstorm_json: serde_json::Value = match serde_json::from_str(Self::clean_json(&brainstorm_raw)) {
            Ok(j) => j,
            Err(e) => {
                error!("ToT Error [Phase 1]: Failed to parse brainstorm JSON: {}. Raw: {}", e, brainstorm_raw);
                return Ok(None);
            }
        };

        if brainstorm_json["paths"].as_array().map_or(true, |a| a.is_empty()) {
            error!("ToT Error [Phase 1]: No reasoning paths generated.");
            return Ok(None);
        }

        // --- PHASE 2: EVALUATE (Pruning/Selection) ---
        info!("⚖️ ToT [Phase 2]: Evaluating paths and selecting winner...");
        let referee_prompt = format!(r#"
You are the 'Referee Spore'. Evaluate these 3 proposed reasoning paths.
Select the WINNING path based on Safety, Value, and Feasibility.
SYSTEM DIRECTIVE: DO NOT USE TOOLS. Respond ONLY with the JSON object.

<PROPOSED_PATHS>
{paths}
</PROPOSED_PATHS>

Compare the paths, find the winner, and output JSON ONLY:
{{
    "winning_path_id": 1,
    "selection_reason": "...",
    "suggested_refinement": "Any minor adjustments..."
}}
"#, paths = brainstorm_json["paths"]);

        let referee_raw = brain.think(&referee_prompt).await;
        let referee_json: serde_json::Value = match serde_json::from_str(Self::clean_json(&referee_raw)) {
            Ok(j) => j,
            Err(e) => {
                error!("ToT Error [Phase 2]: Failed to parse referee JSON: {}. Raw: {}", e, referee_raw);
                return Ok(None);
            }
        };

        let winner_id = referee_json["winning_path_id"].as_u64().unwrap_or(1) as usize;
        let paths = brainstorm_json["paths"].as_array().unwrap();
        let winning_path = paths.iter().find(|p| p["id"].as_u64().unwrap_or(0) as usize == winner_id)
            .unwrap_or(&paths[0]);

        info!("🏆 ToT: Winner selected - Path {} ({})", winner_id, winning_path["strategy"]);

        // --- PHASE 3: FINALIZE (Action Planning) ---
        info!("🗺️ ToT [Phase 3]: Generating Action Plan...");
        let planner_prompt = format!(r#"
You are the 'Planner Spore'. Take the WINNING STRATEGY and generate a single, concrete 'Action Plan'.
Directive: Be direct and technical. No multi-agent or hierarchical overhead.
SYSTEM DIRECTIVE: DO NOT USE TOOLS. Respond ONLY with the JSON object. Do not explain your choice.

Winner: {strategy}
Refinement: {refinement}

Output JSON ONLY:
{{
    "title": "{title}",
    "description": "{description}",
    "type": "feature",
    "implementation_plan": "Step-by-step instructions for executing this change."
}}
"#,
            strategy = winning_path["strategy"],
            refinement = referee_json["suggested_refinement"],
            title = brainstorm_json["title"],
            description = winning_path["reasoning"]
        );

        let planner_raw = brain.think(&planner_prompt).await;
        let idea: Idea = match serde_json::from_str(Self::clean_json(&planner_raw)) {
            Ok(i) => i,
            Err(e) => {
                error!("ToT Error [Phase 3]: Failed to parse planner JSON: {}. Raw: {}", e, planner_raw);
                return Ok(None);
            }
        };

        // --- PHASE 4: FINAL CONSENSUS (Audit) ---
        info!("🔍 ToT [Phase 4]: Final Audit (Consensus Building)...");
        let review_prompt = format!(r#"
You are the 'Reviewer Spore'. Audit this proposed Action Plan.
Plan Title: {title}
Implementation: {plan}

Output JSON ONLY:
{{
    "status": "GREEN_LIGHT" | "REJECT",
    "reason": "..."
}}
"#,
            title = idea.title,
            plan = idea.format_plan()
        );

        let review_raw = brain.think(&review_prompt).await;
        let review_json: serde_json::Value = match serde_json::from_str(Self::clean_json(&review_raw)) {
            Ok(j) => j,
            Err(e) => {
                error!("ToT Error [Phase 4]: Failed to parse reviewer JSON: {}. Raw: {}", e, review_raw);
                return Ok(None);
            }
        };

        if review_json["status"] == "GREEN_LIGHT" {
            info!("✅ ToT: Consensus reached. Proposal finalized.");
        } else {
            warn!("⚠️ ToT Rejected: {}", review_json["reason"]);
            return Ok(None);
        }

        // 3. Update state
        state.run_count += 1;
        state.last_processed_log = chrono::Local::now();
        if let Ok(s) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&state_path, s);
        }

        // 4. Create Proposal
        let proposal_path = Self::create_proposal(&idea, memory)?;
        info!("✨ NEW PROPOSAL GENERATED (via ToT): {}", proposal_path.display());

        Ok(Some(proposal_path))
    }

    fn clean_json(raw: &str) -> &str {
        let first_brace = raw.find('{');
        let last_brace = raw.rfind('}');

        if let (Some(start), Some(end)) = (first_brace, last_brace) {
            if end > start {
                return &raw[start..=end];
            }
        }
        "{}" // Return empty object if no braces found to prevent parsing non-JSON text
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

## Action Plan
{plan}
"#,
            id = id,
            title = idea.title,
            idea_type = idea.idea_type,
            created = chrono::Local::now().to_rfc3339(),
            description = idea.description,
            plan = idea.format_plan()
        );

        let file_path = proposals_dir.join(filename);
        std::fs::write(&file_path, content)?;

        Ok(file_path)
    }
}
