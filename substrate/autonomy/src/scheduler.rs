use openspore_brain::Brain;
use openspore_memory::MemorySystem;
use openspore_telegram::TelegramChannel;
use crate::heartbeat::Heartbeat;
use crate::journal::DailyJournal;
use chrono::Timelike;
use tokio::time::{sleep, Duration};
use tracing::{info, error};

pub struct SporeScheduler;

impl SporeScheduler {
    pub async fn start(brain: Brain, memory: MemorySystem, telegram: Option<TelegramChannel>) {
        info!("🕒 Spore Scheduler: Background engine spinning up...");

        let mut last_heartbeat = std::time::Instant::now(); // Wait for the first interval
        let mut last_journal_day = String::new();

        loop {
            let now = chrono::Local::now();
            let today = now.format("%Y-%m-%d").to_string();

            // 1. Heartbeat every 2 hours
            if last_heartbeat.elapsed() >= Duration::from_secs(2 * 3600) {
                let brain_ref = brain.clone();
                let memory_ref = memory.clone();
                let telegram_ref = telegram.clone();

                tokio::spawn(async move {
                    if let Err(e) = Heartbeat::run(&brain_ref, &memory_ref, telegram_ref.as_ref()).await {
                        error!("Scheduler: Heartbeat error: {}", e);
                    }
                });
                last_heartbeat = std::time::Instant::now();
            }

            // 2. Daily Journal at 22:00
            if now.hour() == 22 && last_journal_day != today {
                info!("🕒 Spore Scheduler: Time for daily synthesis!");
                let brain_ref = brain.clone();
                let memory_ref = memory.clone();
                let telegram_ref = telegram.clone();

                tokio::spawn(async move {
                    match DailyJournal::run(&brain_ref, &memory_ref).await {
                        Ok(Some(path)) => {
                            if let Some(tg) = telegram_ref {
                                let _ = tg.send_raw(&format!("📓 Daily Journal synthesized: {}", path.display())).await;
                            }
                        },
                        Ok(None) => {},
                        Err(e) => error!("Scheduler: Journal error: {}", e),
                    }
                });
                last_journal_day = today;
            }

            // Sleep for 1 minute
            sleep(Duration::from_secs(60)).await;
        }
    }
}
