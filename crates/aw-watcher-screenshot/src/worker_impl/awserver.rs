use std::collections::HashMap;

use crate::worker::Consumer;
use crate::{config::AwServerConfig, event::AwEvent};
use anyhow::Error;
use aw_client_lite::AwClient;
use aw_models::Event;
use chrono::{DateTime, Duration, Utc};
use serde_json::{Map, Value};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tracing::{error, info};

pub struct AwServerProcessor {
    config: AwServerConfig,
    client: AwClient,
    bucket_id: String,

    timeout: Duration,
    last_datas: Option<AwEvent>,
    last_timestamp: HashMap<u32, DateTime<Utc>>,
}

impl AwServerProcessor {
    pub async fn new(config: AwServerConfig) -> Result<Self, Error> {
        let timeout = config.timeout_secs.unwrap_or(60);
        let client = AwClient::new(&config.host, config.port);

        let bucket_id = format!("{}_{}", config.bucket_id, config.hostname);

        let bucket = serde_json::json!({
            "id": bucket_id,
            "client": config.bucket_id,
            "hostname": config.hostname,
            "type": "uno.guan810.screenshot"
        });

        client.create_bucket(&bucket).await?;
        info!("AwServer initialized successfully.");

        Ok(Self {
            config,
            client,
            bucket_id,
            timeout: Duration::seconds(timeout as i64),
            last_datas: None,
            last_timestamp: HashMap::new(),
        })
    }

    pub async fn heartbeat(&self, event: &Event, pulse_time: f64) {
        if let Err(e) = self
            .client
            .heartbeat(&self.bucket_id, event, pulse_time)
            .await
        {
            error!("Failed to heartbeat: {}", e);
        }
    }
}

impl Consumer<AwEvent> for AwServerProcessor {
    fn consume(mut self, mut rx: Receiver<AwEvent>) -> Result<JoinHandle<()>, Error> {
        let Some(pulse_time) = self.config.pulse_time else {
            return Err(anyhow::anyhow!("Pulse time not initialized"));
        };

        Ok(tokio::spawn(async move {
            while let Some(mut event) = rx.recv().await {
                let timestamp = event.timestamp;

                if event.datas.is_empty() {
                    let Some(last_heartbeat) = &self.last_datas else {
                        error!("Empty heartbeat at first.");
                        continue;
                    };

                    info!("Same heartbeat data with last one.");
                    let heart_beat = Event {
                        id: None,
                        timestamp: last_heartbeat.timestamp,
                        duration: Duration::zero(),
                        data: create_heartbeat_data(&last_heartbeat),
                    };

                    self.heartbeat(&heart_beat, pulse_time).await;
                    continue;
                }

                // Update last_timestamp for current images
                for key in event.datas.keys() {
                    self.last_timestamp.insert(*key, timestamp);
                }

                // Check last_datas for missing images and retention
                if let Some(last_datas) = &self.last_datas {
                    for (key, value) in last_datas.datas.iter() {
                        if !event.datas.contains_key(key) {
                            if let Some(last_ts) = self.last_timestamp.get(key) {
                                if timestamp - *last_ts <= self.timeout {
                                    // Keep image if within timeout
                                    event.add_data(*key, value.clone());
                                }
                            }
                        }
                    }
                }

                // Clean up last_timestamp for images no longer being tracked
                self.last_timestamp
                    .retain(|key, _| event.datas.contains_key(key));

                if let Some(last_datas) = &self.last_datas {
                    let finish = Event {
                        id: None,
                        timestamp: timestamp - Duration::milliseconds(1),
                        duration: Duration::zero(),
                        data: create_heartbeat_data(&last_datas),
                    };
                    self.heartbeat(&finish, pulse_time).await;
                }

                let heartbeat = Event {
                    id: None,
                    timestamp,
                    duration: Duration::zero(),
                    data: create_heartbeat_data(&event),
                };

                self.heartbeat(&heartbeat, pulse_time).await;

                self.last_datas = Some(event);
            }
        }))
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
