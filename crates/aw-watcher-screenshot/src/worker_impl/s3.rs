use crate::config::S3Config;
use crate::event::{AwEvent, ImageEvent, UploadS3Info};
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
        let (aw_event, datas) = event.into_parts(UploadS3Info::new(
            self.config.endpoint.clone(),
            self.config.bucket.clone(),
            self.config.key_prefix.clone().unwrap_or_default(),
        ));

        if !self.config.enabled {
            info!("S3 upload is disabled");
            return Ok(aw_event);
        }

        let Some(bucket) = &self.bucket else {
            // S3 disabled, pass through
            return Ok(aw_event);
        };

        let runtime_handle = tokio::runtime::Handle::current();
        runtime_handle.block_on(async {
            let mut upload_futures = Vec::new();

            for (key, data) in datas {
                let Some(upload_info) = aw_event.get_data(key) else {
                    warn!("Failed to get upload info for key {}", key);
                    continue;
                };

                if data.payload.is_none() {
                    warn!("No payload for key {}", key);
                    continue;
                }

                let bucket = bucket.clone();
                let object_path = upload_info.object_key.clone();
                let data_arc = std::sync::Arc::clone(&data);

                let upload_task = async move {
                    let payload = data_arc.payload.as_ref().unwrap();
                    let res = bucket
                        .put_object_with_content_type(&object_path, payload, "image/webp")
                        .await;

                    (object_path, res)
                };

                upload_futures.push(upload_task);
            }

            let results = join_all(upload_futures).await;

            for (object_path, result) in results {
                match result {
                    Ok(response) => {
                        let status = response.status_code();
                        if status == 200 {
                            info!("Successfully uploaded {} to S3", object_path);
                        } else {
                            warn!("S3 upload {} returned status: {}", object_path, status);
                        }
                    }
                    Err(e) => {
                        error!("Failed to upload {} to S3: {:?}", object_path, e);
                    }
                }
            }
        });

        Ok(aw_event)
    }
}
