use crate::{Brain, Message};
use tracing::{info, warn};

// Implement ContextCompressor to satisfy trait bound for ContextManager
impl openspore_memory::context::ContextCompressor for Brain {
    fn compress<'a>(&'a self, current: &'a str, new_items: &'a str) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<String>> + Send + 'a>> {
        Box::pin(async move {
            if new_items.is_empty() {
                return Ok(current.to_string());
            }

            let compress_prompt = format!(r#"Compress this session history into a concise summary.

Current Summary:
{}

New Items to Integrate:
{}

Create an updated summary that:
1. Preserves key facts, decisions, and context
2. Removes redundant information
3. Maintains chronological flow
4. Stays under 500 words

Return ONLY the compressed summary, no preamble."#, current, new_items);

            match self.complete(&[Message{role:"user".into(), content: compress_prompt}]).await {
                Ok(compressed) => {
                    info!("📦 Compressed {} chars -> {} chars", current.len() + new_items.len(), compressed.len());
                    Ok(compressed)
                },
                Err(e) => {
                    warn!("Compression failed: {}, keeping current", e);
                    Ok(format!("{}\n\n{}", current, new_items))
                }
            }
        })
    }
}
