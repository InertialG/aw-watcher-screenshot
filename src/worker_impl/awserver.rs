use crate::config::AwServerConfig;
use crate::event::ImageEvent;
use crate::worker::TaskProcessor;
use anyhow::{Context, Error, Result};
use reqwest::blocking::Client;
use serde::Serialize;
use std::collections::HashMap;
use tracing::{info, warn};

pub struct AwServerProcessor {
    config: AwServerConfig,
    client: Option<Client>,
    bucket_id: Option<String>,
    initialized: bool,
}

#[derive(Serialize)]
struct BucketCreate {
    client: String,
    #[serde(rename = "type")]
    bucket_type: String,
    hostname: String,
}

#[derive(Serialize)]
struct HeartbeatEvent {
    timestamp: String,
    duration: f64,
    data: HashMap<String, serde_json::Value>,
}

impl AwServerProcessor {
    pub fn new(config: AwServerConfig) -> Self {
        Self {
            config,
            client: None,
            bucket_id: None,
            initialized: false,
        }
    }

    /// Lazy initialization - called on first process() inside spawn_blocking
    fn ensure_initialized(&mut self) -> Result<(), Error> {
        if self.initialized {
            return Ok(());
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        // Get hostname for bucket naming
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unknown".to_string());

        // Use configured bucket_id or generate default
        let bucket_id = self
            .config
            .bucket_id
            .clone()
            .unwrap_or_else(|| format!("aw-watcher-screenshot_{}", hostname));

        // Create bucket if not exists
        let bucket_url = format!("{}/api/0/buckets/{}", self.config.host, bucket_id);

        let bucket_create = BucketCreate {
            client: "aw-watcher-screenshot".to_string(),
            bucket_type: "app.editor.activity".to_string(),
            hostname: hostname.clone(),
        };

        let resp = client.post(&bucket_url).json(&bucket_create).send();

        match resp {
            Ok(r) => {
                if r.status().is_success() || r.status().as_u16() == 304 {
                    info!(
                        "AwServerProcessor initialized with bucket '{}' at {}",
                        bucket_id, self.config.host
                    );
                } else {
                    warn!(
                        "Bucket creation returned status {}: {:?}",
                        r.status(),
                        r.text().unwrap_or_default()
                    );
                }
            }
            Err(e) => {
                warn!(
                    "Failed to create bucket (aw-server may not be running): {}",
                    e
                );
            }
        }

        self.client = Some(client);
        self.bucket_id = Some(bucket_id);
        self.initialized = true;
        Ok(())
    }
}

impl TaskProcessor<ImageEvent, ImageEvent> for AwServerProcessor {
    fn init(&mut self) -> Result<(), Error> {
        // Do nothing here - initialization happens lazily in process()
        // because reqwest::blocking::Client cannot be created in async context
        info!("AwServerProcessor will initialize on first event (lazy init)");
        Ok(())
    }

    fn process(&mut self, event: ImageEvent) -> Result<ImageEvent, Error> {
        // Lazy init on first call (inside spawn_blocking)
        self.ensure_initialized()?;

        let client = self
            .client
            .as_ref()
            .context("HTTP client not initialized")?;

        let bucket_id = self
            .bucket_id
            .as_ref()
            .context("Bucket ID not initialized")?;

        let heartbeat_url = format!(
            "{}/api/0/buckets/{}/heartbeat?pulsetime={}",
            self.config.host, bucket_id, self.config.pulsetime
        );

        // Send one heartbeat per image (per monitor_id)
        for (monitor_id, local_path) in &event.file_paths {
            let mut data = HashMap::new();
            data.insert(
                "event_id".to_string(),
                serde_json::json!(event.id.to_string()),
            );
            data.insert("monitor_id".to_string(), serde_json::json!(monitor_id));
            data.insert("local_path".to_string(), serde_json::json!(local_path));

            let heartbeat = HeartbeatEvent {
                timestamp: event.timestamp.to_rfc3339(),
                duration: 0.0,
                data,
            };

            match client.post(&heartbeat_url).json(&heartbeat).send() {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        warn!(
                            "Heartbeat failed for monitor {} with status {}: {:?}",
                            monitor_id,
                            resp.status(),
                            resp.text().unwrap_or_default()
                        );
                    }
                }
                Err(e) => {
                    warn!("Failed to send heartbeat for monitor {}: {}", monitor_id, e);
                }
            }
        }

        Ok(event)
    }
}
