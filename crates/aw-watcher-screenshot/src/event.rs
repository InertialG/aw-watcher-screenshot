use anyhow::Error;
use chrono::{DateTime, Utc};
use image::DynamicImage;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use xcap::Monitor;

pub type CaptureCommand = bool;

pub struct ImageInfo<P> {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub payload: Option<P>,
}

impl<P> ImageInfo<P> {
    pub fn new(monitor: &Monitor) -> Result<Self, Error> {
        let re = Regex::new(r"[^a-zA-Z0-9]")?;
        let safe_name = re.replace_all(&monitor.name()?, "").to_string();
        Ok(Self {
            id: monitor.id()?,
            name: safe_name,
            x: monitor.x()?,
            y: monitor.y()?,
            width: monitor.width()?,
            height: monitor.height()?,
            payload: None,
        })
    }

    pub fn from_base_info<Q>(base_info: &ImageInfo<Q>) -> Self {
        Self {
            id: base_info.id,
            name: base_info.name.clone(),
            x: base_info.x,
            y: base_info.y,
            width: base_info.width,
            height: base_info.height,
            payload: None,
        }
    }

    pub fn set_payload(&mut self, payload: P) {
        self.payload = Some(payload);
    }

    pub fn get_friendly_name(&self) -> String {
        let prefix = if self.name.is_empty() {
            format!("Monitor_{}", self.id)
        } else {
            self.name.clone()
        };
        format!(
            "{}_{}_{}_{}_{}",
            prefix, self.width, self.height, self.x, self.y
        )
    }
}

pub struct CaptureEvent {
    pub images: HashMap<u32, Arc<ImageInfo<DynamicImage>>>,
    pub timestamp: DateTime<Utc>,
}

impl CaptureEvent {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    pub fn add_image(&mut self, image_info: ImageInfo<DynamicImage>) {
        self.images.insert(image_info.id, Arc::new(image_info));
    }

    pub fn _get_image(&self, id: u32) -> Option<Arc<ImageInfo<DynamicImage>>> {
        self.images.get(&id).cloned()
    }

    pub fn image_iter(&self) -> Vec<(u32, Arc<ImageInfo<DynamicImage>>)> {
        self.images
            .iter()
            .map(|(k, v)| (*k, Arc::clone(v)))
            .collect()
    }

    /// Returns the subdirectory path: {yyyy}/{mm}/{dd}/{hh}
    pub fn get_path_subdir(&self) -> String {
        self.timestamp.format("%Y/%m/%d/%H").to_string()
    }
}

pub type WebpImage = Vec<u8>;

pub struct ImageEvent {
    pub datas: HashMap<u32, Arc<ImageInfo<WebpImage>>>,
    pub timestamp: DateTime<Utc>,
    pub local_dir: PathBuf,
}

impl ImageEvent {
    pub fn new(timestamp: DateTime<Utc>, local_dir: PathBuf) -> Self {
        Self {
            datas: HashMap::new(),
            timestamp,
            local_dir,
        }
    }

    pub fn add_data(&mut self, image_info: ImageInfo<WebpImage>) {
        self.datas.insert(image_info.id, Arc::new(image_info));
    }

    pub fn get_format_timestamp(&self) -> String {
        self.timestamp.format("%Y%m%d_%H%M%S%3f").to_string()
    }

    /// Returns the subdirectory path: {yyyy}/{mm}/{dd}/{hh}
    pub fn get_path_subdir(&self) -> String {
        self.timestamp.format("%Y/%m/%d/%H").to_string()
    }

    /// Consumes self and returns (AwEvent, data map)
    pub fn into_parts(
        self,
        s3_info: UploadS3Info,
    ) -> (AwEvent, HashMap<u32, Arc<ImageInfo<WebpImage>>>) {
        let timestamp = self.timestamp;
        let path_subdir = self.get_path_subdir();
        let mut aw_event = AwEvent::new(timestamp, self.local_dir, Some(s3_info));
        let mut datas = HashMap::new();

        for (key, image_info) in self.datas {
            let object_key = format!(
                "{}{}",
                path_subdir,
                format!("{}_{}.webp", timestamp.timestamp_millis(), key)
            );
            let upload_info = UploadImageInfo::new(image_info.get_friendly_name(), key, object_key);
            aw_event.add_data(key, upload_info);
            datas.insert(key, image_info);
        }

        (aw_event, datas)
    }
}

#[derive(Serialize, Clone)]
pub struct UploadImageInfo {
    pub monitor_name: String,
    pub monitor_id: u32,
    pub object_key: String,
}

impl UploadImageInfo {
    pub fn new(monitor_name: String, monitor_id: u32, object_key: String) -> Self {
        Self {
            monitor_name,
            monitor_id,
            object_key,
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
    pub prefix: String,
}

impl UploadS3Info {
    pub fn new(endpoint: String, bucket: String, prefix: String) -> Self {
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

    pub fn get_data(&self, key: u32) -> Option<&UploadImageInfo> {
        self.datas.get(&key)
    }

    pub fn add_data(&mut self, key: u32, upload_info: UploadImageInfo) {
        self.datas.insert(key, upload_info);
    }
}

pub type CompleteCommand = bool;
