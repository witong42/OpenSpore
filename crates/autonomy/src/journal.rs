use openspore_brain::Brain;
use openspore_memory::MemorySystem;
use std::fs;
use std::path::PathBuf;
use tracing::info;
use anyhow::Result;

pub struct DailyJournal;

impl DailyJournal {
    pub async fn run(brain: &Brain, memory: &MemorySystem) -> Result<Option<PathBuf>> {
        info!("📓 Daily Journal: Initiating Deep Synthesis of Day Context...");

        let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
        let journal_dir = memory.project_root.join("workspace/memory");
        let journal_path = journal_dir.join(format!("{}.md", today_str));

        if journal_path.exists() {
            info!("🔄 Journal exists. Appending new context.");
        }

        // 1. Collect Context
        let (aggregated_context, interactions) = Self::collect_context(memory)?;

        if aggregated_context.trim().is_empty() {
             info!("⏸️ No active exchanges detected for today (empty content). Skipping journal generation.");
             return Ok(None);
        }

        // 2. Synthesize
        let synthesis = Self::synthesize_text(brain, &aggregated_context, &today_str).await;

        if !journal_dir.exists() {
            fs::create_dir_all(&journal_dir)?;
        }

        if journal_path.exists() {
            // Append mode
            let timestamp = chrono::Local::now().format("%H:%M:%S");
            let append_content = format!("\n\n---\n\n## 🔄 Update [{}]\n\n{}", timestamp, synthesis.trim());

            // Read existing to append (or just use append open option)
            // Using fs::OpenOptions for atomic append is cleaner
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(&journal_path)?;

            write!(file, "{}", append_content)?;
            info!("✅ Journal updated (appended) at {}", journal_path.display());
        } else {
            // Create mode
            fs::write(&journal_path, synthesis.trim())?;
            info!("✅ Journal created at {}", journal_path.display());
        }

        // 3. Cleanup
        Self::delete_interactions(&interactions)?;
        Self::clear_active_logs(memory)?;

        Ok(Some(journal_path))
    }

    fn collect_context(memory: &MemorySystem) -> Result<(String, Vec<PathBuf>)> {
        let context_dir = memory.project_root.join("workspace/context");
        if !context_dir.exists() {
            std::fs::create_dir_all(&context_dir)?;
        }

        let mut aggregated_context = String::new();
        let mut interactions = Vec::new();

        // Check for exchanges
        if let Ok(entries) = fs::read_dir(&context_dir) {
            interactions = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file() && e.file_name().to_str().map(|s| s.starts_with("exchange_")).unwrap_or(false))
                .map(|e| e.path())
                .collect::<Vec<_>>();

            interactions.sort();

            for path in &interactions {
                if let Ok(content) = fs::read_to_string(path) {
                    aggregated_context.push_str(&format!("\n\n---\n\n{}", content));
                }
            }
        }

        // Include LOGS.md if it exists
        let log_path = context_dir.join("LOGS.md");
        if log_path.exists() {
            if let Ok(raw_log) = fs::read_to_string(&log_path) {
                aggregated_context.push_str(&format!("\n\n### RAW_ACTIVE_CONTEXT\n{}", raw_log));
            }
        }

        Ok((aggregated_context, interactions))
    }

    async fn synthesize_text(brain: &Brain, context: &str, today_str: &str) -> String {
        let prompt = format!(r#"
You are the **OpenSpore Scribe**. Your task is to transform a day's raw interaction logs into a high-level, human-readable Strategic Journal.

<DAILY_CONTEXT>
{context}
</DAILY_CONTEXT>

<OUTPUT_FORMAT_STYLE>
Look at the style of "2026-02-02.md":
- Title: # {today_str}: [Short Catchy Summary]
- Sections:
  ## 🍄 Daily Summary (A brief narrative of the day's vibe and major focus)
  ## 🛠️ System Improvements (What technical fixes or new skills were added?)
  ## ✅ Accomplishments (List of key milestones hit)
  ## 💭 Insights & Learning (What patterns did we notice about the engine or the user?)
  ## 🏎️ Performance Metrics (Summary of latency or efficiency observations)
  ## 🎯 Tomorrow's Focus (What is the highest-leverage task to tackle next?)
</OUTPUT_FORMAT_STYLE>

Rules:
1. Be concise but descriptive.
2. Use professional, strategic language.
3. DO NOT include raw logs. Synthesize the meaning.
4. Output ONLY the markdown content.
"#);
        brain.think(&prompt).await
    }

    fn delete_interactions(interactions: &[PathBuf]) -> Result<()> {
        for path in interactions {
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }
        info!("🧹 Successfully purged {} raw exchanges after synthesis.", interactions.len());
        Ok(())
    }

    fn clear_active_logs(memory: &MemorySystem) -> Result<()> {
        let log_path = memory.project_root.join("workspace/context/LOGS.md");
        if log_path.exists() {
            fs::write(&log_path, "")?;
            info!("🧹 LOGS.md safely wiped and reset.");
        }
        Ok(())
    }
}
