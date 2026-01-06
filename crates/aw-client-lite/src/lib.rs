use anyhow::{Context, Result};
use aw_models::{Bucket, Event};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct AwClient {
    client: reqwest::Client,
    api_url: String,
}

impl AwClient {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_url: format!("http://{}:{}/api/0", host, port),
        }
    }

    pub async fn create_bucket<T: serde::Serialize + ?Sized>(&self, bucket: &T) -> Result<()> {
        let val = serde_json::to_value(bucket)?;
        let bucket_id = val
            .get("id")
            .and_then(|v| v.as_str())
            .context("Bucket object must have an 'id' field")?;

        let url = format!("{}/buckets/{}", self.api_url, bucket_id);
        self.client
            .post(&url)
            .json(bucket)
            .send()
            .await
            .context("Failed to send create bucket request")?
            .error_for_status()
            .context("Failed to create bucket")?;
        Ok(())
    }

    pub async fn delete_bucket(&self, bucket_id: &str) -> Result<()> {
        let url = format!("{}/buckets/{}", self.api_url, bucket_id);
        self.client
            .delete(&url)
            .send()
            .await
            .context("Failed to send delete bucket request")?
            .error_for_status()
            .context("Failed to delete bucket")?;
        Ok(())
    }

    pub async fn heartbeat(&self, bucket_id: &str, event: &Event, pulsetime: f64) -> Result<()> {
        let url = format!(
            "{}/buckets/{}/heartbeat?pulsetime={}",
            self.api_url, bucket_id, pulsetime
        );
        self.client
            .post(&url)
            .json(event)
            .send()
            .await
            .context("Failed to send heartbeat request")?
            .error_for_status()
            .context("Failed to send heartbeat")?;
        Ok(())
    }

    pub async fn insert_event(&self, bucket_id: &str, event: &Event) -> Result<()> {
        let url = format!("{}/buckets/{}/events", self.api_url, bucket_id);
        let events = vec![event];
        self.client
            .post(&url)
            .json(&events)
            .send()
            .await
            .context("Failed to send insert event request")?
            .error_for_status()
            .context("Failed to insert event")?;
        Ok(())
    }

    pub async fn get_events(
        &self,
        bucket_id: &str,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: Option<u64>,
    ) -> Result<Vec<Event>> {
        let url = format!("{}/buckets/{}/events", self.api_url, bucket_id);
        let mut params = Vec::new();
        if let Some(s) = start {
            params.push(("start", s.to_rfc3339()));
        }
        if let Some(e) = end {
            params.push(("end", e.to_rfc3339()));
        }
        if let Some(l) = limit {
            params.push(("limit", l.to_string()));
        }

        let resp = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .context("Failed to send get events request")?
            .error_for_status()
            .context("Get events returned error status")?;

        let events = resp
            .json::<Vec<Event>>()
            .await
            .context("Failed to deserialize events")?;
        Ok(events)
    }

    pub async fn get_buckets(&self) -> Result<HashMap<String, Bucket>> {
        let url = format!("{}/buckets", self.api_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send get buckets request")?
            .error_for_status()
            .context("Get buckets returned error status")?;

        let buckets = resp
            .json::<HashMap<String, Bucket>>()
            .await
            .context("Failed to deserialize buckets")?;
        Ok(buckets)
    }

    pub async fn get_bucket(&self, bucket_id: &str) -> Result<Bucket> {
        let url = format!("{}/buckets/{}", self.api_url, bucket_id);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send get bucket request")?
            .error_for_status()
            .context("Get bucket returned error status")?;

        let bucket = resp
            .json::<Bucket>()
            .await
            .context("Failed to deserialize bucket")?;
        Ok(bucket)
    }

    pub async fn get_info(&self) -> Result<Info> {
        let url = format!("{}/info", self.api_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send get info request")?
            .error_for_status()
            .context("Get info returned error status")?;

        let info = resp
            .json::<Info>()
            .await
            .context("Failed to deserialize info")?;
        Ok(info)
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct Info {
    pub hostname: String,
    pub testing: bool,
}
