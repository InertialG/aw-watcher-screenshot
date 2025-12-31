use anyhow::Result;
use directories::ProjectDirs;
use image::ImageFormat;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info};
use uuid::{NoContext, Timestamp, Uuid};

use crate::event::MonitorImageEvent;

pub struct LocalStorage {
    cache_path: PathBuf,
}

impl LocalStorage {
    pub fn new() -> Result<Self> {
        // 1. 获取项目目录
        let project_dirs = ProjectDirs::from("uno", "guan810", "aw-watcher-screenshot")
            .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

        // 2. 选择基础路径（建议使用 data_dir 存放截图，cache_dir 存放临时文件）
        let cache_path = project_dirs.data_dir().join("images").join("cache");
        if !cache_path.exists() {
            fs::create_dir_all(&cache_path)?;
        }

        Ok(LocalStorage { cache_path })
    }

    pub async fn cache(&self, event: MonitorImageEvent) -> Result<MonitorImageEvent> {
        // Implement caching logic here
        let time = event.timestamp();
        let id = Uuid::new_v7(Timestamp::from_unix(
            NoContext,
            time.timestamp() as u64,
            time.timestamp_subsec_nanos(),
        ))
        .to_u128_le();
        let path = self.cache_path.join(format!("{}.jpg", id));
        let image = event.image();

        let save_result =
            tokio::task::spawn_blocking(move || image.save_with_format(&path, ImageFormat::Jpeg))
                .await;

        match save_result {
            Ok(Ok(_)) => info!(
                "Image {} saved successfully to {}",
                id,
                self.cache_path.display()
            ),
            Ok(Err(e)) => error!("Error saving image: {}", e),
            Err(e) => error!("Error saving image: {}", e),
        }

        Ok(event.set_id(id))
    }
}
