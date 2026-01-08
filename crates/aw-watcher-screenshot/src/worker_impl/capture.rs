//! Timer-based screenshot capture processor.
//!
//! This module provides a `Producer` that captures screenshots from all monitors
//! on a regular interval. The captured images are sent downstream for filtering.

use crate::config::TriggerConfig;
use crate::event::{CaptureEvent, UploadImageInfo};
use crate::worker::Producer;
use anyhow::{Error, Result};
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
            interval: time::interval(interval_duration),
            timeout,
            token,
        })
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
                        // Hot-plug support: refresh monitor list each capture cycle
                        // This handles monitors being connected/disconnected at runtime
                        match tokio::task::spawn_blocking(|| {
                            let monitors = Monitor::all()?;
                            let mut event = CaptureEvent::new();

                            for monitor in monitors {
                                let monitor_info = match MonitorInfo::new(monitor) {
                                    Ok(info) => info,
                                    Err(e) => {
                                        error!(error = %e, "Failed to get monitor info");
                                        continue;
                                    }
                                };

                                match capture_monitor(monitor_info.x, monitor_info.y) {
                                    Ok(image) => {
                                        let upload_info = UploadImageInfo::new(
                                            monitor_info.get_friendly_name(),
                                            monitor_info.id,
                                            format!(
                                                "{}/{}.webp",
                                                event.timestamp.format("%Y/%m/%d/%H"),
                                                format!("{}_{}", event.timestamp.format("%Y%m%d_%H%M%S%3f"), monitor_info.id)
                                            ),
                                        );
                                        event.add_image(monitor_info.id, image, upload_info);
                                    }
                                    Err(e) => {
                                        error!(
                                            monitor_id = monitor_info.id,
                                            monitor_name = %monitor_info.get_friendly_name(),
                                            error = %e,
                                            "Failed to capture monitor"
                                        );
                                    }
                                }
                            }
                            Ok::<_, Error>(event)
                        }).await {
                            Ok(Ok(event)) => {
                                info!(
                                    captured = event.images.len(),
                                    "Captured screenshots from monitors"
                                );
                                if tx.send(event).await.is_err() {
                                    info!("Receiver dropped, stopping TimerCaptureProducer");
                                    break;
                                }
                            }
                            Ok(Err(e)) => {
                                error!(error = %e, "Failed to enumerate monitors");
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to spawn capture task");
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
