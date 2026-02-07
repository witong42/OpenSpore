use crate::{Watchman, types::WatchEvent};
use tracing::info;

impl Watchman {
    /// Process a single event - read file and trigger learn()
    pub(crate) async fn process_event(&self, event: &WatchEvent) -> anyhow::Result<()> {
        let content = tokio::fs::read_to_string(&event.file_path).await?;

        // Truncate content preview
        let preview = if content.len() > 1000 {
            format!("{}...", &content[..1000])
        } else {
            content.clone()
        };

        // Create context for learning
        let context = format!(
            "[System Event]: File {} detected at {:?}.\nContent Preview:\n{}",
            event.event_type,
            event.file_path,
            preview
        );

        // Use brain to analyze and extract knowledge
        // This matches memory.learn() in JS
        let analysis_prompt = format!(
            r#"Analyze this file change for new knowledge:

{}

If this contains:
1. User preferences (likes, dislikes, workflow habits)
2. New factual knowledge
3. Critical context for future tasks

Output JSON:
{{"should_save": true/false, "category": "preferences"|"knowledge"|"context", "title": "Short Title", "content": "...", "tags": ["tag1"]}}

If nothing worth saving, set "should_save": false.
Do not wrap in markdown blocks. Return raw JSON only."#,
            context
        );

        let response = self.brain.think_simple(&analysis_prompt).await;

        // Robust JSON extraction
        let start = response.find('{').unwrap_or(0);
        let end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let clean_json = &response[start..end];

        // Try to parse JSON response
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(clean_json) {
            if data["should_save"].as_bool().unwrap_or(false) {
                let category = data["category"].as_str().unwrap_or("context");
                let title = data["title"].as_str().unwrap_or("untitled");
                let save_content = data["content"].as_str().unwrap_or(&content);
                let tags: Vec<String> = data["tags"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();

                if let Ok(Some(path)) = self.memory.save_memory(category, title, save_content, tags, Some("learned")).await {
                    info!("🧠 Watchman Learned: {} -> {:?}", title, path);
                }
            }
        }

        Ok(())
    }
}
