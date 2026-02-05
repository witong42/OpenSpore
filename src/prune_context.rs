use openspore_memory::context::ContextManager;
use openspore_core::state::AppState;
use std::sync::Arc;


async fn prune_context_task() -> Result<(), Box<dyn std::error::Error>> {
    let app_state = AppState::load().await.map_err(|e| e.to_string())?;
    let context_manager = ContextManager::new(&app_state);
    context_manager.prune_older_items().await.map_err(|e| e.to_string())?;
    Ok(())
}


#[tokio::main]
async fn main() {
    if let Err(e) = prune_context_task().await {
        eprintln!(\"Error during context pruning: {}\", e);
    }
}
