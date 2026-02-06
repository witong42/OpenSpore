//! OpenSpore TUI - Enhanced REPL with History & Signal Handling
//! Uses rustyline for readline-like experience

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use tokio::sync::mpsc;

const VERSION: &str = "0.1.0";

/// ANSI color codes
mod color {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const CYAN: &str = "\x1b[36m";
    pub const MAGENTA: &str = "\x1b[35m";
}

/// Main run function
pub fn run() -> anyhow::Result<()> {
    // Check if we're already in a runtime
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(repl_loop())
        }).join().map_err(|_| anyhow::anyhow!("TUI thread panicked"))?
    } else {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(repl_loop())
    }
}

async fn repl_loop() -> anyhow::Result<()> {
    // Print header
    println!();
    println!("{}{}🍄 OpenSpore v{}{}", color::BOLD, color::CYAN, VERSION, color::RESET);
    println!("{}Type a message, or 'help' for commands. 'exit' to quit.{}", color::DIM, color::RESET);
    println!("{}Use ↑/↓ arrows for history, Ctrl+C to cancel input{}", color::DIM, color::RESET);
    println!("{}─────────────────────────────────────────────────────{}", color::DIM, color::RESET);
    println!();

    // Initialize Core Components ONCE
    let (brain, memory, config) = if let Ok(config) = openspore_core::config::AppConfig::load() {
        // Run Pre-flight Diagnostic (Doctor) synchronously
        let mut doctor = openspore_doctor::SporeDoctor::new();
        doctor.check_all();

        let state = openspore_core::state::AppState::new(config.clone());
        let memory = openspore_memory::MemorySystem::new(&state);
        let brain = openspore_brain::Brain::new(config.clone());
        (Some(brain), Some(memory), Some(config))
    } else {
        (None, None, None)
    };

    // Start Watchman in background
    if let (Some(brain), Some(memory), Some(config)) = (&brain, &memory, &config) {
        let watchman = std::sync::Arc::new(openspore_watchman::Watchman::new(config.clone(), brain.clone_brain(), memory.clone()));
        let wm = watchman.clone();
        tokio::spawn(async move {
            println!("{}👀 Watchman started (monitoring filesystem){}", color::DIM, color::RESET);
            if let Err(e) = wm.start().await {
                eprintln!("{}⚠️ Watchman error: {}{}", color::YELLOW, e, color::RESET);
            }
        });
    }

    // Start Telegram Gateway in background (if token present)
    let telegram = if std::env::var("TELEGRAM_BOT_TOKEN").is_ok() {
        if let Ok(tg) = openspore_telegram::TelegramChannel::new() {
            println!("{}📡 Telegram started (listening for messages){}", color::DIM, color::RESET);
            let tg_clone = tg.clone();
            tokio::spawn(async move {
                if let Err(e) = tg_clone.start().await {
                    eprintln!("{}⚠️ Telegram error: {}{}", color::YELLOW, e, color::RESET);
                }
            });
            Some(tg)
        } else { None }
    } else { None };

    // Start Autonomy Scheduler in background
    if let (Some(brain), Some(memory)) = (&brain, &memory) {
        let brain_clone = brain.clone_brain();
        let memory_clone = memory.clone();
        let tg_clone = telegram.clone();
        tokio::spawn(async move {
            openspore_autonomy::SporeScheduler::start(brain_clone, memory_clone, tg_clone).await;
        });
    }

    let (tx, mut rx) = mpsc::channel::<String>(10);

    // Initialize rustyline editor
    let mut rl = DefaultEditor::new()?;

    // History persistence disabled as per user request

    loop {
        // Check for any pending brain responses
        while let Ok(response) = rx.try_recv() {
            println!();
            println!("{}🍄 {}", color::MAGENTA, color::RESET);
            for line in response.lines() {
                println!("   {}", line);
            }
            println!();
        }

        // Read input with rustyline (supports history and Ctrl+C)
        let readline = rl.readline(&format!("{}Spore>{} ", color::GREEN, color::RESET));

        match readline {
            Ok(input) => {
                let input = input.trim().to_string();
                if input.is_empty() {
                    continue;
                }

                // Add to history
                let _ = rl.add_history_entry(&input);

                let cmd = input.to_lowercase();

                // Handle commands
                if cmd == "exit" || cmd == "quit" {
                    println!("\n{}🍄 Hibernating...{}\n", color::YELLOW, color::RESET);
                    break;
                } else if cmd == "help" {
                    println!();
                    println!("{}Commands:{}", color::YELLOW, color::RESET);
                    println!("  help     - Show this message");
                    println!("  status   - System status");
                    println!("  doctor   - Run diagnostics");
                    println!("  auto     - Trigger autonomy");
                    println!("  clear    - Clear screen");
                    println!("  exit     - Quit");
                    println!();
                    println!("{}Keyboard Shortcuts:{}", color::YELLOW, color::RESET);
                    println!("  ↑/↓      - Navigate command history");
                    println!("  Ctrl+C   - Cancel current input");
                    println!("  Ctrl+D   - Exit");
                    println!();
                    println!("{}Anything else is sent to the AI brain.{}", color::DIM, color::RESET);
                    println!();
                } else if cmd == "clear" {
                    print!("\x1b[2J\x1b[H"); // Clear screen
                    use std::io::Write;
                    std::io::stdout().flush()?;
                } else if cmd == "status" {
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                    let skills = std::fs::read_dir(format!("{}/.openspore/skills", home))
                        .map(|r| r.count()).unwrap_or(0);
                    println!();
                    println!("{}Status:{} Active", color::CYAN, color::RESET);
                    println!("{}Substrate:{} ~/.openspore", color::CYAN, color::RESET);
                    println!("{}Skills:{} {} plugins loaded", color::CYAN, color::RESET, skills);
                    println!();
                } else if cmd == "doctor" {
                    println!();
                    let mut doctor = openspore_doctor::SporeDoctor::new();
                    doctor.check_all();
                    println!();
                } else if cmd == "auto" {
                    println!("\n{}🧠 Triggering autonomy engine...{}\n", color::YELLOW, color::RESET);
                    let tx_clone = tx.clone();
                    let brain_clone = brain.clone().map(|b| b.clone_brain()); // Option<Brain>
                    tokio::spawn(async move {
                        if let Some(brain) = brain_clone {
                            let response = brain.think("Suggest one proactive action based on my context.").await;
                            let _ = tx_clone.send(response).await;
                        }
                    });
                    // Wait a bit for the response
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    wait_for_response(&mut rx).await;
                } else {
                    // Send to brain
                    println!("\n{}🤔 Thinking...{}", color::MAGENTA, color::RESET);

                    if let Some(brain) = &brain {
                        let response = brain.think(&input).await;
                        println!();
                        println!("{}🍄 {}", color::MAGENTA, color::RESET);
                        for line in response.lines() {
                            println!("   {}", line);
                        }
                        println!();
                    } else {
                        eprintln!("{}⚠️ Brain not initialized (check config){}", color::YELLOW, color::RESET);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C pressed - just show a new prompt
                println!("{}^C{}", color::DIM, color::RESET);
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl+D pressed - exit gracefully
                println!("\n{}🍄 Hibernating...{}\n", color::YELLOW, color::RESET);
                break;
            }
            Err(err) => {
                eprintln!("{}Error: {}{}", color::YELLOW, err, color::RESET);
                break;
            }
        }
    }

    // History persistence disabled

    Ok(())
}

async fn wait_for_response(rx: &mut mpsc::Receiver<String>) {
    // Wait up to 30 seconds for a response
    for _ in 0..300 {
        if let Ok(response) = rx.try_recv() {
            println!();
            println!("{}🍄 {}", color::MAGENTA, color::RESET);
            for line in response.lines() {
                println!("   {}", line);
            }
            println!();
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
