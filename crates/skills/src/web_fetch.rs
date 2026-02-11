//! Web Fetch Skill (Core)

use super::Skill;
use async_trait::async_trait;
use reqwest::Client;

pub struct WebFetchSkill;

#[async_trait]
impl Skill for WebFetchSkill {
    fn name(&self) -> &'static str { "web_fetch" }

    fn description(&self) -> &'static str {
        "Fetch content from a URL. Returns JSON with success, status_code, and content. Usage: [WEB_FETCH: \"https://example.com\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let url = args.trim().trim_matches('"').trim_matches('\'');

        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_else(|_| Client::new());

        match client.get(url).send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                match response.text().await {
                    Ok(body) => {
                        let full_length = body.len();
                        let truncated = if full_length > 10000 {
                            format!("{}...\n[Truncated {} bytes]", &body[..10000], full_length - 10000)
                        } else {
                            body
                        };

                        let res = serde_json::json!({
                            "success": true,
                            "status_code": status,
                            "content": truncated,
                            "full_length": full_length,
                            "url": url
                        });
                        Ok(res.to_string())
                    },
                    Err(e) => {
                        let res = serde_json::json!({
                            "success": false,
                            "error": format!("Failed to read body: {}", e),
                            "status_code": status,
                            "url": url
                        });
                        Ok(res.to_string())
                    }
                }
            },
            Err(e) => {
                let res = serde_json::json!({
                    "success": false,
                    "error": format!("Request failed: {}", e),
                    "url": url
                });
                Ok(res.to_string())
            }
        }
    }
}
