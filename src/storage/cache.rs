use anyhow::{Context, Result};
use directories::ProjectDirs;
use image::{DynamicImage, ImageFormat};
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc};
use uuid::{NoContext, Timestamp, Uuid};

use crate::event::{MonitorImageEvent, UploadEvent};

type WebpImage = Vec<u8>;

pub fn run_processers(productor: mpsc::Receiver<MonitorImageEvent>) {
    let (tx1, rx1) = mpsc::channel::<UploadEvent>(10);
    let (tx2, rx2) = mpsc::channel::<UploadEvent>(10);

    // Step1: to webp and send Arc<UploadEvent>
    tokio::spawn(async move {
        let semaphore = Arc::new(Semaphore::new(8));
        while let Some(event) = productor.recv().await {
            let permit = semaphore.clone().acquire_owned().await?;
            let tx1_clone = tx1.clone();
            let event = tokio::spawn(async move {
                let _permit = permit;
                match tokio::task::spawn_blocking(move || event.to_webp()).await {
                    Ok(Ok(image)) => {
                        tx1_clone
                            .send(UploadEvent::new(event.id, image, event.timestamp))
                            .await?;
                    }
                    Ok(Err(err)) => {
                        eprintln!("Failed to convert image to webp: {}", err);
                    }
                    Err(err) => {
                        eprintln!("Failed to spawn blocking task: {}", err);
                    }
                }
            });
        }
    });

    // Step2: local cache and send Arc<UploadEvent>
    tokio::spawn(async move {
        let project_dirs = ProjectDirs::from("uno", "guan810", "aw-watcher-screenshot")
            .context("Failed to get project directories")?;

        // 2. 选择基础路径（建议使用 data_dir 存放截图，cache_dir 存放临时文件）
        let cache_path = project_dirs.data_dir().join("images").join("cache");
        if !cache_path.exists() {
            fs::create_dir_all(&cache_path)?;
        }
        while let Some(event) = rx1.recv().await {
            let image = event.data;
        }
    });
}

fn init_cache_dir() -> Result<()> {
    let project_dirs = ProjectDirs::from("uno", "guan810", "aw-watcher-screenshot")
        .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

    // 2. 选择基础路径（建议使用 data_dir 存放截图，cache_dir 存放临时文件）
    let cache_path = project_dirs.data_dir().join("images").join("cache");
    if !cache_path.exists() {
        fs::create_dir_all(&cache_path)?;
    }
    Ok(())
}
