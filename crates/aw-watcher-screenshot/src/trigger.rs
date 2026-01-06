use crate::event::CaptureCommand;
use anyhow::Result;
use std::time::Duration;
use tokio::signal;
use tokio::sync::mpsc;
use tokio::time;
use tracing::{error, info};

pub struct TimerTrigger {
    interval: Duration,
    tx: mpsc::Sender<CaptureCommand>,
    timeout: Option<Duration>,
}

impl TimerTrigger {
    pub fn new(interval: Duration, tx: mpsc::Sender<CaptureCommand>) -> Self {
        Self {
            interval,
            tx,
            timeout: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub async fn run(&self) -> Result<()> {
        let mut interval = time::interval(self.interval);

        // Handle optional timeout logic
        let timeout_future = async {
            if let Some(t) = self.timeout {
                time::sleep(t).await;
                info!("TimerTrigger timeout reached.");
            } else {
                // If no timeout, sleep forever (effectively pending)
                std::future::pending::<()>().await;
            }
        };
        tokio::pin!(timeout_future);

        info!("TimerTrigger started.");

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    info!("TimerTrigger tick: sending CaptureCommand");
                    if let Err(e) = self.tx.send(true).await {
                        error!("Failed to send CaptureCommand: {}", e);
                        break;
                    }
                }
                _ = &mut timeout_future => {
                    break;
                }
                res = signal::ctrl_c() => {
                     if let Err(e) = res {
                        error!("Failed to listen for ctrl_c: {}", e);
                    } else {
                        info!("Ctrl+C received, stopping TimerTrigger.");
                    }
                    break;
                }
            }
        }
        info!("TimerTrigger stopped.");
        Ok(())
    }
}
