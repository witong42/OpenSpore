//! SPORE DOCTOR
//! The central diagnostic and repair utility for OpenSpore.
//! Consolidates integrity checks, permission fixes, and substrate recovery.
//! Exact port of opensporejs/src/utils/doctor.js

use std::path::PathBuf;
use std::process::Command;

/// Issue severity levels
#[derive(Debug, Clone)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

/// A detected issue
#[derive(Debug, Clone)]
pub struct Issue {
    pub id: String,
    pub label: String,
    pub severity: Severity,
    pub meta: Option<String>,
}

/// Colored terminal output
fn log(msg: &str, color: &str) {
    let code = match color {
        "red" => "\x1b[31m",
        "green" => "\x1b[32m",
        "yellow" => "\x1b[33m",
        "blue" => "\x1b[34m",
        "magenta" => "\x1b[35m",
        "cyan" => "\x1b[36m",
        "gray" => "\x1b[90m",
        _ => "\x1b[37m",
    };
    println!("{}{}\x1b[0m", code, msg);
}

/// SporeDoctor - diagnostic and repair utility
pub struct SporeDoctor {
    root: PathBuf,
    workspace: PathBuf,
    issues: Vec<Issue>,
}

impl SporeDoctor {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let root = PathBuf::from(format!("{}/.openspore", home));
        let workspace = root.join("workspace");

        Self {
            root,
            workspace,
            issues: Vec::new(),
        }
    }

    /// Run all diagnostics
    pub fn check_all(&mut self) -> bool {
        log("\n🩺 --- OpenSpore System Diagnostic ---", "blue");

        self.check_env();
        self.check_structure();
        self.check_permissions();
        self.check_cron();
        self.check_substrate();

        if self.issues.is_empty() {
            log("\n✅ SYSTEM OPTIMAL: No issues detected.", "green");
            true
        } else {
            log(&format!("\n🔴 Found {} issues. Running prescriptions...\n", self.issues.len()), "red");
            self.prescribe();
            false
        }
    }

    // --- DIAGNOSTICS ---

    fn check_env(&mut self) {
        let env_path = self.root.join(".env");

        if !env_path.exists() {
            self.issues.push(Issue {
                id: "MISSING_ENV".to_string(),
                label: ".env file not found".to_string(),
                severity: Severity::Critical,
                meta: None,
            });
            return;
        }

        if let Ok(content) = std::fs::read_to_string(&env_path) {
            if !content.contains("OPENROUTER_API_KEY") || content.contains("YOUR_KEY_HERE") {
                self.issues.push(Issue {
                    id: "NO_API_KEY".to_string(),
                    label: "OpenRouter API Key is missing or default".to_string(),
                    severity: Severity::Critical,
                    meta: None,
                });
            }
            if !content.contains("TELEGRAM_BOT_TOKEN") {
                self.issues.push(Issue {
                    id: "NO_TELEGRAM_BOT".to_string(),
                    label: "Telegram Bot Token not configured".to_string(),
                    severity: Severity::Warning,
                    meta: None,
                });
            }
        }

        log("✅ .env file found", "green");
    }

    fn check_structure(&mut self) {
        // Required directories
        let dirs = [
            "workspace/identity",
            "workspace/context",
            "workspace/knowledge",
            "workspace/memory",
            "workspace/preferences",
            "workspace/autonomy",
            "workspace/autonomy/state",
            "workspace/cron",
            "skills",
        ];

        for d in dirs {
            let path = self.root.join(d);
            if path.exists() {
                log(&format!("✅ {} exists", d), "green");
            } else {
                self.issues.push(Issue {
                    id: "MISSING_DIR".to_string(),
                    label: format!("Directory missing: {}", d),
                    severity: Severity::Warning,
                    meta: Some(d.to_string()),
                });
            }
        }

        // Critical files
        let soul_files = [
            "workspace/identity/SOUL.md",
            "workspace/identity/USER.md",
            "workspace/context/LOGS.md",
            "workspace/identity/AGENTS.md",
        ];

        for f in soul_files {
            let path = self.root.join(f);
            if path.exists() {
                log(&format!("✅ {} exists", f), "green");
            } else {
                self.issues.push(Issue {
                    id: "MISSING_SOUL_FILE".to_string(),
                    label: format!("Critical file missing: {}", f),
                    severity: Severity::Warning,
                    meta: Some(f.to_string()),
                });
            }
        }
    }

    fn check_permissions(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let binary = self.root.join("substrate/target/release/openspore");
            if binary.exists() {
                if let Ok(meta) = std::fs::metadata(&binary) {
                    let mode = meta.permissions().mode();
                    if mode & 0o111 != 0 {
                        log("✅ Binary is executable", "green");
                    } else {
                        self.issues.push(Issue {
                            id: "PERM_ERR".to_string(),
                            label: "Binary not executable".to_string(),
                            severity: Severity::Warning,
                            meta: Some(binary.to_string_lossy().to_string()),
                        });
                    }
                }
            }
        }
    }

    fn check_cron(&mut self) {
        let output = Command::new("crontab")
            .arg("-l")
            .output();

        match output {
            Ok(out) => {
                let content = String::from_utf8_lossy(&out.stdout);
                if content.contains("openspore") {
                    log("✅ OpenSpore jobs found in crontab", "green");
                } else {
                    self.issues.push(Issue {
                        id: "CRON_NOT_INSTALLED".to_string(),
                        label: "OpenSpore jobs not found in crontab".to_string(),
                        severity: Severity::Info,
                        meta: None,
                    });
                }
            }
            Err(_) => {
                self.issues.push(Issue {
                    id: "CRON_NOT_INSTALLED".to_string(),
                    label: "System crontab appears empty/inactive".to_string(),
                    severity: Severity::Info,
                    meta: None,
                });
            }
        }
    }

    fn check_substrate(&mut self) {
        // Check if Rust binary exists
        let binary = self.root.join("substrate/target/release/openspore");
        if binary.exists() {
            log("✅ Rust binary found (release)", "green");
        } else {
            let debug_binary = self.root.join("substrate/target/debug/openspore");
            if debug_binary.exists() {
                log("⚠️ Only debug binary found (run: cargo build --release)", "yellow");
            } else {
                self.issues.push(Issue {
                    id: "NO_BINARY".to_string(),
                    label: "No compiled binary found".to_string(),
                    severity: Severity::Critical,
                    meta: None,
                });
            }
        }

        // Check skills directory
        let skills_path = self.root.join("skills");
        if skills_path.exists() {
            let count = std::fs::read_dir(&skills_path)
                .map(|r| r.filter(|e| e.is_ok()).count())
                .unwrap_or(0);
            log(&format!("✅ skills/ directory ({} plugins)", count), "green");
        }
    }

    // --- PRESCRIPTIONS (Fixes) ---

    fn prescribe(&mut self) {
        for issue in self.issues.clone() {
            log(&format!("💊 Treatment for {}: {}...", issue.id, issue.label), "yellow");

            match issue.id.as_str() {
                "MISSING_DIR" => {
                    if let Some(dir) = &issue.meta {
                        let path = self.root.join(dir);
                        if std::fs::create_dir_all(&path).is_ok() {
                            log(&format!("   ✓ Created directory: {}", dir), "gray");
                        }
                    }
                }
                "MISSING_SOUL_FILE" => {
                    if let Some(file) = &issue.meta {
                        let path = self.root.join(file);
                        let template = if file.contains("QUEUE.md") {
                            "# Proactive Queue\n\n## Pending\n\n## Completed\n".to_string()
                        } else if file.contains("LOGS.md") {
                            "# System Logs\n\nInitialized by Doctor.\n".to_string()
                        } else {
                            let name = path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Unknown");
                            format!("# {}\nInitial substrate established by Doctor.", name)
                        };

                        // Create parent directory if needed
                        if let Some(parent) = path.parent() {
                            std::fs::create_dir_all(parent).ok();
                        }

                        if std::fs::write(&path, template).is_ok() {
                            log(&format!("   ✓ Restored stub for: {}", file), "gray");
                        }
                    }
                }
                "PERM_ERR" => {
                    if let Some(path) = &issue.meta {
                        let _ = Command::new("chmod")
                            .args(["+x", path])
                            .status();
                        log("   ✓ Fixed permissions", "gray");
                    }
                }
                "CRON_NOT_INSTALLED" => {
                    log("   👉 Run 'openspore cron install' to set up cron jobs", "magenta");
                }
                "NO_BINARY" => {
                    log("   👉 Run 'cargo build --release' in substrate/", "magenta");
                }
                "MISSING_ENV" => {
                    // Create .env from example if possible
                    let example = self.root.join(".env.example");
                    let target = self.root.join(".env");
                    if example.exists() {
                        if std::fs::copy(&example, &target).is_ok() {
                            log("   ✓ Created .env from .env.example", "gray");
                            log("   👉 Edit .env and add your API keys", "magenta");
                        }
                    } else {
                        log("   👉 Create .env file with OPENROUTER_API_KEY", "magenta");
                    }
                }
                _ => {
                    log(&format!("   ❌ Auto-repair not available for {}. Manual intervention required.", issue.id), "red");
                }
            }
        }

        log("\n✨ System has been patched. Run 'openspore stop && openspore start' if issues were critical.\n", "cyan");
    }
}

impl Default for SporeDoctor {
    fn default() -> Self {
        Self::new()
    }
}
