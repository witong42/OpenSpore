//! Telegram Send Skill (Core)

use super::Skill;
use async_trait::async_trait;
use reqwest::Client;

pub struct TelegramSendSkill;

#[async_trait]
impl Skill for TelegramSendSkill {
    fn name(&self) -> &'static str { "telegram_send" }

    fn description(&self) -> &'static str {
        "Send a message via Telegram. Usage: [TELEGRAM_SEND: \"message\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let message = args.trim().trim_matches('"').trim_matches('\'');

        let token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| "TELEGRAM_BOT_TOKEN not set")?;
        let chat_id = std::env::var("TELEGRAM_ALLOWED_USERS")
            .map_err(|_| "TELEGRAM_ALLOWED_USERS not set")?;

        let recipient = chat_id.split(',').next().unwrap_or(&chat_id);
        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);

        let client = Client::new();
        let res = client.post(&url)
            .json(&serde_json::json!({
                "chat_id": recipient,
                "text": message,
                "parse_mode": "Markdown"
            }))
            .send()
            .await
            .map_err(|e| format!("Telegram API error: {}", e))?;

        if res.status().is_success() {
            Ok("✅ Message sent to Telegram".to_string())
        } else {
            Err(format!("Telegram error: {}", res.status()))
        }
    }
}
