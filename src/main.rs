mod config;
mod event;
mod trigger;
mod worker;
mod worker_impl;

use crate::event::{CaptureCommand, ImageEvent};
use anyhow::{Error, Result};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info};
use tracing_subscriber;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Starting capture service...");

    let config = match crate::config::Config::load_from_file(&args.config) {
        Ok(c) => c,
        Err(e) => {
            info!(
                "Failed to load config from {:?}: {}. Using defaults.",
                args.config, e
            );
            crate::config::Config::default_config()
        }
    };

    let capture_processor = worker_impl::capture::CaptureProcessor::new(config.capture.clone())?;
    let cache_processor = worker_impl::cache::ToWebpProcessor::new(config.cache.clone());
    let sqlite_processor = worker_impl::sqlite::SqliteProcessor::new(config.sqlite.clone());

    let (tx0, rx0) = mpsc::channel::<CaptureCommand>(5);
    let (tx1, rx1) = mpsc::channel::<ImageEvent>(10);
    let (tx2, rx2) = mpsc::channel::<ImageEvent>(30);
    let (tx3, rx3) = mpsc::channel::<ImageEvent>(30);

    let capture_worker = worker::Worker::new("capture".to_string(), capture_processor, rx0, tx1)?;
    let cache_worker = worker::Worker::new("cache".to_string(), cache_processor, rx1, tx2)?;
    let sqlite_worker = worker::Worker::new("sqlite".to_string(), sqlite_processor, rx2, tx3)?;

    let capture_handle = capture_worker.start();
    let cache_handle = cache_worker.start();
    let sqlite_handle = sqlite_worker.start();

    let trigger_interval = Duration::from_secs(config.trigger.interval_secs);
    let trigger_timeout = config.trigger.timeout_secs.map(Duration::from_secs);

    let mut trigger = trigger::TimerTrigger::new(trigger_interval, tx0);
    if let Some(t) = trigger_timeout {
        trigger = trigger.with_timeout(t);
    }
    let periodic_task = tokio::spawn(async move {
        if let Err(e) = trigger.run().await {
            error!("Trigger failed: {}", e);
        }
    });

    // We need to keep rx3 open so the pipeline doesn't clog, but we don't need to do anything with the result.
    // Ideally, we could have a "sink" worker, or just drop the receiver if we don't care about backpressure propagating technically.
    // But if we drop rx3, the sender tx3 will error, causing sqlite worker to stop.
    // So we spawn a drain task.
    let drain_handle = tokio::spawn(async move {
        let mut rx3 = rx3;
        while let Some(_) = rx3.recv().await {
            // drain
        }
        info!("Drain handler stopped.");
    });

    let results = tokio::join!(
        capture_handle,
        cache_handle,
        sqlite_handle,
        periodic_task,
        drain_handle
    );
    if let Err(e) = results.0 {
        error!("Capture worker joined with error: {}", e);
    }
    if let Err(e) = results.1 {
        error!("ToWebp worker joined with error: {}", e);
    }
    if let Err(e) = results.2 {
        error!("Sqlite worker joined with error: {}", e);
    }
    if let Err(e) = results.3 {
        error!("Periodic task joined with error: {}", e);
    }
    if let Err(e) = results.4 {
        error!("Drain handler joined with error: {}", e);
    }
    Ok(())
}
