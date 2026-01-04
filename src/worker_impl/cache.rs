use crate::event::ImageEvent;
use crate::worker::TaskProcessor;
use anyhow::{Context, Error, Result};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use webp::Encoder;

pub struct ToWebpProcessor {
    cache_path: Option<PathBuf>,
    webp_quality: f32,
}

impl TaskProcessor<ImageEvent, ImageEvent> for ToWebpProcessor {
    fn init(&mut self) -> Result<(), Error> {
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

    fn process(&mut self, mut event: ImageEvent) -> Result<ImageEvent, Error> {
        let cache_path = self
            .cache_path
            .as_ref()
            .context("Cache path not initialized")?;

        for (key, image) in event.image_iter() {
            let id = event.get_id();

            // Use webp crate for faster encoding with quality control
            let encoder = Encoder::from_image(&image)
                .map_err(|e| anyhow::anyhow!("Failed to create WebP encoder: {}", e))?;

            let webp_data = if self.webp_quality >= 100.0 {
                encoder.encode_lossless()
            } else {
                encoder.encode(self.webp_quality)
            };

            let webp_bytes = Arc::new(webp_data.to_vec());

            let file_path = cache_path.join(format!("{}--{}.webp", &id, &key));
            fs::write(&file_path, &*webp_bytes)?;

            event.add_data(key.clone(), webp_bytes);
        }

        Ok(event)
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
