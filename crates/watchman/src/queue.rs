use crate::{Watchman, types::WatchEvent};
use std::path::PathBuf;
use tracing::{info, error};

impl Watchman {
    /// Enqueue a file event for processing
    pub(crate) async fn enqueue(&self, event_type: &str, file_path: PathBuf) {
        if self.memory.is_internal_write(&file_path).await {
            info!("👀 Watchman: Ignoring internal write to {:?}", file_path);
            return;
        }

        if self.should_ignore(&file_path) {
            return;
        }

        info!("👀 Watchman detected {}: {:?}", event_type, file_path);

        let mut queue = self.queue.lock().await;
        queue.push(WatchEvent {
            event_type: event_type.to_string(),
            file_path,
        });
    }

    /// Process queued events - trigger learning
    pub async fn process_queue(&self) {
        let events: Vec<WatchEvent> = {
            let mut queue = self.queue.lock().await;
            std::mem::take(&mut *queue)
        };

        for event in events {
            if let Err(e) = self.process_event(&event).await {
                error!("Watchman Error processing {:?}: {}", event.file_path, e);
            }
        }
    }
}
