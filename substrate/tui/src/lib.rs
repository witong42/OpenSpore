//! OpenSpore TUI - Simple REPL-style Interface
//! Clean, readable, copy-paste friendly

use std::io::{self, Write};
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
    pub const WHITE: &str = "\x1b[37m";
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
    println!("{}─────────────────────────────────────────────────────{}", color::DIM, color::RESET);
    println!();

    // Start Watchman in background
    if let Ok(config) = openspore_core::config::AppConfig::load() {
        let watchman = std::sync::Arc::new(openspore_watchman::Watchman::new(config));
        let wm = watchman.clone();
        tokio::spawn(async move {
            println!("{}👀 Watchman started (monitoring filesystem){}", color::DIM, color::RESET);
            if let Err(e) = wm.start().await {
                eprintln!("{}⚠️ Watchman error: {}{}", color::YELLOW, e, color::RESET);
            }
        });
    }

    // Start Telegram Gateway in background (if token present)
    if std::env::var("TELEGRAM_BOT_TOKEN").is_ok() {
        tokio::spawn(async move {
            if let Ok(tg) = openspore_telegram::TelegramChannel::new() {
                println!("{}📡 Telegram started (listening for messages){}", color::DIM, color::RESET);
                if let Err(e) = tg.start().await {
                    eprintln!("{}⚠️ Telegram error: {}{}", color::YELLOW, e, color::RESET);
                }
            }
        });
    }

    let (tx, mut rx) = mpsc::channel::<String>(10);
    let mut history: Vec<String> = Vec::new();

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

        // Prompt
        print!("{}Spore>{} ", color::GREEN, color::RESET);
        io::stdout().flush()?;

        // Read input
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }

        history.push(input.clone());
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
            println!("{}Anything else is sent to the AI brain.{}", color::DIM, color::RESET);
            println!();
        } else if cmd == "clear" {
            print!("\x1b[2J\x1b[H"); // Clear screen
            io::stdout().flush()?;
        } else if cmd == "status" {
            let home = std::env::var("HOME").unwrap_or_default();
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
            tokio::spawn(async move {
                if let Ok(config) = openspore_core::config::AppConfig::load() {
                    let brain = openspore_brain::Brain::new(config);
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

            let config = openspore_core::config::AppConfig::load()?;
            let brain = openspore_brain::Brain::new(config);
            let response = brain.think(&input).await;

            println!();
            println!("{}🍄 {}", color::MAGENTA, color::RESET);
            for line in response.lines() {
                println!("   {}", line);
            }
            println!();
        }
    }

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
