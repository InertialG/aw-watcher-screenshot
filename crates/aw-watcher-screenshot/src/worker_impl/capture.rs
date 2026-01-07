use crate::event::{CaptureCommand, CaptureEvent};
use crate::worker::TaskProcessor;
use anyhow::{Context, Error, Result};
use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use image::{DynamicImage, imageops};
use tracing::info;
use xcap::Monitor;

use crate::event::ImageInfo;

struct MonitorState {
    last_dhash: Option<u64>,
    last_time: Option<DateTime<Utc>>,
}

impl MonitorState {
    fn new() -> Self {
        Self {
            last_dhash: None,
            last_time: None,
        }
    }
}

use crate::config::CaptureConfig;

pub struct CaptureProcessor {
    monitor_states: Vec<ImageInfo<MonitorState>>,
    config: CaptureConfig,
}

#[async_trait]
impl TaskProcessor<CaptureCommand, CaptureEvent> for CaptureProcessor {
    async fn process(&mut self, _event: CaptureCommand) -> Result<CaptureEvent, Error> {
        let mut event = CaptureEvent::new();

        for monitor_state in &mut self.monitor_states {
            let capture_res = capture(monitor_state.x, monitor_state.y).with_context(|| {
                format!(
                    "Failed to capture image from monitor {}",
                    monitor_state.get_friendly_name()
                )
            })?;

            if should_skip(
                &self.config,
                &capture_res,
                monitor_state
                    .payload
                    .as_mut()
                    .context("Monitor state not found")?,
            ) {
                continue;
            }

            let mut image_info = ImageInfo::from_base_info(&monitor_state);
            image_info.set_payload(capture_res);
            event.add_image(image_info);
        }

        Ok(event)
    }
}

impl CaptureProcessor {
    pub fn new(config: CaptureConfig) -> Result<Self, Error> {
        let real_monitors = Monitor::all()?;
        info!("Found {} monitors", real_monitors.len());
        let mut monitor_states = Vec::new();
        for monitor in real_monitors.iter() {
            let mut image_info = ImageInfo::new(monitor)?;
            image_info.set_payload(MonitorState::new());
            monitor_states.push(image_info);
        }

        Ok(Self {
            monitor_states,
            config,
        })
    }
}

fn should_skip(
    config: &CaptureConfig,
    image: &DynamicImage,
    monitor_state: &mut MonitorState,
) -> bool {
    let dhash = dhash(image);
    let now = Utc::now();

    if let Some(last_time) = monitor_state.last_time {
        // Use configured force interval
        if now - last_time > TimeDelta::try_seconds(config.force_interval_secs as i64).unwrap() {
            monitor_state.last_dhash = Some(dhash);
            monitor_state.last_time = Some(now);
            return false;
        }

        // Rate limit check (keeping hardcoded small limit for safety, or could be configurable too, but 100ms is standard debounce)
        if now - last_time < TimeDelta::try_milliseconds(100).unwrap() {
            return true;
        }
    }

    if let Some(last_dhash) = monitor_state.last_dhash {
        // Use configured dhash threshold
        if hamming_distance(dhash, last_dhash) < config.dhash_threshold {
            return true;
        }
    }

    monitor_state.last_dhash = Some(dhash);
    monitor_state.last_time = Some(now);
    false
}

fn capture(x: i32, y: i32) -> Result<DynamicImage, Error> {
    let monitor = Monitor::from_point(x, y)?;
    let image = monitor.capture_image()?;
    let image = DynamicImage::ImageRgba8(image);
    Ok(image)
}

pub fn dhash(image: &DynamicImage) -> u64 {
    let resolution = 8;
    let resized = imageops::resize(
        image,
        resolution + 1,
        resolution,
        imageops::FilterType::Nearest,
    );
    let gray = imageops::grayscale(&resized);

    let mut hash = 0u64;
    for y in 0..resolution {
        for x in 0..resolution {
            let left = gray.get_pixel(x, y)[0];
            let right = gray.get_pixel(x + 1, y)[0];
            if left < right {
                hash |= 1 << (y * resolution + x);
            }
        }
    }
    hash
}

pub fn hamming_distance(hash1: u64, hash2: u64) -> u32 {
    (hash1 ^ hash2).count_ones()
}
