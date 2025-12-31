use anyhow::Result;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info};

use super::local::LocalStorage;
use crate::event::MonitorImageEvent;

pub struct Storage {}

impl Storage {
    pub fn run(
        &mut self,
        mut productor: mpsc::Receiver<MonitorImageEvent>,
        consumer: mpsc::Sender<MonitorImageEvent>,
    ) -> Result<()> {
        let local = LocalStorage::new()?;

        let (tx, rx) = mpsc::channel::<MonitorImageEvent>(100);

        // Local Cache
        tokio::spawn(async move {
            while let Some(event) = productor.recv().await {
                let Ok(cached_event) = local.cache(event).await else {
                    error!("Failed to cache event");
                    continue;
                };
                if let Err(e) = tx.send(cached_event).await {
                    error!("Failed to send cached event: {}", e);
                    break;
                }
            }
            info!("Storage task completed");
        });

        // Pack and Upload
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                co
            }
            info!("Pack and Upload task completed");
        });

        Ok(())
    }
}
