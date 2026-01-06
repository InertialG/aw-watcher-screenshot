use crate::event::{CaptureCommand, CaptureEvent};
use crate::worker::TaskProcessor;
use anyhow::{Context, Error, Result};
use chrono::{DateTime, TimeDelta, Utc};
use crc32fast::Hasher;
use image::{DynamicImage, imageops};
use regex::Regex;
use xcap::Monitor;

struct MonitorInfo {
    name: String,
    hash: u32,
    x: i32,
    y: i32,
    last_dhash: Option<u64>,
    last_time: Option<DateTime<Utc>>,
}

use crate::config::CaptureConfig;

pub struct CaptureProcessor {
    monitor_infos: Vec<MonitorInfo>,
    config: CaptureConfig,
}

impl TaskProcessor<CaptureCommand, CaptureEvent> for CaptureProcessor {
    fn process(&mut self, _event: CaptureCommand) -> Result<CaptureEvent, Error> {
        let mut event = CaptureEvent::new();

        for monitor_info in &mut self.monitor_infos {
            let capture_res = capture(monitor_info.x, monitor_info.y).with_context(|| {
                format!("Failed to capture image from monitor {}", monitor_info.hash)
            })?;

            if should_skip(&self.config, &capture_res, monitor_info) {
                continue;
            }

            event.add_image(monitor_info.hash.to_string(), capture_res);
        }

        Ok(event)
    }
}

impl CaptureProcessor {
    pub fn new(config: CaptureConfig) -> Result<Self, Error> {
        let real_monitors = Monitor::all()?;
        let mut monitor_infos = Vec::new();
        for monitor in real_monitors {
            let monitor_info = hash_position(&monitor)?;
            monitor_infos.push(monitor_info);
        }

        Ok(Self {
            monitor_infos,
            config,
        })
    }
}

fn should_skip(
    config: &CaptureConfig,
    image: &DynamicImage,
    monitor_info: &mut MonitorInfo,
) -> bool {
    let dhash = dhash(image);
    let now = Utc::now();

    if let Some(last_time) = monitor_info.last_time {
        // Use configured force interval
        if now - last_time > TimeDelta::try_seconds(config.force_interval_secs as i64).unwrap() {
            monitor_info.last_dhash = Some(dhash);
            monitor_info.last_time = Some(now);
            return false;
        }

        // Rate limit check (keeping hardcoded small limit for safety, or could be configurable too, but 100ms is standard debounce)
        if now - last_time < TimeDelta::try_milliseconds(100).unwrap() {
            return true;
        }
    }

    if let Some(last_dhash) = monitor_info.last_dhash {
        // Use configured dhash threshold
        if hamming_distance(dhash, last_dhash) < config.dhash_threshold {
            return true;
        }
    }

    monitor_info.last_dhash = Some(dhash);
    monitor_info.last_time = Some(now);
    false
}

fn capture(x: i32, y: i32) -> Result<DynamicImage, Error> {
    let monitor = Monitor::from_point(x, y)?;
    let image = monitor.capture_image()?;
    let image = DynamicImage::ImageRgba8(image);
    Ok(image)
}

fn hash_position(monitor: &Monitor) -> Result<MonitorInfo, Error> {
    let name = monitor.name()?;
    let x = monitor.x()?;
    let y = monitor.y()?;
    let width = monitor.width()?;
    let height = monitor.height()?;

    let re = Regex::new(r"[^a-zA-Z0-9]")?;
    let safe_name = re.replace_all(&name, "").to_string();
    let prefix = if safe_name.is_empty() {
        "Monitor".to_string()
    } else {
        safe_name
    };
    let geometry_fingerprint = format!("{}_{}_{}_{}_{}", name, width, height, x, y);
    let hash = calculate_crc32(&geometry_fingerprint);
    Ok(MonitorInfo {
        name: format!("{}_{}", prefix, hash),
        hash,
        x,
        y,
        last_dhash: None,
        last_time: None,
    })
}

fn calculate_crc32(data: &str) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data.as_bytes());
    hasher.finalize()
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
