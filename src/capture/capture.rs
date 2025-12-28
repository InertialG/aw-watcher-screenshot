use anyhow::Result;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};
use tracing::{error, info};
use xcap::Monitor;

use super::event::MonitorImageEvent;
use super::monitor::SafeMonitor;

pub struct Capture {
    consumer: mpsc::Sender<MonitorImageEvent>,
    stopper: broadcast::Sender<bool>,
    handlers: Vec<JoinHandle<()>>,
}

impl Capture {
    pub fn new(
        consumer: mpsc::Sender<MonitorImageEvent>,
        stopper: broadcast::Sender<bool>,
    ) -> Self {
        let handlers = Vec::new();
        Self {
            consumer,
            stopper,
            handlers,
        }
    }
}

impl Capture {
    pub fn run(&mut self) -> Result<()> {
        let monitors = Monitor::all().map_err(|e| {
            error!("Get monitors error: {}", e);
            e
        })?;

        for monitor in monitors {
            let mut safe_monitor = SafeMonitor::new(&monitor)?;

            let consumer = self.consumer.clone();
            let mut stopper = self.stopper.subscribe();

            self.handlers.push(tokio::task::spawn(async move {
                let monitor_id = safe_monitor.id();
                loop {
                    tokio::select! {
                        _ = stopper.recv() => {
                            info!("Capture task for {} stopping...", monitor_id);
                            break;
                        }

                        event_res = safe_monitor.get_next_event() => {
                            match event_res {
                                // 情况 A: 成功获取到需要发送的图片
                                Ok(Some(event)) => {
                                    if let Err(e) = consumer.send(event).await {
                                        error!("Consumer dropped: {}", e);
                                        break;
                                    }
                                }
                                // 情况 B: 图片没有变化，跳过
                                Ok(None) => {
                                    info!("Monitor {} skip current capture.", monitor_id);
                                }
                                // 情况 C: 截图过程报错 (如驱动问题、显示器断开)
                                Err(e) => {
                                    error!("Capture logic error for {}: {}", safe_monitor.id(), e);
                                }
                            }

                            // 统一的间隔控制
                            sleep(Duration::from_secs(2)).await;
                        }
                    }
                }
            }))
        }

        Ok(())
    }

    pub async fn wait(&mut self) {
        // 这里的 handlers 就是你之前 push 进去的 Vec<JoinHandle<()>>
        // 使用 drain 确保可以多次调用或清空
        let handles: Vec<_> = self.handlers.drain(..).collect();
        futures::future::join_all(handles).await;
    }
}
