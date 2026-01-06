use crate::config::S3Config;
use crate::event::{AwEvent, ImageEvent};
use crate::worker::TaskProcessor;
use anyhow::{Context, Error, Result};
use futures::future::join_all;
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

impl TaskProcessor<ImageEvent, AwEvent> for S3Processor {
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

    fn process(&mut self, event: ImageEvent) -> Result<AwEvent, Error> {
        let (aw_event, datas) = event.into_parts();
        if !self.config.enabled {
            info!("S3 upload is disabled");
            return Ok(aw_event);
        }

        let Some(bucket) = &self.bucket else {
            // S3 disabled, pass through
            return Ok(aw_event);
        };

        let _prefix = self.config.key_prefix.as_deref().unwrap_or("");

        let runtime_handle = tokio::runtime::Handle::current();
        runtime_handle.block_on(async {
            let mut upload_futures = Vec::new();

            for (key, data) in datas {
                let Some(object_key) = aw_event.get_data(&key) else {
                    warn!("Failed to get object key {}", key);
                    continue;
                };

                let key_str = object_key.clone();

                let upload_task = async move {
                    let res = bucket
                        .put_object_with_content_type(&key_str, &data, "image/webp")
                        .await;
                    (key_str, res)
                };

                upload_futures.push(upload_task);
            }

            let results = join_all(upload_futures).await;

            for (object_key, result) in results {
                match result {
                    Ok(response) => {
                        let status = response.status_code();
                        if status == 200 {
                            info!("Successfully uploaded {} to S3", object_key);
                        } else {
                            warn!("S3 upload {} returned status: {}", object_key, status);
                        }
                    }
                    Err(e) => {
                        error!("Failed to upload {} to S3: {:?}", object_key, e);
                    }
                }
            }
        });

        Ok(aw_event)
    }
}
