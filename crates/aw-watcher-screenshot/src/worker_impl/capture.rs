//! Timer-based screenshot processor.
//!
//! This module provides a `TaskProcessor` that combines timer triggering
//! with screenshot capture functionality, producing `CaptureEvent`s on a
//! regular interval.

use crate::config::{CaptureConfig, TriggerConfig};
use crate::event::CaptureEvent;
use crate::screenshot::ScreenshotService;
use crate::worker::Producer;
use anyhow::{Error, Result};
use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tokio::time::{self, Instant, Interval, sleep};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

/// Timer-based screenshot processor that produces `CaptureEvent`s on a schedule.
///
/// This processor operates in **Source mode**, meaning it has no input channel
/// and only produces outputs. It combines the functionality of the original
/// `TimerTrigger` and `CaptureProcessor` into a single unified processor.
///
/// # Example
///
/// ```ignore
/// let processor = TimerScreenshotProducer::new(trigger_config, capture_config)?;
/// let worker = Worker::source("timer_screenshot", processor, tx);
/// worker.start();
/// ```
pub struct TimerScreenshotProducer {
    screenshot_service: ScreenshotService,
    interval: Interval,
    timeout: Option<Duration>,
    start_time: Option<Instant>,

    token: CancellationToken,
}

impl TimerScreenshotProducer {
    /// Create a new timer-based screenshot processor.
    ///
    /// # Arguments
    ///
    /// * `trigger_config` - Configuration for timer interval and timeout
    /// * `capture_config` - Configuration for screenshot capture (dhash threshold, etc.)
    pub fn new(
        trigger_config: TriggerConfig,
        capture_config: CaptureConfig,
        token: CancellationToken,
    ) -> Result<Self, Error> {
        let screenshot_service = ScreenshotService::new(capture_config)?;
        let interval_duration = Duration::from_secs(trigger_config.interval_secs);
        let timeout = trigger_config.timeout_secs.map(Duration::from_secs);

        Ok(Self {
            screenshot_service,
            interval: time::interval(interval_duration),
            timeout,
            start_time: None,
            token,
        })
    }

    /// Create a new timer-based screenshot processor with just Duration values.
    ///
    /// This is a convenience constructor for simpler use cases.
    pub fn with_duration(
        interval: Duration,
        timeout: Option<Duration>,
        capture_config: CaptureConfig,
        token: CancellationToken,
    ) -> Result<Self, Error> {
        let screenshot_service = ScreenshotService::new(capture_config)?;

        Ok(Self {
            screenshot_service,
            interval: time::interval(interval),
            timeout,
            start_time: None,
            token,
        })
    }

    fn is_timeout_reached(&self) -> bool {
        if let (Some(timeout), Some(start)) = (self.timeout, self.start_time) {
            start.elapsed() >= timeout
        } else {
            false
        }
    }
}

#[async_trait]
impl Producer<CaptureEvent> for TimerScreenshotProducer {
    async fn produce(mut self, tx: Sender<CaptureEvent>) -> Result<JoinHandle<()>, Error> {
        let handler = tokio::spawn(async move {
            let mut timeout_future: Pin<Box<dyn Future<Output = ()> + Send>> = match self.timeout {
                Some(duration) => Box::pin(sleep(duration)),
                None => Box::pin(std::future::pending()),
            };
            let mut service = Arc::new(self.screenshot_service);

            loop {
                tokio::select! {
                    _ = self.token.cancelled() => {
                        info!("TimerScreenshotProducer cancelled");
                        break;
                    }
                    _ = timeout_future => {
                        info!("TimerScreenshotProducer timed out");
                        break;
                    }
                    _ = self.interval.tick() => {
                        let service_cloned = Arc::clone(&service);
                        match tokio::task::spawn_blocking(move || {
                            let event = service_cloned.capture()?;
                            Ok(event)
                        }).await {
                            Ok(Ok(event)) => {
                                // Only send if we have images (capture might return empty event if no changes)
                                // But CaptureEvent usually contains whatever was captured.
                                // The capture() method returns a Result<CaptureEvent>.
                                if tx.send(event).await.is_err() {
                                    info!("Receiver dropped, stopping TimerScreenshotProducer");
                                    break;
                                }
                            }
                            Ok(Err(e)) => {
                                error!("Failed to capture screenshot: {:?}", e);
                            }
                            Err(e) => {
                                error!("Failed to create capture task: {:?}", e);
                            }
                        }
                    }
                }
            }
        });

        Ok(handler)
    }
}
