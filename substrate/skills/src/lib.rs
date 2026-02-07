//! OpenSpore Skills System
//!
//! Hybrid architecture:
//! 1. Core skills (hardcoded in Rust) - fast, reliable
//! 2. Plugin skills (JS/shell in ~/.openspore/skills/) - dynamic, extensible

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::info;

// Core skill modules (hardcoded in Rust)
pub mod exec;
pub mod read_file;
pub mod write_file;
pub mod edit_file;
pub mod list_dir;
pub mod purge;
pub mod web_fetch;
pub mod search;
pub mod delegate;
pub mod telegram_send;
pub mod diff_patch;
pub mod cron_manager;
pub mod submit_skill;
pub mod utils;

/// Skill trait - all skills implement this interface
#[async_trait]
pub trait Skill: Send + Sync {
    /// Name of the skill (used in [SKILL_NAME: args] syntax)
    fn name(&self) -> &'static str;

    /// Description for the LLM system prompt
    fn description(&self) -> &'static str;

    /// Execute the skill with given arguments
    async fn execute(&self, args: &str) -> Result<String, String>;
}

/// Plugin skill - loads external JS/shell scripts from ~/.openspore/skills/
pub struct PluginSkill {
    pub name: String,
    pub description: String,
    pub script_path: PathBuf,
}

#[async_trait]
impl Skill for PluginSkill {
    fn name(&self) -> &'static str {
        // Leak the string to get a static lifetime (safe for long-running process)
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        Box::leak(self.description.clone().into_boxed_str())
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let ext = self.script_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let sanitized_args = crate::utils::sanitize_path(args);
        let script_path = self.script_path.to_string_lossy();

        let full_command = match ext {
            "js" => format!("node \"{}\" {}", script_path, sanitized_args),
            "sh" => format!("sh \"{}\" {}", script_path, sanitized_args),
            "py" => format!("python3 \"{}\" {}", script_path, sanitized_args),
            _ => format!("\"{}\" {}", script_path, sanitized_args),
        };

        let output_res = Command::new("sh")
            .arg("-c")
            .arg(&full_command)
            .output()
            .await;

        match output_res {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                let success = output.status.success();

                let res = serde_json::json!({
                    "success": success,
                    "exit_code": exit_code,
                    "stdout": stdout,
                    "stderr": stderr
                });
                Ok(res.to_string())
            },
            Err(e) => {
                let res = serde_json::json!({
                    "success": false,
                    "error": format!("Execution failed: {}", e)
                });
                Ok(res.to_string())
            }
        }
    }
}

/// Skill Registry - loads core skills + plugin skills
pub struct SkillLoader {
    skills: HashMap<String, Box<dyn Skill>>,
    plugin_dir: PathBuf,
}

impl SkillLoader {
    pub fn new() -> Self {
        let mut skills: HashMap<String, Box<dyn Skill>> = HashMap::new();

        // Register core skills (hardcoded in Rust)
        let core_skills: Vec<Box<dyn Skill>> = vec![
            Box::new(exec::ExecSkill),
            Box::new(read_file::ReadFileSkill),
            Box::new(write_file::WriteFileSkill),
            Box::new(edit_file::EditFileSkill),
            Box::new(list_dir::ListDirSkill),
            Box::new(purge::PurgeSkill),
            Box::new(web_fetch::WebFetchSkill),
            Box::new(search::SearchSkill),
            Box::new(delegate::DelegateSkill),
            Box::new(telegram_send::TelegramSendSkill),
            Box::new(diff_patch::DiffPatchSkill),
            Box::new(cron_manager::CronManagerSkill),
            Box::new(submit_skill::SubmitSkill),
        ];

        for skill in core_skills {
            skills.insert(skill.name().to_lowercase(), skill);
        }

        // Determine plugin directory
        let root = openspore_core::path_utils::get_app_root();
        let plugin_dir = root.join("skills");

        let mut loader = Self { skills, plugin_dir };
        loader.load_plugins();
        loader
    }

    /// Load plugin skills from ~/.openspore/skills/
    fn load_plugins(&mut self) {
        if !self.plugin_dir.exists() {
            return;
        }

        let entries = match std::fs::read_dir(&self.plugin_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !["js", "sh", "py"].contains(&ext) {
                continue;
            }

            // Skill name from filename (e.g., "my_skill.js" -> "my_skill")
            let name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Try to read description from first non-shebang comment
            let description = std::fs::read_to_string(&path)
                .ok()
                .and_then(|content| {
                    content.lines()
                        .find(|line| {
                            (line.starts_with("//") || line.starts_with("#")) && !line.starts_with("#!")
                        })
                        .map(|line| line.trim_start_matches(['/', '#', ' ']).to_string())
                })
                .unwrap_or_else(|| format!("Plugin skill: {}", name));

            info!("🔌 Loaded plugin skill: {} from {:?}", name, path);

            let plugin = PluginSkill {
                name: name.clone(),
                description,
                script_path: path,
            };

            self.skills.insert(name.to_lowercase(), Box::new(plugin));
        }
    }

    /// Get a skill by name (case-insensitive)
    pub fn get(&self, name: &str) -> Option<&dyn Skill> {
        self.skills.get(&name.to_lowercase()).map(|s| s.as_ref())
    }

    /// Generate system prompt listing available skills, optionally validating against an exclusion list.
    pub fn get_system_prompt(&self, excluded_skills: &[&str]) -> String {
        let mut prompt = String::from("Available Skills:\n");

        for skill in self.skills.values() {
            if !excluded_skills.contains(&skill.name()) {
                prompt.push_str(&format!("- [{}]: {}\n", skill.name().to_uppercase(), skill.description()));
            }
        }

        prompt
    }

    /// Reload plugin skills (hot reload)
    pub fn reload_plugins(&mut self) {
        // Remove existing plugins (keep core skills)
        let core_names: Vec<String> = ["exec", "read_file", "write_file", "edit_file", "list_dir", "purge",
                                        "web_fetch", "search", "delegate", "telegram_send", "diff_patch", "cron_manager", "submit_skill"]
            .iter().map(|s| s.to_string()).collect();

        self.skills.retain(|name, _| core_names.contains(name));
        self.load_plugins();
    }
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}
