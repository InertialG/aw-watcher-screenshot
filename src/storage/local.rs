use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, broadcast};
use tokio::task::JoinHandle;
use directories::ProjectDirs;
use anyhow::Result;
use std::fs;
use tracing::{info, error};
use uuid::Uuid;

use crate::capture::event::MonitorImageEvent;

// type Consumer = mpsc::Sender<MonitorImageEvent>;

pub struct LocalStorage {
    productor: mpsc::Receiver<MonitorImageEvent>,
    stopper: broadcast::Sender<bool>,
    consumer: mpsc::Sender<MonitorImageEvent>,

    handler: Option<JoinHandle<()>>,
    cache_path: PathBuf,
    pending_path: PathBuf,
}

impl LocalStorage {
    pub fn new(productor: mpsc::Receiver<MonitorImageEvent>, stopper: broadcast::Sender<bool>, consumer: mpsc::Sender<MonitorImageEvent>) -> Result<Self> {
        // 1. 获取项目目录
        let project_dirs = ProjectDirs::from("uno", "guan810", "aw-watcher-screenshot")
            .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

        // 2. 选择基础路径（建议使用 data_dir 存放截图，cache_dir 存放临时文件）
        let base_data_path = project_dirs.data_dir();

        // 3. 逐级创建目录
        let image_path = join_and_create_dir(base_data_path, "images")?;
        let cache_path = join_and_create_dir(&image_path, "cache")?;
        let pending_path = join_and_create_dir(&image_path, "pending")?;

        Ok(LocalStorage {
            productor,
            stopper,
            consumer,
            handler: None,
            cache_path,
            pending_path,
        })
    }

    pub async fn run(&mut self) {
        let mut stopper = self.stopper.subscribe();
        let mut productor = self.productor.clone();
        self.handler = Some(tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stopper.recv() => {
                        info!("Local cache task stopping...");
                        break;
                    }

                    event = productor.recv() => {
                        if let Some(event) = event {
                            let timestamp = event.timestamp();
                            let id = Uuid::new_v7();
                        } else {
                            error!("Productor channel closed unexpectedly");
                        }
                    }
                }
            }
        }));
    }
}


fn join_and_create_dir<P: AsRef<Path>>(path: P, sub_path: &str) -> Result<PathBuf> {
    let full_path = path.as_ref().join(sub_path);
    if !full_path.exists() {
        fs::create_dir_all(&full_path)?;
    }
    Ok(full_path)
}
