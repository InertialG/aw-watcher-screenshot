use std::collections::HashMap;

use crate::event::{CompleteCommand, UploadImageInfo, UploadS3Info};
use crate::worker::TaskProcessor;
use crate::{config::AwServerConfig, event::AwEvent};
use anyhow::{Context, Error, Result};
use async_trait::async_trait;
use aw_client_lite::AwClient;
use aw_models::Event;
use chrono::{DateTime, Duration, Utc};
use serde_json::{Map, Value};
use tracing::info;

pub struct AwServerProcessor {
    config: AwServerConfig,
    client: Option<AwClient>,
    bucket_id: Option<String>,

    timeout: Duration,
    last_datas: Option<AwEvent>,
    last_timestamp: Option<HashMap<u32, DateTime<Utc>>>,
}

impl AwServerProcessor {
    pub fn new(config: AwServerConfig) -> Self {
        let timeout = config.timeout_secs.unwrap_or(60);
        Self {
            config,
            client: None,
            bucket_id: None,
            timeout: Duration::seconds(timeout as i64),
            last_datas: None,
            last_timestamp: None,
        }
    }

    async fn heartbeat_data(&self, upload: &Event, pulse_time: f64) -> Result<(), Error> {
        let Some(bucket_id) = &self.bucket_id else {
            return Err(anyhow::anyhow!("Bucket ID not initialized"));
        };
        let Some(client) = &self.client else {
            return Err(anyhow::anyhow!("Client not initialized"));
        };
        client
            .heartbeat(bucket_id, upload, pulse_time)
            .await
            .context("Failed to send heartbeat")?;
        Ok(())
    }
}

#[async_trait]
impl TaskProcessor<AwEvent, CompleteCommand> for AwServerProcessor {
    async fn init(&mut self) -> Result<(), Error> {
        let client = AwClient::new(&self.config.host, self.config.port);

        let bucket_id = format!("{}_{}", self.config.bucket_id, self.config.hostname);

        let bucket = serde_json::json!({
            "id": bucket_id,
            "client": self.config.bucket_id,
            "hostname": self.config.hostname,
            "type": "uno.guan810.screenshot"
        });

        client
            .create_bucket(&bucket)
            .await
            .context("Failed to create bucket")?;

        self.client = Some(client);
        self.bucket_id = Some(bucket_id);

        info!("AwServer initialized successfully.");

        Ok(())
    }

    async fn process(&mut self, mut event: AwEvent) -> Result<CompleteCommand, Error> {
        let timestamp = event.timestamp;
        let Some(pulse_time) = self.config.pulse_time else {
            return Err(anyhow::anyhow!("Pulse time not initialized"));
        };

        let last_timestamp = self.last_timestamp.get_or_insert_with(HashMap::new);

        if event.datas.is_empty() {
            let Some(last_heartbeat) = &self.last_datas else {
                return Err(anyhow::anyhow!("Empty heartbeat at first."));
            };

            info!("Some heartbeat data with last one.");
            let heart_beat = Event {
                id: None,
                timestamp,
                duration: Duration::zero(),
                data: create_heartbeat_data(&last_heartbeat),
            };
            self.heartbeat_data(&heart_beat, pulse_time).await?;
            return Ok(true);
        }

        // Update last_timestamp for current images
        for key in event.datas.keys() {
            last_timestamp.insert(*key, timestamp);
        }

        // Check last_datas for missing images and retention
        if let Some(last_datas) = &self.last_datas {
            for (key, value) in last_datas.datas.iter() {
                if !event.datas.contains_key(key) {
                    if let Some(last_ts) = last_timestamp.get(key) {
                        if timestamp - *last_ts <= self.timeout {
                            // Keep image if within timeout
                            event.add_data(*key, value.clone());
                        }
                    }
                }
            }
        }

        // Clean up last_timestamp for images no longer being tracked
        last_timestamp.retain(|key, _| event.datas.contains_key(key));

        if let Some(last_datas) = &self.last_datas {
            let finish = Event {
                id: None,
                timestamp: timestamp - Duration::milliseconds(1),
                duration: Duration::zero(),
                data: create_heartbeat_data(&last_datas),
            };
            self.heartbeat_data(&finish, pulse_time).await?;
        }

        let heartbeat = Event {
            id: None,
            timestamp,
            duration: Duration::zero(),
            data: create_heartbeat_data(&event),
        };

        self.heartbeat_data(&heartbeat, pulse_time).await?;

        self.last_datas = Some(event);
        Ok(true)
    }
}

fn create_heartbeat_data(event: &AwEvent) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert(
        "local_dir".to_string(),
        Value::String(event.local_dir.display().to_string()),
    );
    map.insert(
        "s3_info".to_string(),
        serde_json::to_value(event.s3_info.clone()).unwrap_or(Value::Null),
    );

    let mut images = Vec::new();
    for value in event.datas.values() {
        images.push(serde_json::to_value(value.clone()).unwrap_or(Value::Null));
    }
    map.insert("images".to_string(), Value::Array(images));

    map
}
