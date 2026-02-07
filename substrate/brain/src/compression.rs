use crate::{Brain, Message};
use tracing::{info, warn};

// Implement ContextCompressor to satisfy trait bound for ContextManager
impl openspore_memory::context::ContextCompressor for Brain {
    fn compress<'a>(&'a self, current: &'a str, new_items: &'a str) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<String>> + Send + 'a>> {
        Box::pin(async move {
            if new_items.is_empty() {
                return Ok(current.to_string());
            }

            let compress_prompt = format!(r#"Synthesize a new, concise session summary from the existing state and the latest interactions.

PREVIOUS SESSION STATE:
{}

NEW INTERACTIONS TO SYNTHESIZE:
{}

CONSTRUCT A REPLACEMENT SUMMARY THAT:
1. Preserves only high-leverage facts and decisions
2. Discards minor conversational filler or resolved issues
3. STRICTLY stays under 400 words

Return only the new summary text."#, current, new_items);

             match self.complete(&[Message{role:"user".into(), content: compress_prompt}]).await {
                 Ok(compressed) => {
                     info!("📦 Compressed {} chars -> {} chars", current.len() + new_items.len(), compressed.len());
                     Ok(compressed)
                 },
                 Err(e) => {
                    warn!("Context compression failed: {}. Falling back to existing summary to prevent system bloat.", e);
                    Ok(current.to_string())
                 }
             }
        })
    }
}
