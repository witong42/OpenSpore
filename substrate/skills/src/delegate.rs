//! Delegate Skill (Core) - Spawn sub-spores

use super::Skill;
use async_trait::async_trait;

pub struct DelegateSkill;

#[async_trait]
impl Skill for DelegateSkill {
    fn name(&self) -> &'static str { "delegate" }

    fn description(&self) -> &'static str {
        "Spawn a specialized sub-spore for parallel task execution. Usage: [DELEGATE: \"task description\" --role=\"ExpertRole\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let parts: Vec<&str> = args.splitn(2, "--role=").collect();
        let task = parts[0].trim().trim_matches('"').trim_matches('\'').trim();
        let role = parts.get(1).map(|r| r.trim().trim_matches('"').trim_matches('\'')).unwrap_or("GeneralExpert");

        let swarm = openspore_swarm::SwarmManager::new();

        match swarm.spawn(task, role).await {
            Ok(result) => {
                Ok(format!(
                    "\n--- 🤖 Agent Delegate (Role: {}) ---\n{}\n-----------------------------------\n",
                    role, result
                ))
            }
            Err(e) => {
                Err(format!("Delegation Failed: {}", e))
            }
        }
    }
}
