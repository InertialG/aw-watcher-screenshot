mod event;
mod worker;
mod worker_impl;

use crate::event::{CaptureCommand, ImageEvent};
use anyhow::{Error, Result};
use std::time::Duration;
use tokio::signal;
use tokio::sync::mpsc;
use tokio::time;
use tracing::{error, info};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Starting capture service...");

    let capture_processor = worker_impl::capture::CaptureProcessor::new()?;
    let cache_processor = worker_impl::cache::ToWebpProcessor::new();

    let (tx0, rx0) = mpsc::channel::<CaptureCommand>(5);
    let (tx1, rx1) = mpsc::channel::<ImageEvent>(10);
    let (tx2, rx2) = mpsc::channel::<ImageEvent>(30);

    let capture_worker = worker::Worker::new("capture".to_string(), capture_processor, rx0, tx1)?;
    let cache_worker = worker::Worker::new("cache".to_string(), cache_processor, rx1, tx2)?;

    let capture_handle = capture_worker.start();
    let cache_handle = cache_worker.start();

    // 启动一个异步任务，每隔两秒发送一个 ImageEvent 到 tx0
    // 20秒后或收到 Ctrl+C 后终止
    let periodic_task = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(2));
        let timeout = time::sleep(Duration::from_secs(20));
        tokio::pin!(timeout);

        info!("Periodic sender started.");

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    info!("Periodic tick: sending CaptureCommand");
                    let reason = "periodic tick";
                    if let Err(e) = tx0.send(CaptureCommand::new(reason.to_string())).await {
                        error!("Failed to send CaptureCommand: {}", e);
                        break;
                    }
                }
                _ = &mut timeout => {
                    info!("20 seconds reached, stopping periodic sender.");
                    break;
                }
                res = signal::ctrl_c() => {
                    if let Err(e) = res {
                        error!("Failed to listen for ctrl_c: {}", e);
                    } else {
                        info!("Ctrl+C received, stopping periodic sender.");
                    }
                    break;
                }
            }
        }
        info!("Periodic sender stopped and tx0 dropped.");
    });

    let finish_handler = tokio::spawn(async move {
        let mut rx2 = rx2;
        let mut count = 0;
        while let Some(_) = rx2.recv().await {
            count += 1;
            info!("Received ImageEvent, continue...");
        }
        info!("Received ImageEvent, finish. {}", count);
    });

    let results = tokio::join!(capture_handle, cache_handle, periodic_task, finish_handler);
    if let Err(e) = results.0 {
        error!("Capture worker joined with error: {}", e);
    }
    if let Err(e) = results.1 {
        error!("ToWebp worker joined with error: {}", e);
    }
    if let Err(e) = results.2 {
        error!("Periodic task joined with error: {}", e);
    }
    if let Err(e) = results.3 {
        error!("Finish handler joined with error: {}", e);
    }
    Ok(())
}
