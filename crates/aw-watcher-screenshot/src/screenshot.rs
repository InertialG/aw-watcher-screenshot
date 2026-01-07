//! Reusable screenshot service module.
//!
//! This module provides core screenshot functionality that can be used by
//! different trigger mechanisms (timer, event-driven, manual, etc.).

use crate::config::CaptureConfig;
use crate::event::{CaptureEvent, ImageInfo};
use anyhow::{Context, Error, Result};
use chrono::{DateTime, TimeDelta, Utc};
use image::{DynamicImage, imageops};
use tracing::info;
use xcap::Monitor;

/// State tracking for a single monitor to support skip detection.
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

/// Core screenshot service providing reusable capture functionality.
///
/// This service manages monitor states and provides a `capture()` method
/// that can be called by any trigger mechanism.
pub struct ScreenshotService {
    monitor_states: Vec<ImageInfo<MonitorState>>,
    config: CaptureConfig,
}

impl ScreenshotService {
    /// Create a new ScreenshotService with the given configuration.
    ///
    /// This will enumerate all available monitors and initialize their states.
    pub fn new(config: CaptureConfig) -> Result<Self, Error> {
        let real_monitors = Monitor::all()?;
        info!("ScreenshotService: Found {} monitors", real_monitors.len());

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

    /// Capture screenshots from all monitors.
    ///
    /// Returns a `CaptureEvent` containing images from monitors that have
    /// changed since the last capture (based on dhash comparison).
    ///
    /// This method performs:
    /// 1. Screen capture for each monitor
    /// 2. Duplicate detection using perceptual hashing (dhash)
    /// 3. Rate limiting to avoid excessive captures
    /// 4. Forced capture after configured interval
    pub fn capture(&mut self) -> Result<CaptureEvent, Error> {
        let mut event = CaptureEvent::new();

        for monitor_state in &mut self.monitor_states {
            let capture_res =
                capture_monitor(monitor_state.x, monitor_state.y).with_context(|| {
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

            let mut image_info = ImageInfo::from_base_info(monitor_state);
            image_info.set_payload(capture_res);
            event.add_image(image_info);
        }

        Ok(event)
    }
}

/// Determine if the current capture should be skipped based on:
/// - Rate limiting (< 100ms since last capture)
/// - Perceptual hash similarity (dhash threshold)
/// - Force interval (always capture after configured seconds)
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

        // Rate limit check (100ms debounce)
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

/// Capture a screenshot from the monitor at the given screen coordinates.
fn capture_monitor(x: i32, y: i32) -> Result<DynamicImage, Error> {
    let monitor = Monitor::from_point(x, y)?;
    let image = monitor.capture_image()?;
    let image = DynamicImage::ImageRgba8(image);
    Ok(image)
}

/// Compute perceptual hash (difference hash) for an image.
///
/// The dhash algorithm:
/// 1. Resize image to 9x8
/// 2. Convert to grayscale
/// 3. Compare adjacent pixels horizontally
/// 4. Generate 64-bit hash based on comparisons
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

/// Compute Hamming distance between two hashes.
///
/// Returns the number of bits that differ between the two hashes.
pub fn hamming_distance(hash1: u64, hash2: u64) -> u32 {
    (hash1 ^ hash2).count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dhash_identical() {
        // Two identical images should have hamming distance of 0
        let img = DynamicImage::new_rgba8(100, 100);
        let hash1 = dhash(&img);
        let hash2 = dhash(&img);
        assert_eq!(hamming_distance(hash1, hash2), 0);
    }

    #[test]
    fn test_hamming_distance() {
        assert_eq!(hamming_distance(0b0000, 0b0000), 0);
        assert_eq!(hamming_distance(0b0001, 0b0000), 1);
        assert_eq!(hamming_distance(0b1111, 0b0000), 4);
        assert_eq!(hamming_distance(0xFF, 0x00), 8);
    }
}
