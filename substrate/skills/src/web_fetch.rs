//! Web Fetch Skill (Core)

use super::Skill;
use async_trait::async_trait;
use reqwest::Client;

pub struct WebFetchSkill;

#[async_trait]
impl Skill for WebFetchSkill {
    fn name(&self) -> &'static str { "web_fetch" }

    fn description(&self) -> &'static str {
        "Fetch content from a URL. Usage: [WEB_FETCH: \"https://example.com\"]"
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let url = args.trim().trim_matches('"').trim_matches('\'');

        let client = Client::new();
        let response = client.get(url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        let body = response.text()
            .await
            .map_err(|e| format!("Failed to read body: {}", e))?;

        let truncated = if body.len() > 5000 {
            format!("{}...\n[Truncated {} bytes]", &body[..5000], body.len() - 5000)
        } else {
            body
        };

        Ok(format!("Status: {}\n\n{}", status, truncated))
    }
}
