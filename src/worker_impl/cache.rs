use crate::event::ImageEvent;
use crate::worker::TaskProcessor;
use anyhow::{Context, Error, Result};
use image::ImageFormat;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ToWebpProcessor {
    cache_path: Option<PathBuf>,
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
            let mut buffer = Cursor::new(Vec::new());
            let id = event.get_id();
            image
                .write_to(&mut buffer, ImageFormat::WebP)
                .context("Failed to encode image to WebP")?;
            let webp_bytes = Arc::new(buffer.into_inner());

            let file_path = cache_path.join(format!("{}--{}.webp", &id, &key));
            fs::write(file_path, &*webp_bytes)?;

            event.add_data(key.clone(), webp_bytes);
        }

        Ok(event)
    }
}

use crate::config::CacheConfig;

impl ToWebpProcessor {
    pub fn new(config: CacheConfig) -> Self {
        let cache_path = Some(PathBuf::from(config.cache_dir));
        Self { cache_path }
    }
}
