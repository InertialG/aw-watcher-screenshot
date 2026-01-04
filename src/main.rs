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

    // Initialize tracing with EnvFilter
    // Default: show info level, but filter out noisy xcap platform errors
    // Override with RUST_LOG env var, e.g.: RUST_LOG=debug,xcap=off
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,xcap::platform=off"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
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

    // S3 upload processor (optional, based on config)
    let (s3_handle, final_rx) = if let Some(s3_config) = config.s3.clone() {
        if s3_config.enabled {
            let (tx4, rx4) = mpsc::channel::<ImageEvent>(30);
            let s3_processor = worker_impl::s3::S3Processor::new(s3_config);
            let s3_worker = worker::Worker::new("s3".to_string(), s3_processor, rx3, tx4)?;
            let handle = s3_worker.start();
            (Some(handle), rx4)
        } else {
            info!("S3 upload disabled in config");
            (None, rx3)
        }
    } else {
        info!("S3 config not found, skipping S3 upload");
        (None, rx3)
    };

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

    // Drain the final output channel to prevent backpressure
    let drain_handle = tokio::spawn(async move {
        let mut rx = final_rx;
        while let Some(_) = rx.recv().await {
            // drain
        }
        info!("Drain handler stopped.");
    });

    // Wait for all tasks to complete
    let (capture_result, cache_result, sqlite_result, periodic_result, drain_result) = tokio::join!(
        capture_handle,
        cache_handle,
        sqlite_handle,
        periodic_task,
        drain_handle
    );

    // Separately await S3 handle if present
    if let Some(handle) = s3_handle {
        if let Err(e) = handle.await {
            error!("S3 worker joined with error: {}", e);
        }
    }

    let results = (
        capture_result,
        cache_result,
        sqlite_result,
        periodic_result,
        drain_result,
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
