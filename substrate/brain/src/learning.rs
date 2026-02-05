use crate::{Brain, Message};
use tracing::{info, warn};

impl Brain {
    /// Extract and save preferences/knowledge from conversation
    pub async fn learn(&self, prompt: &str, response: &str) -> bool {
        let learn_prompt = format!(r#"Analyze this conversation for learnable information:

User: {}
AI: {}

Extract:
1. User preferences (likes, dislikes, habits, workflow)
2. Factual knowledge worth remembering
3. Important context for future interactions

Return JSON ONLY (no markdown):
{{
  "should_save": true/false,
  "category": "preferences"|"knowledge"|"context",
  "title": "Short descriptive title",
  "content": "What to remember",
  "tags": ["tag1", "tag2"]
}}

If nothing worth saving, set should_save to false."#, prompt, response);

        match self.complete(&[Message{role:"user".into(), content: learn_prompt}]).await {
            Ok(json_str) => {
                // Robust JSON extraction
                let start = json_str.find('{');
                let end = json_str.rfind('}');

                if let (Some(s), Some(e)) = (start, end) {
                    if s <= e {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_str[s..=e]) {
                            if data["should_save"].as_bool().unwrap_or(false) {
                                let category = data["category"].as_str().unwrap_or("context");
                                let title = data["title"].as_str().unwrap_or("Learned Info");
                                let content = data["content"].as_str().unwrap_or("");
                                let tags: Vec<String> = data["tags"]
                                    .as_array()
                                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                                    .unwrap_or_default();

                                if let Ok(Some(path)) = self.memory.save_memory(category, title, content, tags, Some("learned")).await {
                                    info!("🧠 Learned: {} -> {:?}", title, path);

                                    // Log to LOGS.md
                                    let log_entry = format!("\n[{}] 🧠 Learned: {} ({})\n",
                                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                                        title,
                                        category
                                    );
                                    let _ = self.memory.save_journal(&log_entry).await;

                                    return true;
                                }
                            }
                        }
                    }
                }
            },
            Err(e) => {
                warn!("Learn failed: {}", e);
            }
        }
        false
    }
}
