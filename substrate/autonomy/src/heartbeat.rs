use openspore_brain::Brain;
use openspore_memory::MemorySystem;
use openspore_telegram::TelegramChannel;
use crate::engine::AutonomyEngine;
use chrono::Timelike;
use tracing::{info, error};
use anyhow::Result;
use sysinfo::Disks;

pub struct Heartbeat;

impl Heartbeat {
    pub async fn run(brain: &Brain, memory: &MemorySystem, telegram: Option<&TelegramChannel>) -> Result<()> {
        info!("--- OpenSpore Heartbeat ---");
        if telegram.is_none() {
            info!("📡 Heartbeat: Telegram is not configured, running as local-only report.");
        }
        let mut logs = Vec::new();
        let mut status = "🟢 OPTIMAL";

        // 1. Disk Check
        let disks = Disks::new_with_refreshed_list();
        if let Some(disk) = disks.iter().find(|d| d.mount_point() == std::path::Path::new("/")) {
            let avail_gb = disk.available_space() / 1024 / 1024 / 1024;
            logs.push(format!("💾 Disk: {} GB available", avail_gb));
        }

        // 2. Journal Check
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let journal = memory.project_root.join("workspace/memory").join(format!("{}.md", today));
        let hour = chrono::Local::now().hour();

        if journal.exists() {
            logs.push(format!("📝 Journal: {}.md is present", today));
        } else if hour >= 22 {
            logs.push(format!("⚠️ Journal: {}.md missing", today));
            status = "🟡 CAUTION";
        } else {
            logs.push("📝 Journal: Pending (22:00)".to_string());
        }

        // 3. Substrate Integrity
        let mut doctor = openspore_doctor::SporeDoctor::new();
        let doctor_ok = doctor.check_all();

        if doctor_ok {
            logs.push("📁 Substrate: Healthy (Doctor Verified)".to_string());
        } else {
            logs.push("🩹 Substrate: Issues found & treated by Doctor".to_string());
            status = "🟡 CAUTION";
        }

        // 4. Trigger Autonomy Engine
        if let Ok(enabled) = std::env::var("AUTONOMY_ENABLED") {
            if enabled == "true" {
                match AutonomyEngine::run(brain, memory).await {
                    Ok(Some(proposal_path)) => {
                        let filename = proposal_path.file_name().and_then(|f| f.to_str()).unwrap_or("proposal.md");

                        // Parse title from file
                        let title = if let Ok(content) = std::fs::read_to_string(&proposal_path) {
                            content.lines()
                                .find(|line| line.starts_with("title: "))
                                .map(|line| line.trim_start_matches("title: ").trim().to_string())
                                .unwrap_or_else(|| filename.to_string())
                        } else {
                            filename.to_string()
                        };

                        logs.push(format!("✨ NEW PROPOSAL: {}", title));
                    },
                    Ok(None) => {},
                    Err(e) => {
                        error!("Autonomy Error: {}", e);
                        status = "🟡 CAUTION";
                    }
                }
            }
        }

        // 5. Send Combined Report
        let time = chrono::Local::now().format("%H:%M:%S").to_string();
        let report = format!(
            "💓 *OpenSpore Heartbeat*\n\nStatus: *{}*\n\n{}\n\n_{}_",
            status,
            logs.join("\n"),
            time
        );

        info!("--- Heartbeat Result ---\n{}", report);

         if let Some(tg) = telegram {
             info!("📡 Heartbeat: Attempting to send report via Telegram...");
             if let Err(e) = tg.send_raw(&report).await {
                 error!("❌ Heartbeat: Failed to send report to Telegram: {}", e);
             } else {
                 info!("✅ Heartbeat: Telegram notification sent.");
             }
         }

        Ok(())
    }
}
