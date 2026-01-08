use chrono::{DateTime, Utc};
use image::DynamicImage;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct CaptureEvent {
    pub images: HashMap<u32, Arc<DynamicImage>>,
    pub monitors: HashMap<u32, UploadImageInfo>,
    pub timestamp: DateTime<Utc>,
}

impl CaptureEvent {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
            monitors: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    pub fn add_image(
        &mut self,
        monitor_id: u32,
        image_info: DynamicImage,
        monitor_info: UploadImageInfo,
    ) {
        self.images.insert(monitor_id, Arc::new(image_info));
        self.monitors.insert(monitor_id, monitor_info);
    }
}

pub type WebpImage = Vec<u8>;

pub struct ImageEvent {
    pub datas: HashMap<u32, Arc<WebpImage>>,
    pub monitors: HashMap<u32, UploadImageInfo>,
    pub timestamp: DateTime<Utc>,
    pub local_dir: PathBuf,
}

impl ImageEvent {
    pub fn new(
        timestamp: DateTime<Utc>,
        local_dir: PathBuf,
        monitors: HashMap<u32, UploadImageInfo>,
    ) -> Self {
        Self {
            datas: HashMap::new(),
            timestamp,
            local_dir,
            monitors,
        }
    }

    pub fn add_data(&mut self, monitor_id: u32, image_info: WebpImage) {
        self.datas.insert(monitor_id, Arc::new(image_info));
    }
}

#[derive(Serialize, Clone)]
pub struct UploadImageInfo {
    pub monitor_name: String,
    pub monitor_id: u32,
    pub object_key: String,
    pub uploaded: bool,
}

impl UploadImageInfo {
    pub fn new(monitor_name: String, monitor_id: u32, object_key: String) -> Self {
        Self {
            monitor_name,
            monitor_id,
            object_key,
            uploaded: false,
        }
    }
}

impl From<UploadImageInfo> for Value {
    fn from(upload: UploadImageInfo) -> Self {
        serde_json::to_value(upload).unwrap_or(Value::Null)
    }
}

#[derive(Serialize, Clone)]
pub struct UploadS3Info {
    pub endpoint: String,
    pub bucket: String,
    pub prefix: Option<String>,
}

impl UploadS3Info {
    pub fn new(endpoint: String, bucket: String, prefix: Option<String>) -> Self {
        Self {
            endpoint,
            bucket,
            prefix,
        }
    }
}

impl From<UploadS3Info> for Value {
    fn from(upload: UploadS3Info) -> Self {
        serde_json::to_value(upload).unwrap_or(Value::Null)
    }
}

pub struct AwEvent {
    pub datas: HashMap<u32, UploadImageInfo>,
    pub timestamp: DateTime<Utc>,
    pub local_dir: PathBuf,
    pub s3_info: Option<UploadS3Info>,
}

impl AwEvent {
    pub fn new(
        timestamp: DateTime<Utc>,
        local_dir: PathBuf,
        s3_info: Option<UploadS3Info>,
    ) -> Self {
        Self {
            datas: HashMap::new(),
            timestamp,
            local_dir,
            s3_info,
        }
    }

    pub fn _get_data(&self, key: u32) -> Option<&UploadImageInfo> {
        self.datas.get(&key)
    }

    pub fn add_data(&mut self, key: u32, upload_info: UploadImageInfo) {
        self.datas.insert(key, upload_info);
    }

    pub fn set_uploaded(&mut self, key: u32) {
        let upload_info = self.datas.get_mut(&key);
        if let Some(upload_info) = upload_info {
            upload_info.uploaded = true;
        }
    }
}
