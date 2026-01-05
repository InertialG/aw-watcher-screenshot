use chrono::{DateTime, Utc};
use image::DynamicImage;
use std::collections::HashMap;
use std::sync::Arc;

pub type CaptureCommand = bool;

pub struct CaptureEvent {
    pub images: HashMap<String, Arc<DynamicImage>>,
    pub timestamp: DateTime<Utc>,
}

impl CaptureEvent {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    pub fn add_image(&mut self, id: String, image: DynamicImage) {
        self.images.insert(id, Arc::new(image));
    }

    pub fn _get_image(&self, id: &str) -> Option<Arc<DynamicImage>> {
        self.images.get(id).cloned()
    }

    pub fn image_iter(&self) -> Vec<(String, Arc<DynamicImage>)> {
        self.images
            .iter()
            .map(|(k, v)| (k.clone(), Arc::clone(v)))
            .collect()
    }

    pub fn _get_format_timestamp(&self) -> String {
        self.timestamp.format("%Y%m%d_%H%M%S%3f").to_string()
    }

    /// Returns the subdirectory path: {yyyy}/{mm}/{dd}/{hh}
    pub fn get_path_subdir(&self) -> String {
        self.timestamp.format("%Y/%m/%d/%H").to_string()
    }

    /// Returns the filename: {timestamp_millis}_{key}.webp
    pub fn get_filename(&self, key: &str) -> String {
        format!("{}_{}.webp", self.timestamp.timestamp_millis(), key)
    }
}

pub type WebpImage = Vec<u8>;

pub struct ImageEvent {
    pub datas: HashMap<String, Arc<WebpImage>>,
    pub timestamp: DateTime<Utc>,
}

impl ImageEvent {
    pub fn new(timestamp: DateTime<Utc>) -> Self {
        Self {
            datas: HashMap::new(),
            timestamp,
        }
    }

    pub fn add_data(&mut self, id: String, data: Arc<WebpImage>) {
        self.datas.insert(id, data);
    }

    pub fn _data_iter(&self) -> Vec<(String, Arc<WebpImage>)> {
        self.datas
            .iter()
            .map(|(k, v)| (k.clone(), Arc::clone(v)))
            .collect()
    }

    pub fn _get_format_timestamp(&self) -> String {
        self.timestamp.format("%Y%m%d_%H%M%S%3f").to_string()
    }

    /// Returns the subdirectory path: {yyyy}/{mm}/{dd}/{hh}
    pub fn get_path_subdir(&self) -> String {
        self.timestamp.format("%Y/%m/%d/%H").to_string()
    }

    /// Returns the filename: {timestamp_millis}_{key}.webp
    pub fn get_filename(&self, key: &str) -> String {
        format!("{}_{}.webp", self.timestamp.timestamp_millis(), key)
    }

    /// Consumes self and returns (AwEvent, data map)
    pub fn into_parts(self) -> (AwEvent, HashMap<String, Arc<WebpImage>>) {
        let mut aw_event = HashMap::new();
        for (key, _) in self.datas.iter() {
            aw_event.insert(
                key.clone(),
                format!("{}/{}", self.get_path_subdir(), self.get_filename(&key)),
            );
        }
        (
            AwEvent {
                datas: aw_event,
                timestamp: self.timestamp,
            },
            self.datas,
        )
    }
}

pub struct AwEvent {
    pub datas: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

impl AwEvent {
    pub fn _new(timestamp: DateTime<Utc>) -> Self {
        Self {
            datas: HashMap::new(),
            timestamp,
        }
    }

    pub fn get_data(&self, key: &str) -> Option<&String> {
        self.datas.get(key)
    }

    pub fn into_parts(self) -> (DateTime<Utc>, HashMap<String, String>) {
        (self.timestamp, self.datas)
    }
}

pub type CompleteCommand = bool;
