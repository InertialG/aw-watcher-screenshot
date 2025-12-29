use chrono::{DateTime, Utc};
use image::DynamicImage;

pub struct MonitorImageEvent {
    monitor_id: String,
    image: DynamicImage,
    timestamp: DateTime<Utc>,
}

impl MonitorImageEvent {
    pub fn new(monitor_id: String, image: DynamicImage, timestamp: DateTime<Utc>) -> Self {
        MonitorImageEvent {
            monitor_id,
            image,
            timestamp,
        }
    }

    pub fn image(&self) -> &DynamicImage {
        &self.image
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}
