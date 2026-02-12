//! OpenSpore Skills System
//!
//! Hybrid architecture:
//! 1. Core skills (hardcoded in Rust) - fast, reliable
//! 2. Plugin skills (JS/shell in ~/.openspore/skills/) - dynamic, extensible

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::process::Command;
use tracing::info;
use openspore_core::config::AppConfig;

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
pub mod browser;
pub mod utils;

use crate::browser::launcher::BrowserType;

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

/// Plugin skill - loads single-file JS/shell scripts from ~/.openspore/skills/
pub struct PluginSkill {
    pub name: String,
    pub description: String,
    pub script_path: PathBuf,
}

#[async_trait]
impl Skill for PluginSkill {
    fn name(&self) -> &'static str {
        // Leak the string to get a static lifetime
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
        let script_path_str = self.script_path.to_string_lossy().to_string();
        let cmd_args = crate::utils::split_arguments(&sanitized_args);

        match ext {
            "js" => {
                let mut full_args = vec![script_path_str];
                full_args.extend(cmd_args);
                execute_process("node", &full_args).await
            },
            "sh" => {
                let mut full_args = vec![script_path_str];
                full_args.extend(cmd_args);
                execute_process("sh", &full_args).await
            },
            "py" => {
                let mut full_args = vec![script_path_str];
                full_args.extend(cmd_args);
                execute_process("python3", &full_args).await
            },
            _ => execute_process(&script_path_str, &cmd_args).await,
        }
    }
}

/// AgentSkill - loads OpenClaw/AgentSkills compatible folders (with SKILL.md)
pub struct AgentSkill {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub script_path: Option<PathBuf>,
}

#[async_trait]
impl Skill for AgentSkill {
    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        // Combine description and short instructions for the system prompt
        let combined = format!("{} (AgentSkill)\nInstructions:\n{}", self.description, self.instructions);
        Box::leak(combined.into_boxed_str())
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        if let Some(ref script_path) = self.script_path {
            let ext = script_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let sanitized_args = crate::utils::sanitize_path(args);
            let path_str = script_path.to_string_lossy().to_string();
            let cmd_args = crate::utils::split_arguments(&sanitized_args);

            match ext {
                "js" => {
                    let mut full_args = vec![path_str];
                    full_args.extend(cmd_args);
                    execute_process("node", &full_args).await
                },
                "sh" => {
                    let mut full_args = vec![path_str];
                    full_args.extend(cmd_args);
                    execute_process("sh", &full_args).await
                },
                "py" => {
                    let mut full_args = vec![path_str];
                    full_args.extend(cmd_args);
                    execute_process("python3", &full_args).await
                },
                _ => execute_process(&path_str, &cmd_args).await,
            }
        } else {
            let res = serde_json::json!({
                "success": true,
                "stdout": format!("Skill '{}' is instruction-based. Please execute the following logic using base tools:\n\n{}", self.name, self.instructions),
                "stderr": ""
            });
            Ok(res.to_string())
        }
    }
}

/// Helper to execute a process and return JSON result
async fn execute_process(program: &str, args: &[String]) -> Result<String, String> {
    let root = openspore_core::path_utils::get_app_root();

    // Ensure we have a decent PATH (same as ExecSkill)
    let engine_bin = root.join("crates/target/release");
    let path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", engine_bin.to_string_lossy(), path);

    let output_res = Command::new(program)
        .args(args)
        .envs(std::env::vars())
        .env("PATH", new_path)
        .current_dir(&root)
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

/// Skill Registry - loads core skills + plugin skills
pub struct SkillLoader {
    skills: HashMap<String, Box<dyn Skill>>,
    plugin_dir: PathBuf,
}

impl SkillLoader {
    pub fn new(config: AppConfig) -> Self {
        let mut skills: HashMap<String, Box<dyn Skill>> = HashMap::new();

        let preferred_browser = config.browser_type.as_deref()
            .and_then(|s| {
                tracing::info!("AppConfig has browser_type: {:?}", s);
                BrowserType::from_str(s).ok()
            });

        if preferred_browser.is_none() && config.browser_type.is_some() {
            tracing::error!("Failed to parse browser type: {:?}", config.browser_type);
        }

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
            Box::new(browser::BrowserSkill::new(preferred_browser)),
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

            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if !["js", "sh", "py"].contains(&ext) {
                    continue;
                }

                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

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
            } else if path.is_dir() {
                // AgentSkill (Directory Compatibility)
                let skill_md_path = path.join("SKILL.md");
                if skill_md_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&skill_md_path) {
                        let (metadata, instructions) = parse_skill_md(&content);

                        let name = metadata.get("name")
                            .cloned()
                            .unwrap_or_else(|| path.file_name().and_then(|s| s.to_str()).unwrap_or("unknown").to_string());

                        let description = metadata.get("description")
                            .cloned()
                            .unwrap_or_else(|| format!("AgentSkill: {}", name));

                        // Find entrypoint script
                        let script_path = find_skill_script(&path, &name);

                        info!("📂 Loaded AgentSkill: {} from {:?}", name, path);
                        if let Some(ref sp) = script_path {
                            info!("   -> Entrypoint: {:?}", sp);
                        }

                        let skill = AgentSkill {
                            name: name.clone(),
                            description,
                            instructions,
                            script_path,
                        };

                        self.skills.insert(name.to_lowercase(), Box::new(skill));
                    }
                }
            }
        }
    }
}

/// Simple YAML frontmatter parser for SKILL.md
fn parse_skill_md(content: &str) -> (HashMap<String, String>, String) {
    let mut metadata = HashMap::new();
    let instructions;

    if content.starts_with("---") {
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() >= 3 {
            let frontmatter = parts[1];
            instructions = parts[2].trim().to_string();

            for line in frontmatter.lines() {
                if let Some(pos) = line.find(':') {
                    let key = line[..pos].trim().to_lowercase();
                    let value = line[pos+1..].trim().trim_matches('"').trim_matches('\'').to_string();
                    metadata.insert(key, value);
                }
            }
        } else {
            // No valid frontmatter block, treat entire content as instructions
            instructions = content.to_string();
        }
    } else {
        // No frontmatter, treat entire content as instructions
        instructions = content.to_string();
    }

    (metadata, instructions)
}

/// Find a script within an AgentSkill folder
fn find_skill_script(dir: &std::path::Path, skill_name: &str) -> Option<PathBuf> {
    let extensions = ["js", "sh", "py"];
    // Prioritize skill_name.ext, then index.ext, main.ext, run.ext
    let bases = [skill_name, "index", "main", "run", "handler"];

    // Check root of skill folder
    for base in &bases {
        for ext in &extensions {
            let path = dir.join(format!("{}.{}", base, ext));
            if path.exists() {
                return Some(path);
            }
        }
    }

    // Also check scripts/ subdirectory
    let scripts_dir = dir.join("scripts");
    if scripts_dir.exists() && scripts_dir.is_dir() {
        for base in &bases {
            for ext in &extensions {
                let path = scripts_dir.join(format!("{}.{}", base, ext));
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    None
}

impl SkillLoader {
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
                                        "web_fetch", "search", "delegate", "telegram_send", "diff_patch", "cron_manager", "submit_skill", "browser"]
            .iter().map(|s| s.to_string()).collect();

        self.skills.retain(|name, _| core_names.contains(name));
        self.load_plugins();
    }
}
