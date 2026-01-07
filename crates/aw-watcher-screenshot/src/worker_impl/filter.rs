//! Screenshot filtering processor using perceptual hashing.
//!
//! This module provides a `Processor` that filters captured screenshots
//! based on perceptual hash (dhash) comparison to skip unchanged screens.

use crate::config::CaptureConfig;
use crate::event::CaptureEvent;
use crate::worker::Processor;
use anyhow::{Error, Result};
use chrono::{DateTime, TimeDelta, Utc};
use image::{DynamicImage, imageops};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::info;

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

/// Screenshot filter processor that removes unchanged screens.
///
/// This processor receives `CaptureEvent`s and produces `FilteredCaptureEvent`s
/// containing only the monitors that have changed since the last capture.
pub struct FilterProcessor {
    config: CaptureConfig,
    monitor_states: HashMap<u32, MonitorState>,
}

impl FilterProcessor {
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            config,
            monitor_states: HashMap::new(),
        }
    }

    /// Determine if the current capture should be skipped based on:
    /// - Rate limiting (< 100ms since last capture)
    /// - Perceptual hash similarity (dhash threshold)
    /// - Force interval (always capture after configured seconds)
    fn should_skip(&mut self, monitor_id: u32, image: &DynamicImage) -> bool {
        let dhash = dhash(image);
        let now = Utc::now();

        let state = self
            .monitor_states
            .entry(monitor_id)
            .or_insert_with(MonitorState::new);

        if let Some(last_time) = state.last_time {
            // Use configured force interval
            if now - last_time
                > TimeDelta::try_seconds(self.config.force_interval_secs as i64).unwrap()
            {
                state.last_dhash = Some(dhash);
                state.last_time = Some(now);
                return false;
            }

            // Rate limit check (100ms debounce)
            if now - last_time < TimeDelta::try_milliseconds(100).unwrap() {
                return true;
            }
        }

        if let Some(last_dhash) = state.last_dhash {
            // Use configured dhash threshold
            if hamming_distance(dhash, last_dhash) < self.config.dhash_threshold {
                return true;
            }
        }

        state.last_dhash = Some(dhash);
        state.last_time = Some(now);
        false
    }

    /// Generate short hash for monitor identification.
    fn _get_short_hash(name: &str, width: u32, height: u32, x: i32, y: i32) -> String {
        let mut hasher = DefaultHasher::new();
        (name, width, height, x, y).hash(&mut hasher);
        format!("{:08x}", hasher.finish())
    }
}

impl Processor<CaptureEvent, CaptureEvent> for FilterProcessor {
    fn process(
        mut self,
        mut rx: Receiver<CaptureEvent>,
        tx: Sender<CaptureEvent>,
    ) -> Result<JoinHandle<()>, Error> {
        let handler = tokio::spawn(async move {
            while let Some(mut event) = rx.recv().await {
                let original_count = event.images.len();
                event
                    .images
                    .retain(|id, image| !self.should_skip(*id, image));
                // Sync monitors with images - remove monitors that were filtered out
                event.monitors.retain(|id, _| event.images.contains_key(id));
                let filtered_count = event.images.len();
                info!(
                    "FilterProcessor: received {} images, {} passed filter",
                    original_count, filtered_count
                );

                if let Err(e) = tx.send(event).await {
                    info!("FilterProcessor: receiver dropped, stopping: {}", e);
                    break;
                }
            }
            info!("FilterProcessor finished");
        });

        Ok(handler)
    }
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
