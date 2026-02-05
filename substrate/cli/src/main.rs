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
    std::env::var("HOME").unwrap_or_else(|_| ".".to_string()) + "/.openspore"
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let app_dir = get_app_dir();
    let log_file = format!("{}/openspore.log", app_dir);

    // Initialize logging to file
    let file = std::fs::File::create(&log_file).unwrap_or_else(|_| {
        std::fs::File::open("/dev/null").unwrap()
    });

    let (non_blocking, _guard) = tracing_appender::non_blocking(file);

    if matches!(args.command, Some(Commands::Start) | None) {
        // TUI mode: Only log to file, keep stdout clean for REPL
        tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .init();
    } else {
        // CLI mode: Log to stdout for feedback
        tracing_subscriber::fmt()
            .with_writer(std::io::stdout)
            .init();
    }

    openspore_core::init();

    match args.command {
        Some(Commands::Start) | None => {
            // TUI Mode
            if let Err(e) = openspore_tui::run() {
                eprintln!("TUI Error: {}", e);
            }
        }
        Some(Commands::Stop) => {
            println!("🛑 Stopping all OpenSpore instances...");
            let _ = Command::new("pkill")
                .args(["-f", "openspore"])
                .status();
            println!("✅ All instances stopped.");
        }
        Some(Commands::Doctor) => {
            let mut doctor = openspore_doctor::SporeDoctor::new();
            doctor.check_all();
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
                                let binary = format!("{}/substrate/target/release/openspore", app_dir);
                                let mut cron_lines = Vec::new();
                                for (name, job) in obj {
                                    let schedule = job["schedule"].as_str().unwrap_or("* * * * *");
                                    cron_lines.push(format!("{} {} job {} > /dev/null 2>&1", schedule, binary, name));
                                }
                                println!("Would install {} cron entries:", cron_lines.len());
                                for line in &cron_lines {
                                    println!("  {}", line);
                                }
                                println!("\n⚠️ Manual installation required (run: crontab -e)");
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
        Some(Commands::Auto) => {
            println!("🧠 [Anticipation Engine]: Analyzing patterns...");
            match AppConfig::load() {
                Ok(config) => {
                    let brain = Brain::new(config);
                    let response = brain.think("Review my recent context and suggest one proactive action I should take.").await;
                    println!("\n{}", response);
                }
                Err(e) => error!("Failed to load config: {}", e),
            }
        }
        Some(Commands::Logs) => {
            let context_dir = format!("{}/workspace/context", app_dir);
            println!("📜 Recent context files:\n");
            let output = Command::new("ls")
                .args(["-lt", &context_dir])
                .output();
            if let Ok(out) = output {
                let content = String::from_utf8_lossy(&out.stdout);
                for line in content.lines().take(10) {
                    println!("{}", line);
                }
            }
        }
        Some(Commands::Think { prompt, role }) => {
            match AppConfig::load() {
                Ok(config) => {
                    let brain = Brain::new(config);
                    // Set IS_SPORE for this process
                    std::env::set_var("IS_SPORE", "true");
                    if let Some(r) = role {
                        std::env::set_var("SPORE_ROLE", r);
                    }

                    let response = brain.think(&prompt).await;
                    println!("{}", response);
                }
                Err(e) => error!("Failed to load config: {}", e),
            }
        }
    }
}
