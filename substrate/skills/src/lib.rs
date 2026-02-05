//! OpenSpore Skills System
//!
//! Hybrid architecture:
//! 1. Core skills (hardcoded in Rust) - fast, reliable
//! 2. Plugin skills (JS/shell in ~/.openspore/skills/) - dynamic, extensible

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{info, warn};

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

        let output = match ext {
            "js" => {
                Command::new("node")
                    .arg(&self.script_path)
                    .arg(args)
                    .output()
                    .await
            }
            "sh" => {
                Command::new("sh")
                    .arg(&self.script_path)
                    .arg(args)
                    .output()
                    .await
            }
            "py" => {
                Command::new("python3")
                    .arg(&self.script_path)
                    .arg(args)
                    .output()
                    .await
            }
            _ => {
                // Try to execute directly (for binaries)
                Command::new(&self.script_path)
                    .arg(args)
                    .output()
                    .await
            }
        }.map_err(|e| e.to_string())?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(stdout.to_string())
        } else {
            Err(format!("{}\n{}", stdout, stderr))
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
        ];

        for skill in core_skills {
            skills.insert(skill.name().to_lowercase(), skill);
        }

        // Determine plugin directory
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let plugin_dir = PathBuf::from(format!("{}/.openspore/skills", home));

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

            // Try to read description from first line comment
            let description = std::fs::read_to_string(&path)
                .ok()
                .and_then(|content| {
                    content.lines().next().and_then(|line| {
                        if line.starts_with("//") || line.starts_with("#") {
                            Some(line.trim_start_matches(['/', '#', ' ']).to_string())
                        } else {
                            None
                        }
                    })
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

    /// Generate system prompt listing all available skills
    pub fn get_system_prompt(&self) -> String {
        let mut prompt = String::from("Available Skills:\n");

        for skill in self.skills.values() {
            prompt.push_str(&format!("- [{}]: {}\n", skill.name().to_uppercase(), skill.description()));
        }

        prompt
    }

    /// Reload plugin skills (hot reload)
    pub fn reload_plugins(&mut self) {
        // Remove existing plugins (keep core skills)
        let core_names: Vec<String> = ["exec", "read_file", "write_file", "edit_file", "list_dir", "purge",
                                        "web_fetch", "search", "delegate", "telegram_send"]
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
