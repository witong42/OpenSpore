use clap::{Parser, Subcommand};
use tracing::error;
use openspore_core::config::AppConfig;
use openspore_brain::Brain;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(author, version, about = "OpenSpore - Autonomous AI Agent", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the OpenSpore TUI Agent
    Start,
    /// Stop all running OpenSpore instances
    Stop,
    /// Run system diagnostic and self-repair
    Doctor,
    /// Manage system cron jobs (list/install)
    Cron {
        /// Subcommand: list or install
        #[arg(index = 1, default_value = "list")]
        action: String,
    },
    /// Manually run a workspace job
    Job {
        /// Job name (e.g., heartbeat, cleanup)
        #[arg(index = 1)]
        name: String,
    },
    /// Trigger the autonomy anticipation engine
    Auto,
    /// View recent context logs
    Logs,
    /// List active sub-spores (swarm activity)
    Swarm,
    /// Run system heartbeat and status check
    Heartbeat,
    /// Manually trigger daily journal synthesis
    Journal,
    /// One-shot think for swarm/spores
    Think {
        /// The prompt for the AI
        #[arg(index = 1)]
        prompt: String,
        /// Optional role for the spore
        #[arg(long)]
        role: Option<String>,
    },
}

fn get_app_dir() -> String {
    openspore_core::path_utils::get_app_root().to_string_lossy().to_string()
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let app_dir = get_app_dir();
    let log_file = format!("{}/openspore.log", app_dir);

    // Initialize logging to file (append mode)
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .unwrap_or_else(|_| {
            std::fs::File::open("/dev/null").unwrap()
        });

    let (non_blocking, _guard) = tracing_appender::non_blocking(file);

    if matches!(args.command, Some(Commands::Start) | None) {
        // TUI mode: Only log to file, keep stdout clean for REPL
        tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_ansi(false)
            .init();
    } else {
        // CLI mode: Log to stdout for feedback
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_ansi(true)
            .init();
    }

    openspore_core::init();

    // 1. Commands that DON'T require config
    if let Some(cmd) = &args.command {
        match cmd {
            Commands::Stop => {
                println!("🛑 Stopping all OpenSpore instances and browsers...");
                let _ = Command::new("pkill").args(["-f", "openspore"]).status();
                let _ = Command::new("pkill").args(["-f", "Google Chrome"]).status();
                let _ = Command::new("pkill").args(["-f", "Chromium"]).status();
                let _ = Command::new("pkill").args(["-f", "Brave Browser"]).status();

                let app_dir = openspore_core::path_utils::get_app_root();
                let session_file = app_dir.join("workspace").join("browser_sessions").join("active_session.json");
                if session_file.exists() {
                    let _ = std::fs::remove_file(session_file);
                }

                println!("✅ All instances and browsers stopped.");
                return;
            }
            Commands::Doctor => {
                let mut doctor = openspore_doctor::SporeDoctor::new();
                doctor.check_all();
                return;
            }
            _ => {} // Continue to config loading for other commands
        }
    }

    // 2. Load Config (Required for all remaining commands)
    let config = match AppConfig::load() {
        Ok(cfg) => cfg,
        Err(_) => {
            eprintln!("❌ Error: Configuration missing or invalid.");
            eprintln!("   Please ensure ~/.openspore/.env exists and contains a valid OPENROUTER_API_KEY.");
            eprintln!("   Run 'openspore doctor' for diagnostics.");
            std::process::exit(1);
        }
    };

    // 3. Command Dispatch
    match args.command {
        Some(Commands::Start) | None => {
            // TUI Mode
            if let Err(e) = openspore_tui::run().await {
                eprintln!("TUI Error: {}", e);
            }
        }
        Some(Commands::Cron { action }) => {
            let cron_dir = format!("{}/workspace/cron", app_dir);
            let manifest = format!("{}/crontab.json", cron_dir);

            match action.as_str() {
                "list" => {
                    println!("⏰ Active OpenSpore Cron Jobs:\n");
                    if let Ok(content) = std::fs::read_to_string(&manifest) {
                        if let Ok(jobs) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(obj) = jobs.as_object() {
                                for (name, job) in obj {
                                    let schedule = job["schedule"].as_str().unwrap_or("?");
                                    let script = job["script"].as_str().unwrap_or("?");
                                    println!("  {:15} | {:12} | {}", name, schedule, script);
                                }
                            }
                        }
                    } else {
                        println!("No crontab.json found at {}", manifest);
                    }
                }
                "install" => {
                    println!("🛠️ Installing cron jobs...");
                    if let Ok(content) = std::fs::read_to_string(&manifest) {
                        if let Ok(jobs) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(obj) = jobs.as_object() {
                                let binary = format!("{}/crates/target/release/openspore", app_dir);
                                let mut cron_lines = Vec::new();

                                if let Ok(output) = Command::new("crontab").arg("-l").output() {
                                    let content = String::from_utf8_lossy(&output.stdout);
                                    for line in content.lines() {
                                        if !line.contains("openspore job") && !line.trim().is_empty() {
                                            cron_lines.push(line.to_string());
                                        }
                                    }
                                }

                                for (name, job) in obj {
                                    let schedule = job["schedule"].as_str().unwrap_or("* * * * *");
                                    cron_lines.push(format!("{} {} job \"{}\" > /dev/null 2>&1", schedule, binary, name));
                                }

                                let new_crontab = cron_lines.join("\n") + "\n";
                                println!("Installing {} cron entries...", cron_lines.len());

                                use std::io::Write;
                                let mut child = Command::new("crontab")
                                    .stdin(std::process::Stdio::piped())
                                    .spawn()
                                    .expect("Failed to spawn crontab command");

                                if let Some(mut stdin) = child.stdin.take() {
                                    stdin.write_all(new_crontab.as_bytes()).expect("Failed to write to crontab stdin");
                                }

                                match child.wait() {
                                    Ok(status) if status.success() => println!("✅ Cron jobs installed successfully."),
                                    _ => println!("❌ Failed to install cron jobs."),
                                }
                            }
                        }
                    } else {
                        println!("❌ No crontab.json found");
                    }
                }
                _ => println!("Usage: openspore cron [list|install]"),
            }
        }
        Some(Commands::Job { name }) => {
            let cron_dir = format!("{}/workspace/cron", app_dir);
            let manifest = format!("{}/crontab.json", cron_dir);

            if let Ok(content) = std::fs::read_to_string(&manifest) {
                if let Ok(jobs) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(script) = jobs.get(&name).and_then(|j| j["script"].as_str()) {
                        let script_path = format!("{}/{}", cron_dir, script);
                        println!("🚀 [Spore Job]: {} ({})", name, script);

                        let status = if script.ends_with(".js") {
                            Command::new("node").arg(&script_path).status()
                        } else {
                            Command::new("sh").arg(&script_path).status()
                        };

                        match status {
                            Ok(s) if s.success() => println!("✅ Job completed"),
                            Ok(s) => println!("⚠️ Job exited with: {}", s),
                            Err(e) => println!("❌ Failed to run job: {}", e),
                        }
                    } else {
                        println!("❌ Job '{}' not found in crontab.json", name);
                    }
                }
            } else {
                println!("❌ No crontab.json found");
            }
        }
        Some(Commands::Swarm) => {
            println!("🐝 [Swarm Status]: Scanning for active sub-spores...");
            let swarm = openspore_swarm::SwarmManager::new();
            match swarm.discovery().await {
                Ok(lines) => {
                    if lines.is_empty() {
                        println!("📭 No active sub-spores found.");
                    } else {
                        println!("✅ Found {} active sub-spores:", lines.len());
                        for line in lines {
                            println!("  - {}", line);
                        }
                    }
                }
                Err(e) => println!("❌ Error: {}", e),
            }
        }
        Some(Commands::Auto) => {
            println!("🧠 [Autonomy Engine]: Analyzing patterns & generating proposal...");
            let state = openspore_core::state::AppState::new(config.clone());
            let brain = Brain::new(config);
            let memory = openspore_memory::MemorySystem::new(&state);

            match openspore_autonomy::AutonomyEngine::run(&brain, &memory).await {
                Ok(Some(path)) => println!("\n✅ Proposal created at: {}", path.display()),
                Ok(None) => println!("\n⏸️ No new proposal deemed necessary at this time."),
                Err(e) => error!("Autonomy Engine failed: {}", e),
            }
        }
        Some(Commands::Logs) => {
            let context_dir = format!("{}/workspace/context", app_dir);
            println!("📜 Recent context files:\n");
            let output = Command::new("ls").args(["-lt", &context_dir]).output();
            if let Ok(out) = output {
                let content = String::from_utf8_lossy(&out.stdout);
                for line in content.lines().take(10) { println!("{}", line); }
            }
        }
        Some(Commands::Think { prompt, role }) => {
            let brain = Brain::new(config);
            unsafe {
                std::env::set_var("IS_SPORE", "true");
                if let Some(r) = role { std::env::set_var("SPORE_ROLE", r); }
            }
            let response = brain.think(&prompt).await;
            println!("{}", response);
        }
        Some(Commands::Heartbeat) => {
            println!("💓 [System Heartbeat]");
            let state = openspore_core::state::AppState::new(config.clone());
            let brain = openspore_brain::Brain::new(config);
            let memory = openspore_memory::MemorySystem::new(&state);
            let telegram = openspore_telegram::TelegramChannel::new().ok();

            if let Err(e) = openspore_autonomy::Heartbeat::run(&brain, &memory, telegram.as_ref()).await {
                error!("Heartbeat failed: {}", e);
            }
        }
        Some(Commands::Journal) => {
            println!("📓 [Daily Journal Synthesis]");
            let state = openspore_core::state::AppState::new(config.clone());
            let brain = openspore_brain::Brain::new(config);
            let memory = openspore_memory::MemorySystem::new(&state);

            match openspore_autonomy::DailyJournal::run(&brain, &memory).await {
                Ok(Some(path)) => println!("✅ Journal created: {:?}", path),
                Ok(None) => println!("⏸️ No journal created (already exists or empty context)"),
                Err(e) => error!("Journal synthesis failed: {}", e),
            }
        }
        _ => {} // Already handled Stop/Doctor
    }
}
