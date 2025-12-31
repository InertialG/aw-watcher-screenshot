use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use image::{DynamicImage, ImageFormat};
use std::sync::Arc;
use tracing_subscriber::registry::Data;
use uuid::{NoContext, Timestamp, Uuid};

pub struct MonitorImageEvent {
    monitor_id: String,
    image: Arc<DynamicImage>,
    timestamp: DateTime<Utc>,
    id: u128,
}

impl MonitorImageEvent {
    pub fn new(monitor_id: String, image: DynamicImage, timestamp: DateTime<Utc>) -> Self {
        let id = Uuid::new_v7(Timestamp::from_unix(
            NoContext,
            timestamp.timestamp() as u64,
            timestamp.timestamp_subsec_nanos(),
        ))
        .to_u128_le();
        MonitorImageEvent {
            monitor_id,
            image: Arc::new(image),
            timestamp,
            id,
        }
    }

    pub fn image(&self) -> Arc<DynamicImage> {
        self.image.clone()
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    pub fn filename(&self) -> String {
        format!("{}.jpg", self.id)
    }

    pub fn to_webp(&self) -> Result<Vec<u8>> {
        // 1. 直接使用原始图片，不再 Resize
        // 这样保留了 100% 的像素细节，对 VLM 的 OCR 极其友好

        let mut buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buffer);

        // 2. 编码为 WebP
        // 这里的开销主要在编码计算上，不仅保留了画质，逻辑也更简单
        self.image
            .write_to(&mut cursor, ImageFormat::WebP)
            .context("Failed to encode original image to WebP")?;

        Ok(buffer)
    }
}

pub struct UploadEvent {
    pub id: u128,
    pub data: Vec<u8>,
    pub timestamp: DateTime<Utc>,
}

impl UploadEvent {
    pub fn new(id: u128, data: Vec<u8>, timestamp: DateTime<Utc>) -> Self {
        UploadEvent {
            id,
            data,
            timestamp,
        }
    }
}
