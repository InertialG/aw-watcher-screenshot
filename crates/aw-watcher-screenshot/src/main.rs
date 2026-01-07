mod config;
mod event;
mod screenshot;
mod worker;
mod worker_impl;

use crate::event::{AwEvent, CaptureCommand, CaptureEvent, CompleteCommand, ImageEvent};
use crate::worker::TaskProcessor;
use anyhow::{Error, Result};
use async_trait::async_trait;
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

/// Drain processor - consumes events at the end of the pipeline
struct DrainProcessor;

#[async_trait]
impl TaskProcessor<CompleteCommand, ()> for DrainProcessor {
    async fn consume(&mut self, _event: CompleteCommand) -> Result<(), Error> {
        // Just drain, do nothing
        Ok(())
    }
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

    info!("Config loaded, aw_server: {:?}", config.aw_server);

    // Create processors
    let trigger_interval = Duration::from_secs(config.trigger.interval_secs);
    let trigger_timeout = config.trigger.timeout_secs.map(Duration::from_secs);
    let mut trigger = trigger::TimerTrigger::new(trigger_interval);
    if let Some(t) = trigger_timeout {
        trigger = trigger.with_timeout(t);
    }

    let capture_processor = worker_impl::capture::CaptureProcessor::new(config.capture.clone())?;
    let cache_processor = worker_impl::cache::ToWebpProcessor::new(config.cache.clone());
    let s3_processor = worker_impl::s3::S3Processor::new(config.s3.clone());
    let aw_processor = worker_impl::awserver::AwServerProcessor::new(config.aw_server.clone());
    let drain_processor = DrainProcessor;

    // Unbounded channel between trigger and capture (trigger must never block)
    let (tx0, rx0) = mpsc::unbounded_channel::<CaptureCommand>();
    let (tx1, rx1) = mpsc::channel::<CaptureEvent>(10);
    let (tx2, rx2) = mpsc::channel::<ImageEvent>(30);
    let (tx3, rx3) = mpsc::channel::<AwEvent>(30);
    let (tx4, rx4) = mpsc::channel::<CompleteCommand>(30);

    // Create workers using the unified Worker architecture
    let trigger_worker = worker::Worker::source("trigger", trigger, tx0);
    let capture_worker = worker::Worker::new("capture", capture_processor, rx0, tx1);
    let cache_worker = worker::Worker::new("cache", cache_processor, rx1, tx2);
    let s3_worker = worker::Worker::new("s3", s3_processor, rx2, tx3);
    let aw_worker = worker::Worker::new("awserver", aw_processor, rx3, tx4);
    let drain_worker = worker::Worker::sink("drain", drain_processor, rx4);

    // Start all workers
    let trigger_handle = trigger_worker.start();
    let capture_handle = capture_worker.start();
    let cache_handle = cache_worker.start();
    let s3_handle = s3_worker.start();
    let aw_handle = aw_worker.start();
    let drain_handle = drain_worker.start();

    // Wait for all tasks to complete
    let (trigger_result, capture_result, cache_result, s3_result, aw_result, drain_result) = tokio::join!(
        trigger_handle,
        capture_handle,
        cache_handle,
        s3_handle,
        aw_handle,
        drain_handle
    );

    if let Err(e) = trigger_result {
        error!("Trigger worker joined with error: {}", e);
    }
    if let Err(e) = capture_result {
        error!("Capture worker joined with error: {}", e);
    }
    if let Err(e) = cache_result {
        error!("ToWebp worker joined with error: {}", e);
    }
    if let Err(e) = s3_result {
        error!("S3 worker joined with error: {}", e);
    }
    if let Err(e) = aw_result {
        error!("AWServer worker joined with error: {}", e);
    }
    if let Err(e) = drain_result {
        error!("Drain worker joined with error: {}", e);
    }

    Ok(())
}
