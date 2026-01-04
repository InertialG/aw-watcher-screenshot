use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use image::DynamicImage;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use xcap::Window;

pub struct CaptureCommand {
    pub reason: String,
}

impl CaptureCommand {
    pub fn new(reason: String) -> Self {
        Self { reason }
    }
}

pub type WebpImage = Vec<u8>;
pub struct ImageEvent {
    pub id: Uuid,
    pub images: HashMap<String, Arc<DynamicImage>>,
    pub datas: HashMap<String, Arc<WebpImage>>,
    pub file_paths: HashMap<String, String>, // monitor_id -> local file path
    pub timestamp: DateTime<Utc>,
    pub focus_window: Option<FocusWindow>,
}

pub struct FocusWindow {
    pub title: String,
    pub id: u32,
    pub app_name: String,
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub current_monitor: u32,
}

impl ImageEvent {
    pub fn new() -> Self {
        Self {
            id: Uuid::now_v7(),
            images: HashMap::new(),
            datas: HashMap::new(),
            file_paths: HashMap::new(),
            timestamp: Utc::now(),
            focus_window: None,
        }
    }

    pub fn set_focus_window(&mut self, focus_window: Option<FocusWindow>) -> &mut Self {
        self.focus_window = focus_window;
        self
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn add_image(&mut self, id: String, image: DynamicImage) -> &mut Self {
        self.images.insert(id, Arc::new(image));
        self
    }

    pub fn get_image(&self, id: &str) -> Option<Arc<DynamicImage>> {
        self.images.get(id).cloned()
    }

    pub fn get_data(&self, id: &str) -> Option<Arc<WebpImage>> {
        self.datas.get(id).cloned()
    }

    pub fn add_data(&mut self, id: String, data: Arc<WebpImage>) -> &mut Self {
        self.datas.insert(id, data);
        self
    }

    pub fn image_iter(&self) -> Vec<(String, Arc<DynamicImage>)> {
        self.images
            .iter()
            .map(|(k, v)| (k.clone(), Arc::clone(v)))
            .collect()
    }

    pub fn data_iter(&self) -> Vec<(String, Arc<WebpImage>)> {
        self.datas
            .iter()
            .map(|(k, v)| (k.clone(), Arc::clone(v)))
            .collect()
    }
}

impl FocusWindow {
    pub fn new(window: Window) -> Result<Self, Error> {
        Ok(Self {
            title: window.title()?,
            id: window.id()?,
            app_name: window.app_name()?,
            position: (window.x()?, window.y()?),
            size: (window.width()?, window.height()?),
            current_monitor: window.current_monitor()?.id()?,
        })
    }
}
