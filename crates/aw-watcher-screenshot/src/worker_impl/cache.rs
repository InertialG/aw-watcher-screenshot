use crate::event::{CaptureEvent, ImageEvent, ImageInfo};
use crate::worker::Processor;
use anyhow::{Context, Error, Result};
use async_trait::async_trait;
use std::fs;
use std::path::PathBuf;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use webp::Encoder;

pub struct ToWebpProcessor {
    cache_path: PathBuf,
    webp_quality: f32,
}

#[async_trait]
impl Processor<CaptureEvent, ImageEvent> for ToWebpProcessor {
    async fn process(
        mut self,
        rx: Receiver<CaptureEvent>,
        tx: Sender<ImageEvent>,
    ) -> Result<JoinHandle<()>, Error> {
        Ok(tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let mut image_event = ImageEvent::new(event.timestamp, self.cache_path.clone());

                for (key, image_info) in event.image_iter() {
                    let image_data = image_info
                        .payload
                        .as_ref()
                        .context("Image data not found")?;

                    let webp_bytes = tokio::task::spawn_blocking(move || {
                        let encoder = Encoder::from_image(image_data)
                            .map_err(|e| anyhow::anyhow!("Failed to create WebP encoder: {}", e))?;

                        let webp_data = if self.webp_quality >= 100.0 {
                            encoder.encode_lossless()
                        } else {
                            encoder.encode(self.webp_quality)
                        };

                        webp_data.to_vec()
                    })
                    .await
                    .context("Failed to encode image to WebP")?;

                    // Path format: {yyyy/mm/dd}/{hh}/{timestamp}_{device_hash}.webp
                    let file_dir = self.cache_path.join(event.get_path_subdir());
                    if !file_dir.exists() {
                        fs::create_dir_all(&file_dir)?;
                    }
                    let file_path = file_dir.join(format!(
                        "{}_{}.webp",
                        event.timestamp.format("%Y%m%d_%H%M%S%3f"),
                        key
                    ));
                    fs::write(&file_path, &webp_bytes)?;

                    let mut new_image_info = ImageInfo::from_base_info(&image_info);
                    new_image_info.set_payload(webp_bytes);
                    image_event.add_data(new_image_info);
                }
                tx.send(image_event).await?;
            }
        }))
    }
}

use crate::config::CacheConfig;

impl ToWebpProcessor {
    pub fn new(config: CacheConfig) -> Self {
        let cache_path = PathBuf::from(config.cache_dir);

        if !cache_path.exists() {
            fs::create_dir_all(&cache_path)?;
        }
        Self {
            cache_path,
            webp_quality: config.webp_quality as f32,
        }
    }
}
