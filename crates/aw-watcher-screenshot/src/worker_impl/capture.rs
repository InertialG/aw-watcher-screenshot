//! Timer-based screenshot capture processor.
//!
//! This module provides a `Producer` that captures screenshots from all monitors
//! on a regular interval. The captured images are sent downstream for filtering.

use crate::config::TriggerConfig;
use crate::event::{CaptureEvent, UploadImageInfo};
use crate::worker::Producer;
use anyhow::{Context, Error, Result};
use image::DynamicImage;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tokio::time::{self, Interval, sleep};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use xcap::Monitor;

/// Monitor information for capture.
struct MonitorInfo {
    name: String,
    id: u32,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl MonitorInfo {
    fn new(monitor: Monitor) -> Result<Self, Error> {
        Ok(Self {
            name: monitor.name()?,
            id: monitor.id()?,
            x: monitor.x()?,
            y: monitor.y()?,
            width: monitor.width()?,
            height: monitor.height()?,
        })
    }

    fn get_friendly_name(&self) -> String {
        format!(
            "{}_{}_{}_{}_{}",
            self.name, self.width, self.height, self.x, self.y
        )
    }
}

/// Timer-based screenshot producer that captures from all monitors.
///
/// This producer operates in **Source mode**, meaning it has no input channel
/// and only produces outputs. It captures screenshots at regular intervals
/// without any filtering - filtering is done by a downstream processor.
///
/// # Example
///
/// ```ignore
/// let producer = TimerCaptureProducer::new(trigger_config, token)?;
/// // Wire to filter processor downstream
/// ```
pub struct TimerCaptureProducer {
    monitors: Vec<MonitorInfo>,
    interval: Interval,
    timeout: Option<Duration>,
    token: CancellationToken,
}

impl TimerCaptureProducer {
    /// Create a new timer-based capture producer.
    ///
    /// # Arguments
    ///
    /// * `trigger_config` - Configuration for timer interval and timeout
    /// * `token` - Cancellation token for graceful shutdown
    pub fn new(trigger_config: TriggerConfig, token: CancellationToken) -> Result<Self, Error> {
        let real_monitors = Monitor::all()?;
        info!(
            "TimerCaptureProducer: Found {} monitors",
            real_monitors.len()
        );

        let mut monitors = Vec::new();
        for monitor in real_monitors {
            let monitor_info = MonitorInfo::new(monitor)?;
            monitors.push(monitor_info);
        }

        let interval_duration = Duration::from_secs(trigger_config.interval_secs);
        let timeout = trigger_config.timeout_secs.map(Duration::from_secs);

        Ok(Self {
            monitors,
            interval: time::interval(interval_duration),
            timeout,
            token,
        })
    }

    /// Create a new timer-based capture producer with just Duration values.
    pub fn _with_duration(
        interval: Duration,
        timeout: Option<Duration>,
        token: CancellationToken,
    ) -> Result<Self, Error> {
        let real_monitors = Monitor::all()?;
        info!(
            "TimerCaptureProducer: Found {} monitors",
            real_monitors.len()
        );

        let mut monitors = Vec::new();
        for monitor in real_monitors {
            let monitor_info = MonitorInfo::new(monitor)?;
            monitors.push(monitor_info);
        }

        Ok(Self {
            monitors,
            interval: time::interval(interval),
            timeout,
            token,
        })
    }

    /// Capture screenshots from all monitors.
    fn _capture_all(&self) -> Result<CaptureEvent, Error> {
        let mut event = CaptureEvent::new();

        for monitor_info in &self.monitors {
            let capture_res =
                capture_monitor(monitor_info.x, monitor_info.y).with_context(|| {
                    format!(
                        "Failed to capture image from monitor {}",
                        monitor_info.get_friendly_name()
                    )
                })?;

            let upload_info = UploadImageInfo::new(
                monitor_info.get_friendly_name(),
                monitor_info.id,
                String::new(), // Object key will be set by filter processor
            );

            event.add_image(monitor_info.id, capture_res, upload_info);
        }

        Ok(event)
    }
}

/// Capture a screenshot from the monitor at the given screen coordinates.
fn capture_monitor(x: i32, y: i32) -> Result<DynamicImage, Error> {
    let monitor = Monitor::from_point(x, y)?;
    let image = monitor.capture_image()?;
    let image = DynamicImage::ImageRgba8(image);
    Ok(image)
}

// #[async_trait]
impl Producer<CaptureEvent> for TimerCaptureProducer {
    fn produce(mut self, tx: Sender<CaptureEvent>) -> Result<JoinHandle<()>, Error> {
        let handler = tokio::spawn(async move {
            let timeout_future: Pin<Box<dyn Future<Output = ()> + Send>> = match self.timeout {
                Some(duration) => Box::pin(sleep(duration)),
                None => Box::pin(std::future::pending::<()>()),
            };
            // Pin the future so we can borrow it in the select! loop
            tokio::pin!(timeout_future);

            loop {
                tokio::select! {
                    _ = self.token.cancelled() => {
                        info!("TimerCaptureProducer cancelled");
                        break;
                    }
                    _ = &mut timeout_future => {
                        info!("TimerCaptureProducer timed out");
                        break;
                    }
                    _ = self.interval.tick() => {
                        // Capture in blocking task since xcap operations are blocking
                        let monitors_clone: Vec<(i32, i32, String, u32)> = self.monitors
                            .iter()
                            .map(|m| (m.x, m.y, m.get_friendly_name(), m.id))
                            .collect();

                        match tokio::task::spawn_blocking(move || {
                            let mut event = CaptureEvent::new();
                            for (x, y, name, id) in monitors_clone {
                                match capture_monitor(x, y) {
                                    Ok(image) => {
                                        let upload_info = UploadImageInfo::new(
                                            name,
                                            id,
                                            format!(
                                                "{}/{}.webp",
                                                event.timestamp.format("%Y/%m/%d/%H"),
                                                format!("{}_{}", event.timestamp.format("%Y%m%d_%H%M%S%3f"), id)
                                            ),
                                        );
                                        event.add_image(id, image, upload_info);
                                    }
                                    Err(e) => {
                                        error!("Failed to capture monitor {}: {:?}", id, e);
                                    }
                                }
                            }
                            event
                        }).await {
                            Ok(event) => {
                                info!("TimerCaptureProducer: captured {} images", event.images.len());
                                if tx.send(event).await.is_err() {
                                    info!("Receiver dropped, stopping TimerCaptureProducer");
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Failed to spawn capture task: {:?}", e);
                            }
                        }
                    }
                }
            }
            info!("TimerCaptureProducer finished");
        });

        Ok(handler)
    }
}
