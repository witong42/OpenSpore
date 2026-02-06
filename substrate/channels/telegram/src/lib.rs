//! Telegram Channel for OpenSpore
//! Port of opensporejs/src/channels/telegram.js

use teloxide::prelude::*;
use teloxide::types::ParseMode;
use openspore_core::config::AppConfig;
use openspore_brain::Brain;
use tracing::info;
use std::sync::Arc;

#[derive(Clone)]
pub struct TelegramChannel {
    token: String,
    allowed_users: Vec<String>,
}

impl TelegramChannel {
    pub fn new() -> anyhow::Result<Self> {
        let _config = AppConfig::load()?;
        let token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_BOT_TOKEN not set"))?;

        let allowed_users = std::env::var("TELEGRAM_ALLOWED_USERS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(Self {
            token,
            allowed_users,
        })
    }

    /// Send a message to the first allowed user (Heartbeat style)
    pub async fn send_raw(&self, text: &str) -> anyhow::Result<()> {
        let chat_id = self.allowed_users.first()
            .ok_or_else(|| anyhow::anyhow!("No allowed users set"))?;

        Self::send_stateless(text, Some(chat_id)).await
    }

    /// Start the Telegram bot listener (Long Polling)
    pub async fn start(&self) -> anyhow::Result<()> {
        info!("📡 Telegram Gateway Starting...");

        let bot = Bot::new(&self.token);
        let allowed_users = self.allowed_users.clone();

        // Load brain in thread/arc to share
        let config = AppConfig::load()?;
        let brain = Arc::new(Brain::new(config));

        info!("✅ Telegram Gateway Active. Allowed Users: {:?}", allowed_users);

        teloxide::repl(bot, move |bot: Bot, msg: Message| {
            let allowed_users = allowed_users.clone();
            let brain = brain.clone();
            async move {
                let user_id = msg.from.as_ref().map(|u| u.id.to_string()).unwrap_or_default();

                // Security Check
                if !allowed_users.is_empty() && !allowed_users.contains(&user_id) {
                    let _ = bot.send_message(msg.chat.id, "⛔ Access Denied.").await;
                    return Ok(());
                }

                if let Some(text) = msg.text() {
                    let text = text.to_string(); // Own the text for the thread
                    info!("📩 [Telegram] Message from {}: {}", user_id, text);

                    // Show typing action
                    let _ = bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing).await;

                    // Spawn a task so thinking doesn't block the next message
                    tokio::spawn(async move {
                        // Think
                        let response = brain.think(&text).await;

                        // Split and send
                        for chunk in split_message(&response, 4000) {
                            // Try sending with MarkdownV2, fallback to plain text if it fails
                            if let Err(_) = bot.send_message(msg.chat.id, chunk)
                                .parse_mode(ParseMode::MarkdownV2)
                                .await
                            {
                                let _ = bot.send_message(msg.chat.id, chunk).await;
                            }
                        }
                    });
                }
                Ok(())
            }
        })
        .await;

        Ok(())
    }

    /// Stateless Send - Send a message without starting a listener (for cron/notifications)
    pub async fn send_stateless(text: &str, target_id: Option<&str>) -> anyhow::Result<()> {
        let token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_BOT_TOKEN not set"))?;

        let chat_id = if let Some(id) = target_id {
            id.to_string()
        } else {
            std::env::var("TELEGRAM_ALLOWED_USERS")
                .ok()
                .and_then(|s| s.split(',').next().map(|id| id.trim().to_string()))
                .ok_or_else(|| anyhow::anyhow!("No target user ID provided and TELEGRAM_ALLOWED_USERS not set"))?
        };

        let bot = Bot::new(token);
        info!("📡 Telegram: Sending message to chat_id: {}", chat_id);

        for chunk in split_message(text, 4000) {
            match bot.send_message(chat_id.clone(), chunk)
                .parse_mode(ParseMode::MarkdownV2)
                .await
            {
                Ok(_) => info!("✅ Telegram: Message sent successfully."),
                Err(e) => {
                    info!("⚠️ Telegram: MarkdownV2 failed, retrying with plain text. Error: {}", e);
                    if let Err(e2) = bot.send_message(chat_id.clone(), chunk).await {
                        info!("❌ Telegram: Final fallback failed. Error: {}", e2);
                        return Err(anyhow::anyhow!("Failed to send Telegram message: {}", e2));
                    } else {
                        info!("✅ Telegram: Fallback message sent successfully.");
                    }
                }
            }
        }

        Ok(())
    }
}

fn split_message(text: &str, max_length: usize) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut current = text;
    while current.len() > max_length {
        let (chunk, rest) = current.split_at(max_length);
        chunks.push(chunk);
        current = rest;
    }
    chunks.push(current);
    chunks
}
