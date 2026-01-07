mod config;
mod event;
mod worker;
mod worker_impl;

use crate::event::{AwEvent, CaptureEvent, ImageEvent};
use crate::worker::{Consumer, Processor, Producer};
use anyhow::{Error, Result};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

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

    info!("Config loaded, aw_server: {:?}", config.aw_server);

    // Create channels for the worker pipeline
    // Flow: Capture -> Filter -> Cache (ToWebp) -> S3 -> AwServer
    let cancel_token = CancellationToken::new();

    // Setup Ctrl-C handler to trigger graceful shutdown
    let ctrl_c_token = cancel_token.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl-C");
        info!("Ctrl-C received, initiating graceful shutdown...");
        ctrl_c_token.cancel();
    });

    let (tx_capture, rx_capture) = mpsc::channel::<CaptureEvent>(10);
    let (tx_filter, rx_filter) = mpsc::channel::<CaptureEvent>(10);
    let (tx_cache, rx_cache) = mpsc::channel::<ImageEvent>(10);
    let (tx_s3, rx_s3) = mpsc::channel::<AwEvent>(10);

    // Create processors
    let capture_producer =
        worker_impl::capture::TimerCaptureProducer::new(config.trigger, cancel_token.clone())?;
    let filter_processor = worker_impl::filter::FilterProcessor::new(config.capture.clone());
    let cache_processor = worker_impl::cache::ToWebpProcessor::new(config.cache.clone())?;
    let s3_processor = worker_impl::s3::S3Processor::new(config.s3.clone())?;
    let aw_processor =
        worker_impl::awserver::AwServerProcessor::new(config.aw_server.clone()).await?;

    // Start all workers with proper channel wiring
    // Producer: TimerCaptureProducer -> tx_capture
    let capture_handle = capture_producer.produce(tx_capture)?;
    // Processor: rx_capture -> FilterProcessor -> tx_filter
    let filter_handle = filter_processor.process(rx_capture, tx_filter)?;
    // Processor: rx_filter -> ToWebpProcessor -> tx_cache
    let cache_handle = cache_processor.process(rx_filter, tx_cache)?;
    // Processor: rx_cache -> S3Processor -> tx_s3
    let s3_handle = s3_processor.process(rx_cache, tx_s3)?;
    // Consumer: rx_s3 -> AwServerProcessor
    let aw_handle = aw_processor.consume(rx_s3)?;

    // Wait for all tasks to complete, with graceful shutdown timeout
    let all_workers = async {
        let (capture_result, filter_result, cache_result, s3_result, aw_result) = tokio::join!(
            capture_handle,
            filter_handle,
            cache_handle,
            s3_handle,
            aw_handle
        );

        if let Err(e) = capture_result {
            error!("Capture worker joined with error: {}", e);
        }
        if let Err(e) = filter_result {
            error!("Filter worker joined with error: {}", e);
        }
        if let Err(e) = cache_result {
            error!("Cache worker joined with error: {}", e);
        }
        if let Err(e) = s3_result {
            error!("S3 worker joined with error: {}", e);
        }
        if let Err(e) = aw_result {
            error!("AwServer worker joined with error: {}", e);
        }
    };

    // Wait for cancellation, then give workers time to finish gracefully
    tokio::select! {
        _ = all_workers => {
            info!("All workers finished normally.");
        }
        _ = cancel_token.cancelled() => {
            info!("Shutdown initiated, waiting up to 5 seconds for workers to finish...");
            let shutdown_timeout = tokio::time::sleep(std::time::Duration::from_secs(5));
            tokio::select! {
                _ = shutdown_timeout => {
                    info!("Shutdown timeout reached, forcing exit.");
                }
            }
        }
    }

    info!("Shutdown complete.");
    Ok(())
}
