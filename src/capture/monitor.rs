use anyhow::Result;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use image::DynamicImage;
use xcap::Monitor;

use super::event::MonitorImageEvent;
use super::utils;

pub struct SafeMonitor {
    id: String,
    x: i32,
    y: i32,
    last_time: Option<DateTime<Utc>>,
    last_hash: Option<u64>,
}

impl SafeMonitor {
    pub fn new(monitor: &Monitor) -> Result<Self> {
        let x = monitor.x()?;
        let y = monitor.y()?;

        let id = format!(
            "{}_{}_{}_{}_{}",
            monitor.name()?,
            monitor.width()?,
            monitor.height()?,
            x,
            y
        );

        Ok(Self {
            id,
            x,
            y,
            last_time: None,
            last_hash: None,
        })
    }

    pub async fn get_next_event(&mut self) -> Result<Option<MonitorImageEvent>> {
        // 1. 截图 (复用之前封装的异步捕获)
        let raw_img = self.capture_image().await?;

        // 2. 校验并决定是否构建事件
        // 注意：这里逻辑很清晰，没通过校验就返回 Ok(None)
        Ok(self.try_create_event(raw_img))
    }

    pub async fn capture_image(&self) -> Result<DynamicImage> {
        let (x, y) = (self.x, self.y);

        // 在内部处理线程池切换
        let image_res = tokio::task::spawn_blocking(move || {
            Monitor::from_point(x, y)
                .map_err(|e| anyhow!("Failed to find monitor at ({}, {}): {}", x, y, e))?
                .capture_image()
                .map(DynamicImage::ImageRgba8)
                .map_err(|e| anyhow!("Capture error: {}", e))
        })
        .await;

        // 处理 JoinError 和 业务 Error
        match image_res {
            Ok(result) => result,
            Err(e) => Err(anyhow!("Task join error: {}", e)),
        }
    }

    /// 尝试根据新图创建事件。如果不需要发送，返回 None
    pub fn try_create_event(&mut self, image: DynamicImage) -> Option<MonitorImageEvent> {
        let now = Utc::now();
        let current_hash = utils::dhash(&image);

        // 提前返回：如果太相似，直接返回 None
        if let (Some(lt), Some(lh)) = (self.last_time, self.last_hash) {
            let duration = now.signed_duration_since(lt).num_milliseconds();
            let distance = utils::hamming_distance(lh, current_hash);
            if duration < 5000 && distance < 10 {
                return None;
            }
        }

        // 只有在确定要发送时，才更新内部状态并封装对象
        self.last_time = Some(now);
        self.last_hash = Some(current_hash);

        Some(MonitorImageEvent::new(self.id.clone(), image, now))
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }
}
