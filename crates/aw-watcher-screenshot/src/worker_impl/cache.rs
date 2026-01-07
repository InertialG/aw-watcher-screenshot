use crate::event::{CaptureEvent, ImageEvent};
use crate::worker::Processor;
use anyhow::{Error, Result};
use futures::future::join_all;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::{error, info};
use webp::Encoder;

pub struct ToWebpProcessor {
    cache_dir: PathBuf,
    webp_quality: f32,
}

impl Processor<CaptureEvent, ImageEvent> for ToWebpProcessor {
    fn process(
        self,
        mut rx: Receiver<CaptureEvent>,
        tx: Sender<ImageEvent>,
    ) -> Result<JoinHandle<()>, Error> {
        let cache_dir = self.cache_dir.clone();
        let webp_quality = self.webp_quality;

        Ok(tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                info!("ToWebpProcessor: processing {} images", event.images.len());

                // Compute cache path based on event timestamp
                let cache_path = cache_dir.join(event.timestamp.format("%Y/%m/%d/%H").to_string());

                if !cache_path.exists() {
                    if let Err(e) = fs::create_dir_all(&cache_path) {
                        error!("Failed to create cache directory: {}", e);
                        continue;
                    }
                }

                let cache_path = Arc::new(cache_path);
                let mut cache_futures = Vec::new();

                for (key, image_data) in event.images.iter() {
                    let cache_path = cache_path.clone();
                    let image_data = image_data.clone();
                    let key = *key;
                    let timestamp = event.timestamp;

                    let cache_task = async move {
                        let encoder = Encoder::from_image(&*image_data)
                            .map_err(|e| anyhow::anyhow!("Failed to create WebP encoder: {}", e))?;

                        let webp_data = if webp_quality >= 100.0 {
                            encoder.encode_lossless()
                        } else {
                            encoder.encode(webp_quality)
                        };

                        let file_path = cache_path.join(format!(
                            "{}_{}.webp",
                            timestamp.format("%Y%m%d_%H%M%S%3f"),
                            key
                        ));

                        fs::write(&file_path, &*webp_data)?;
                        info!("ToWebpProcessor: saved {}", file_path.display());

                        Ok::<_, Error>((key, webp_data.to_vec()))
                    };

                    cache_futures.push(cache_task);
                }

                let mut image_event =
                    ImageEvent::new(event.timestamp, cache_path.to_path_buf(), event.monitors);

                let results: Vec<Result<_, Error>> = join_all(cache_futures).await;

                for result in results {
                    match result {
                        Ok((key, webp_data)) => image_event.add_data(key, webp_data),
                        Err(e) => error!("Failed to cache image: {}", e),
                    }
                }

                if let Err(e) = tx.send(image_event).await {
                    error!("Failed to send image event: {}", e);
                    break;
                }
            }
            info!("ToWebpProcessor finished");
        }))
    }
}

use crate::config::CacheConfig;

impl ToWebpProcessor {
    pub fn new(config: CacheConfig) -> Result<Self, Error> {
        let cache_dir = PathBuf::from(config.cache_dir);

        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }
        Ok(Self {
            cache_dir,
            webp_quality: config.webp_quality as f32,
        })
    }
}
