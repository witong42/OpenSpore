//! Cron Manager Skill (Core)
//! Port of cron_manager.js

use super::Skill;
use async_trait::async_trait;
use tokio::fs;
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
        "Manage OpenSpore automation jobs. Actions: list, add, remove. Returns JSON with success and results. Usage: [CRON_MANAGER: {\"action\": \"list\"}]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let mut sanitized = args.trim();
        if sanitized.starts_with('"') && sanitized.ends_with('"') && sanitized.contains('{') {
            sanitized = &sanitized[1..sanitized.len()-1];
        }

        let params: CronParams = match serde_json::from_str(sanitized) {
            Ok(p) => p,
            Err(e) => {
                let res = serde_json::json!({ "success": false, "error": format!("Invalid JSON: {}. Error: {}", sanitized, e) });
                return Ok(res.to_string());
            }
        };

        let root = openspore_core::path_utils::get_app_root();
        let cron_dir = root.join("workspace/cron");
        let manifest_path = cron_dir.join("crontab.json");

        if !cron_dir.exists() {
            fs::create_dir_all(&cron_dir).await.ok();
        }

        let mut manifest: Value = if manifest_path.exists() {
            match fs::read_to_string(&manifest_path).await {
                Ok(content) => serde_json::from_str(&content).unwrap_or(serde_json::json!({})),
                Err(_) => serde_json::json!({})
            }
        } else {
            serde_json::json!({})
        };

        match params.action.as_str() {
            "list" => {
                let res = serde_json::json!({
                    "success": true,
                    "action": "list",
                    "jobs": manifest
                });
                Ok(res.to_string())
            },
            "add" => {
                let name = match params.name {
                    Some(n) => n,
                    None => {
                        let res = serde_json::json!({ "success": false, "error": "'name' is required for 'add'" });
                        return Ok(res.to_string());
                    }
                };
                let schedule = match params.schedule {
                    Some(s) => s,
                    None => {
                        let res = serde_json::json!({ "success": false, "error": "'schedule' is required for 'add'" });
                        return Ok(res.to_string());
                    }
                };
                let content = match params.script_content {
                    Some(c) => c,
                    None => {
                        let res = serde_json::json!({ "success": false, "error": "'script_content' is required for 'add'" });
                        return Ok(res.to_string());
                    }
                };

                let script_name = if name.ends_with(".js") { name.clone() } else { format!("{}.js", name) };
                let script_path = cron_dir.join(&script_name);

                if let Err(e) = fs::write(&script_path, content).await {
                    let res = serde_json::json!({ "success": false, "error": format!("Failed to write script: {}", e) });
                    return Ok(res.to_string());
                }

                let job = serde_json::json!({
                    "schedule": schedule,
                    "script": script_name,
                    "description": params.description.unwrap_or_else(|| "Added via Brain".to_string())
                });
                manifest[name.clone()] = job;

                if let Err(e) = fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).await {
                    let res = serde_json::json!({ "success": false, "error": format!("Failed to update manifest: {}", e) });
                    return Ok(res.to_string());
                }

                let _ = Command::new("openspore").arg("cron").arg("install").output().await;

                let res = serde_json::json!({
                    "success": true,
                    "action": "add",
                    "message": format!("Job '{}' added and crontab synced.", name)
                });
                Ok(res.to_string())
            },
            "remove" => {
                let name = match params.name {
                    Some(n) => n,
                    None => {
                        let res = serde_json::json!({ "success": false, "error": "'name' is required for 'remove'" });
                        return Ok(res.to_string());
                    }
                };
                if manifest.get(&name).is_none() {
                    let res = serde_json::json!({ "success": false, "error": format!("Job '{}' not found.", name) });
                    return Ok(res.to_string());
                }

                if let Some(job) = manifest.get(&name) {
                    if let Some(script) = job.get("script").and_then(|s| s.as_str()) {
                        let target = cron_dir.join(script);
                        let _ = fs::remove_file(target).await;
                    }
                }

                manifest.as_object_mut().unwrap().remove(&name);
                if let Err(e) = fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).await {
                    let res = serde_json::json!({ "success": false, "error": format!("Failed to update manifest: {}", e) });
                    return Ok(res.to_string());
                }

                let _ = Command::new("openspore").arg("cron").arg("install").output().await;

                let res = serde_json::json!({
                    "success": true,
                    "action": "remove",
                    "message": format!("Job '{}' removed and crontab synced.", name)
                });
                Ok(res.to_string())
            },
            _ => {
                let res = serde_json::json!({ "success": false, "error": format!("Unknown action: {}", params.action) });
                Ok(res.to_string())
            }
        }
    }
}
