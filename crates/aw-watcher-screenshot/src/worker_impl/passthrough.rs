//! Passthrough processor for when S3 upload is disabled.
//!
//! This module provides a processor that converts ImageEvent to AwEvent
//! without uploading to S3, used when S3 is disabled in configuration.

use crate::event::{AwEvent, ImageEvent};
use crate::worker::Processor;
use anyhow::{Error, Result};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::info;

/// Passthrough processor that converts ImageEvent to AwEvent without S3 upload.
///
/// This processor is used when S3 is disabled. It simply converts the
/// ImageEvent to AwEvent, preserving local file paths but marking
/// images as not uploaded.
pub struct PassthroughProcessor;

impl PassthroughProcessor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PassthroughProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor<ImageEvent, AwEvent> for PassthroughProcessor {
    fn process(
        self,
        mut rx: Receiver<ImageEvent>,
        tx: Sender<AwEvent>,
    ) -> Result<JoinHandle<()>, Error> {
        Ok(tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                info!(
                    images_count = event.datas.len(),
                    "PassthroughProcessor: passing through images (S3 disabled)"
                );

                // Create AwEvent without S3 info
                let mut aw_event = AwEvent::new(event.timestamp, event.local_dir, None);

                // Add all monitor info, marked as not uploaded
                for (key, monitor_info) in event.monitors {
                    aw_event.add_data(key, monitor_info);
                }

                if let Err(e) = tx.send(aw_event).await {
                    info!("PassthroughProcessor: receiver dropped, stopping: {}", e);
                    break;
                }
            }
            info!("PassthroughProcessor finished");
        }))
    }
}
