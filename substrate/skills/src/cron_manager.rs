//! Cron Manager Skill (Core)
//! Port of cron_manager.js

use super::Skill;
use async_trait::async_trait;
use tokio::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::process::Command;

pub struct CronManagerSkill;

#[derive(Debug, Serialize, Deserialize)]
struct CronParams {
    action: String,
    name: Option<String>,
    schedule: Option<String>,
    script_content: Option<String>,
    description: Option<String>,
}

#[async_trait]
impl Skill for CronManagerSkill {
    fn name(&self) -> &'static str { "cron_manager" }

    fn description(&self) -> &'static str {
        "Manage OpenSpore automation jobs. Actions: list, add, remove. Usage: [CRON_MANAGER: {\"action\": \"list\"}]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let mut sanitized = args.trim();
        if sanitized.starts_with('"') && sanitized.ends_with('"') && sanitized.contains('{') {
            sanitized = &sanitized[1..sanitized.len()-1];
        }

        let params: CronParams = serde_json::from_str(sanitized)
            .map_err(|e| format!("Invalid JSON: {}. Error: {}", sanitized, e))?;

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let cron_dir = PathBuf::from(format!("{}/.openspore/workspace/cron", home));
        let manifest_path = cron_dir.join("crontab.json");

        if !cron_dir.exists() {
            fs::create_dir_all(&cron_dir).await.ok();
        }

        let mut manifest: Value = if manifest_path.exists() {
            let content = fs::read_to_string(&manifest_path).await.map_err(|e| e.to_string())?;
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        match params.action.as_str() {
            "list" => {
                Ok(serde_json::to_string_pretty(&manifest).unwrap_or_default())
            },
            "add" => {
                let name = params.name.ok_or("Error: 'name' is required for 'add'")?;
                let schedule = params.schedule.ok_or("Error: 'schedule' is required for 'add'")?;
                let content = params.script_content.ok_or("Error: 'script_content' is required for 'add'")?;

                let script_name = if name.ends_with(".js") { name.clone() } else { format!("{}.js", name) };
                let script_path = cron_dir.join(&script_name);

                // 1. Write script
                fs::write(&script_path, content).await.map_err(|e| e.to_string())?;

                // 2. Update manifest
                let job = serde_json::json!({
                    "schedule": schedule,
                    "script": script_name,
                    "description": params.description.unwrap_or_else(|| "Added via Brain".to_string())
                });
                manifest[name.clone()] = job;

                fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap())
                    .await.map_err(|e| e.to_string())?;

                // 3. Sync crontab
                let _ = Command::new("openspore").arg("cron").arg("install").output().await;

                Ok(format!("✅ Job '{}' added and crontab synced.", name))
            },
            "remove" => {
                let name = params.name.ok_or("Error: 'name' is required for 'remove'")?;
                if manifest.get(&name).is_none() {
                    return Err(format!("Error: Job '{}' not found.", name));
                }

                if let Some(job) = manifest.get(&name) {
                    if let Some(script) = job.get("script").and_then(|s| s.as_str()) {
                        let target = cron_dir.join(script);
                        let _ = fs::remove_file(target).await;
                    }
                }

                manifest.as_object_mut().unwrap().remove(&name);
                fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap())
                    .await.map_err(|e| e.to_string())?;

                // 3. Sync crontab
                let _ = Command::new("openspore").arg("cron").arg("install").output().await;

                Ok(format!("✅ Job '{}' removed and crontab synced.", name))
            },
            _ => Err(format!("Unknown action: {}", params.action))
        }
    }
}
