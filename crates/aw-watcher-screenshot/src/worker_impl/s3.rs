use std::sync::Arc;

use crate::config::S3Config;
use crate::event::{AwEvent, ImageEvent, UploadS3Info};
use crate::worker::Processor;
use anyhow::{Context, Error, Result};
use futures::future::join_all;
use s3::creds::Credentials;
use s3::{Bucket, Region};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

pub struct S3Processor {
    upload_config: UploadS3Info,
    bucket: Arc<Box<Bucket>>,
}

impl S3Processor {
    pub fn new(config: S3Config) -> Result<Self, Error> {
        let region = Region::Custom {
            region: config.region,
            endpoint: config.endpoint.clone(),
        };

        let credentials = Credentials::new(
            Some(&config.access_key),
            Some(&config.secret_key),
            None,
            None,
            None,
        )
        .context("Failed to create S3 credentials")?;

        let bucket = Bucket::new(&config.bucket, region, credentials)
            .context("Failed to create S3 bucket")?
            .with_path_style();

        Ok(Self {
            upload_config: UploadS3Info::new(config.endpoint, config.bucket, config.key_prefix),
            bucket: Arc::new(bucket),
        })
    }
}

impl Processor<ImageEvent, AwEvent> for S3Processor {
    fn process(
        self,
        mut rx: Receiver<ImageEvent>,
        tx: Sender<AwEvent>,
    ) -> Result<JoinHandle<()>, Error> {
        Ok(tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                info!("S3Processor: uploading {} images", event.datas.len());

                let mut upload_futures = Vec::new();
                for (key, data) in event.datas {
                    let bucket = self.bucket.clone();
                    let Some(image_info) = event.monitors.get(&key) else {
                        warn!("Failed to get upload info for key {}", key);
                        continue;
                    };

                    let object_key = image_info.object_key.clone();
                    let upload_task = async move {
                        match bucket
                            .put_object_with_content_type(&object_key, &data, "image/webp")
                            .await
                        {
                            Ok(_) => {
                                info!("S3Processor: uploaded {}", object_key);
                                (true, key)
                            }
                            Err(e) => {
                                error!("Failed to upload {} to S3: {:?}", object_key, e);
                                (false, key)
                            }
                        }
                    };

                    upload_futures.push(upload_task);
                }

                // Create AwEvent with all monitor info
                let mut aw_event = AwEvent::new(
                    event.timestamp,
                    event.local_dir,
                    Some(self.upload_config.clone()),
                );

                // Add all monitor info to the event
                for (key, monitor_info) in event.monitors {
                    aw_event.add_data(key, monitor_info);
                }

                // Run uploads and update status
                let results = join_all(upload_futures).await;

                for (success, key) in results {
                    if success {
                        aw_event.set_uploaded(key);
                    }
                }

                info!("S3Processor: event has {} images", aw_event.datas.len());
                if let Err(e) = tx.send(aw_event).await {
                    error!("Failed to send event to channel: {}", e);
                    break;
                }
            }
            info!("S3Processor finished");
        }))
    }
}
