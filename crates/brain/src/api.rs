use crate::{Brain, Message};
use tracing::warn;

impl Brain {
    /// LLM API Call with Retry Logic (Exponential Backoff)
    pub async fn complete(&self, messages: &[Message]) -> anyhow::Result<String> {
        let model = self.get_model();
        let api_key = std::env::var("OPENROUTER_API_KEY")?;

        // Standard temp
        let temp = 0.7;

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": temp,
        });

        // Retry logic: 3 attempts with exponential backoff
        let mut attempts = 0;
        let max_attempts = 3;

        loop {
            attempts += 1;

            let res = self.client.post("https://openrouter.ai/api/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .header("HTTP-Referer", "https://openspore.net")
                .header("X-Title", "OpenSpore")
                .json(&body)
                .send()
                .await?;

            let status = res.status();

            if status.is_success() {
                let json: serde_json::Value = res.json().await?;
                return Ok(json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string());
            }

            // Handle retryable errors (429 Too Many Requests, 500+ Server Errors)
            if (status.as_u16() == 429 || status.as_u16() >= 500) && attempts < max_attempts {
                let backoff_ms = 1000 * u64::pow(2, attempts - 1); // 1s, 2s, 4s
                warn!("⚠️ API Error {}, retrying in {}ms (attempt {}/{})", status, backoff_ms, attempts, max_attempts);
                tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                continue;
            }

            // Non-retryable error or max attempts reached
            return Err(anyhow::anyhow!("API Error: {} (after {} attempts)", status, attempts));
        }
    }

    pub(crate) fn get_model(&self) -> String {
        self.config.model.clone().unwrap_or("google/gemini-2.0-flash-001".to_string())
    }

    /// Simple one-shot thought for other modules (Watchman, etc)
    pub async fn think_simple(&self, prompt: &str) -> String {
        let msgs = vec![Message{role:"user".to_string(), content: prompt.to_string()}];
        self.complete(&msgs).await.unwrap_or_else(|e| e.to_string())
    }
}
