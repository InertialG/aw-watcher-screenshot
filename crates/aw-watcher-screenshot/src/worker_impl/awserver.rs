use std::collections::HashMap;

use crate::event::CompleteCommand;
use crate::worker::TaskProcessor;
use crate::{config::AwServerConfig, event::AwEvent};
use anyhow::{Context, Error, Result};
use aw_client_lite::AwClient;
use aw_models::Event;
use chrono::Duration;
use serde_json::{Map, Value};

pub struct AwServerProcessor {
    config: AwServerConfig,
    client: Option<AwClient>,
    bucket_id: Option<String>,

    last_datas: Option<HashMap<String, String>>,
}

impl AwServerProcessor {
    pub fn new(config: AwServerConfig) -> Self {
        Self {
            config,
            client: None,
            bucket_id: None,
            last_datas: None,
        }
    }
}

impl TaskProcessor<AwEvent, CompleteCommand> for AwServerProcessor {
    fn init(&mut self) -> Result<(), Error> {
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
            .context("Failed to create bucket")?;

        self.client = Some(client);
        self.bucket_id = Some(bucket_id);

        Ok(())
    }

    fn process(&mut self, event: AwEvent) -> Result<CompleteCommand, Error> {
        let Some(bucket_id) = &self.bucket_id else {
            return Err(anyhow::anyhow!("Bucket ID not initialized"));
        };
        let Some(client) = &self.client else {
            return Err(anyhow::anyhow!("Client not initialized"));
        };

        let (timestamp, mut datas) = event.into_parts();
        let is_empty = datas.is_empty();

        if let Some(last_datas) = &self.last_datas {
            if !is_empty {
                let finish = Event {
                    id: None,
                    timestamp: timestamp - Duration::milliseconds(1),
                    duration: Duration::zero(), // TODO：待验证
                    data: change_datas(&last_datas),
                };
                client
                    .heartbeat(&bucket_id, &finish, 10.0)
                    .context("Failed to send heartbeat")?;
            }

            for (key, value) in last_datas.iter() {
                if !datas.contains_key(key) {
                    datas.insert(key.clone(), value.clone());
                }
            }
        } else {
            if is_empty {
                return Err(anyhow::anyhow!("Empty datas in first event."));
            }
        };

        // 更新 last_datas
        self.last_datas = Some(datas);

        let heartbeat = Event {
            id: None,
            timestamp: timestamp,
            duration: Duration::zero(),
            data: change_datas(self.last_datas.as_ref().unwrap()),
        };

        client
            .heartbeat(&bucket_id, &heartbeat, 10.0)
            .context("Failed to send heartbeat")?;
        Ok(true)
    }
}

fn change_datas(datas: &HashMap<String, String>) -> Map<String, Value> {
    datas
        .into_iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect()
}
