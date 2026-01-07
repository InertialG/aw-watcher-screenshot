use crate::event::{CaptureEvent, ImageEvent, ImageInfo};
use crate::worker::TaskProcessor;
use anyhow::{Context, Error, Result};
use async_trait::async_trait;
use std::fs;
use std::path::PathBuf;
use webp::Encoder;

pub struct ToWebpProcessor {
    cache_path: Option<PathBuf>,
    webp_quality: f32,
}

#[async_trait]
impl TaskProcessor<CaptureEvent, ImageEvent> for ToWebpProcessor {
    async fn init(&mut self) -> Result<(), Error> {
        let cache_path = if let Some(ref path) = self.cache_path {
            path.clone()
        } else {
            std::env::current_dir()?.join("cache")
        };

        if !cache_path.exists() {
            fs::create_dir_all(&cache_path)?;
        }
        self.cache_path = Some(cache_path);
        Ok(())
    }

    async fn process(&mut self, event: CaptureEvent) -> Result<ImageEvent, Error> {
        let cache_path = self
            .cache_path
            .as_ref()
            .context("Cache path not initialized")?;

        let mut image_event = ImageEvent::new(event.timestamp, cache_path.clone());

        for (key, image_info) in event.image_iter() {
            let image_data = image_info
                .payload
                .as_ref()
                .context("Image data not found")?;

            let encoder = Encoder::from_image(image_data)
                .map_err(|e| anyhow::anyhow!("Failed to create WebP encoder: {}", e))?;

            let webp_data = if self.webp_quality >= 100.0 {
                encoder.encode_lossless()
            } else {
                encoder.encode(self.webp_quality)
            };

            let webp_bytes = webp_data.to_vec();

            // Path format: {yyyy/mm/dd}/{hh}/{timestamp}_{device_hash}.webp
            let file_dir = cache_path.join(event.get_path_subdir());
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
        Ok(image_event)
    }
}

use crate::config::CacheConfig;

impl ToWebpProcessor {
    pub fn new(config: CacheConfig) -> Self {
        let cache_path = Some(PathBuf::from(config.cache_dir));
        Self {
            cache_path,
            webp_quality: config.webp_quality as f32,
        }
    }
}
