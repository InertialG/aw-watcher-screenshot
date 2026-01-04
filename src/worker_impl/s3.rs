use crate::config::S3Config;
use crate::event::ImageEvent;
use crate::worker::TaskProcessor;
use anyhow::{Context, Error, Result};
use s3::creds::Credentials;
use s3::{Bucket, Region};
use tracing::{error, info, warn};

pub struct S3Processor {
    config: S3Config,
    bucket: Option<Box<Bucket>>,
}

impl S3Processor {
    pub fn new(config: S3Config) -> Self {
        Self {
            config,
            bucket: None,
        }
    }
}

impl TaskProcessor<ImageEvent, ImageEvent> for S3Processor {
    fn init(&mut self) -> Result<(), Error> {
        if !self.config.enabled {
            info!("S3 upload is disabled");
            return Ok(());
        }

        let region = Region::Custom {
            region: self.config.region.clone(),
            endpoint: self.config.endpoint.clone(),
        };

        let credentials = Credentials::new(
            Some(&self.config.access_key),
            Some(&self.config.secret_key),
            None,
            None,
            None,
        )
        .context("Failed to create S3 credentials")?;

        let bucket = Bucket::new(&self.config.bucket, region, credentials)
            .context("Failed to create S3 bucket")?
            .with_path_style();

        self.bucket = Some(bucket);
        info!(
            "S3Processor initialized for bucket: {} at {}",
            self.config.bucket, self.config.endpoint
        );
        Ok(())
    }

    fn process(&mut self, event: ImageEvent) -> Result<ImageEvent, Error> {
        let Some(bucket) = &self.bucket else {
            // S3 disabled, pass through
            return Ok(event);
        };

        let prefix = self.config.key_prefix.as_deref().unwrap_or("");

        for (key, data) in event.data_iter() {
            // S3 key = prefix + filename (as confirmed by user)
            let object_key = format!("{}{}--{}.webp", prefix, event.get_id(), key);

            // rust-s3 requires tokio runtime for async operations
            // We're in a blocking context (spawn_blocking), so we need block_on
            let result = tokio::runtime::Handle::current()
                .block_on(async { bucket.put_object(&object_key, &data).await });

            match result {
                Ok(response) => {
                    let status = response.status_code();
                    if status == 200 {
                        info!("Uploaded {} to S3 ({} bytes)", object_key, data.len());
                    } else {
                        warn!("S3 upload {} returned status: {}", object_key, status);
                    }
                }
                Err(e) => {
                    error!("Failed to upload {} to S3: {:?}", object_key, e);
                    // Continue with other files instead of failing completely
                }
            }
        }

        Ok(event)
    }
}
