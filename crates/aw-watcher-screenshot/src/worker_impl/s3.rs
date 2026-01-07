use crate::config::S3Config;
use crate::event::{AwEvent, ImageEvent, UploadS3Info};
use crate::worker::Processor;
use anyhow::{Context, Error, Result};
use async_trait::async_trait;
use futures::future::join_all;
use s3::creds::Credentials;
use s3::{Bucket, Region};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

pub struct S3Processor {
    config: S3Config,
    bucket: Box<Bucket>,
}

impl S3Processor {
    pub fn new(config: S3Config) -> Result<Self, Error> {
        let region = Region::Custom {
            region: config.region,
            endpoint: config.endpoint,
        };

        let credentials = Credentials::new(
            Some(&config.access_key),
            Some(&config.secret_key),
            None,
            None,
            None,
        )
        .context("Failed to create S3 credentials")?;

        let bucket = Bucket::new(&self.config.bucket, region, credentials)
            .context("Failed to create S3 bucket")?
            .with_path_style();

        Ok(Self {
            config,
            bucket: bucket,
        })
    }
}

#[async_trait]
impl Processor<ImageEvent, AwEvent> for S3Processor {
    async fn process(
        mut self,
        rx: Receiver<ImageEvent>,
        tx: Sender<AwEvent>,
    ) -> Result<JoinHandle<()>, Error> {
        Ok(tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let (aw_event, datas) = event.into_parts(UploadS3Info::new(
                    self.config.endpoint,
                    self.config.bucket,
                    self.config.key_prefix?,
                ));

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

                    let bucket = self.bucket.clone();
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
            }
        }))
    }

    // if !self.config.enabled {
    //     info!("S3 upload is disabled");
    //     return Ok(aw_event);
    // }

    // let Some(bucket) = &self.bucket else {
    //     // S3 disabled, pass through
    //     return Ok(aw_event);
    // };

    // {
    //     let mut upload_futures = Vec::new();

    //     for (key, data) in datas {
    //         let Some(upload_info) = aw_event.get_data(key) else {
    //             warn!("Failed to get upload info for key {}", key);
    //             continue;
    //         };

    //         if data.payload.is_none() {
    //             warn!("No payload for key {}", key);
    //             continue;
    //         }

    //         let bucket = bucket.clone();
    //         let object_path = upload_info.object_key.clone();
    //         let data_arc = std::sync::Arc::clone(&data);

    //         let upload_task = async move {
    //             let payload = data_arc.payload.as_ref().unwrap();
    //             let res = bucket
    //                 .put_object_with_content_type(&object_path, payload, "image/webp")
    //                 .await;

    //             (object_path, res)
    //         };

    //         upload_futures.push(upload_task);
    //     }

    //     let results = join_all(upload_futures).await;

    //     for (object_path, result) in results {
    //         match result {
    //             Ok(response) => {
    //                 let status = response.status_code();
    //                 if status == 200 {
    //                     info!("Successfully uploaded {} to S3", object_path);
    //                 } else {
    //                     warn!("S3 upload {} returned status: {}", object_path, status);
    //                 }
    //             }
    //             Err(e) => {
    //                 error!("Failed to upload {} to S3: {:?}", object_path, e);
    //             }
    //         }
    //     }
    // }

    // Ok(aw_event)
}
