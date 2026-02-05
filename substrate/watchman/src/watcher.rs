use crate::Watchman;
use std::sync::Arc;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use tracing::info;

impl Watchman {
    /// Start the filesystem watcher
    pub async fn start(self: Arc<Self>) -> anyhow::Result<()> {
        info!("👀 Watchman: Starting filesystem observation at {:?}", self.project_root);

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let watchman = self.clone();

        // Spawn watcher in blocking thread
        let project_root = self.project_root.clone();
        std::thread::spawn(move || {
            let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            }).expect("Failed to create watcher");

            watcher.watch(&project_root, RecursiveMode::Recursive)
                .expect("Failed to watch directory");

            // Keep watcher alive
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            }
        });

        info!("👀 Watchman: Ready and watching.");

        // Process events
        while let Some(event) = rx.recv().await {
            match event.kind {
                EventKind::Create(_) => {
                    for path in event.paths {
                        watchman.enqueue("add", path).await;
                    }
                }
                EventKind::Modify(_) => {
                    for path in event.paths {
                        watchman.enqueue("change", path).await;
                    }
                }
                _ => {}
            }

            // Process queue after each batch
            watchman.process_queue().await;
        }

        Ok(())
    }
}
