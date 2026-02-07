//! Telegram Send Skill (Core)

use super::Skill;
use async_trait::async_trait;
use reqwest::Client;

pub struct TelegramSendSkill;

#[async_trait]
impl Skill for TelegramSendSkill {
    fn name(&self) -> &'static str { "telegram_send" }

    fn description(&self) -> &'static str {
        "Send a message via Telegram. Returns JSON with success and message. Usage: [TELEGRAM_SEND: \"message\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let message = args.trim().trim_matches('"').trim_matches('\'');

        let token = match std::env::var("TELEGRAM_BOT_TOKEN") {
            Ok(t) => t,
            Err(_) => {
                let res = serde_json::json!({ "success": false, "error": "TELEGRAM_BOT_TOKEN not set" });
                return Ok(res.to_string());
            }
        };
        let chat_id = match std::env::var("TELEGRAM_ALLOWED_USERS") {
            Ok(c) => c,
            Err(_) => {
                let res = serde_json::json!({ "success": false, "error": "TELEGRAM_ALLOWED_USERS not set" });
                return Ok(res.to_string());
            }
        };

        let recipient = chat_id.split(',').next().unwrap_or(&chat_id);
        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);

        let client = Client::new();
        match client.post(&url)
            .json(&serde_json::json!({
                "chat_id": recipient,
                "text": message,
                "parse_mode": "Markdown"
            }))
            .send()
            .await
        {
            Ok(res) => {
                if res.status().is_success() {
                    let result = serde_json::json!({ "success": true, "message": "Message sent to Telegram" });
                    Ok(result.to_string())
                } else {
                    let result = serde_json::json!({ "success": false, "error": format!("Telegram API error: {}", res.status()) });
                    Ok(result.to_string())
                }
            },
            Err(e) => {
                let result = serde_json::json!({ "success": false, "error": format!("Network error: {}", e) });
                Ok(result.to_string())
            }
        }
    }
}
