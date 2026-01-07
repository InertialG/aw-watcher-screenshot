use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub trigger: TriggerConfig,
    pub capture: CaptureConfig,
    pub cache: CacheConfig,
    pub s3: S3Config,
    pub aw_server: AwServerConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TriggerConfig {
    pub interval_secs: u64,
    pub timeout_secs: Option<u64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CaptureConfig {
    pub force_interval_secs: u64,
    pub dhash_threshold: u32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct CacheConfig {
    pub cache_dir: String,
    /// WebP quality (1-100). Use 100 for lossless, lower values for lossy compression.
    /// Default is 75 which provides good balance between quality and file size.
    pub webp_quality: u8,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: "cache".to_string(),
            webp_quality: 75,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct S3Config {
    pub enabled: bool,
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub key_prefix: Option<String>,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "".to_string(),
            bucket: "".to_string(),
            access_key: "".to_string(),
            secret_key: "".to_string(),
            region: "".to_string(),
            key_prefix: None,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct AwServerConfig {
    pub host: String,
    pub port: u16,
    pub bucket_id: String,
    pub hostname: String,
    pub timeout_secs: Option<u64>,
    pub pulse_time: Option<f64>,
}

impl Default for AwServerConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5600,
            bucket_id: "aw-watcher-screenshot".to_string(),
            hostname: hostname::get()
                .ok()
                .and_then(|s| s.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string()),
            timeout_secs: Some(60),
            pulse_time: Some(10.0),
        }
    }
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).context("Failed to read config file")?;
        let mut config: Config = toml::from_str(&content).context("Failed to parse config file")?;

        // aw的pulsetime应当比截图的触发间隔大4-5倍
        if let Some(pulse_time) = &config.aw_server.pulse_time {
            let trigger_interval = config.trigger.interval_secs as f64;
            if *pulse_time < trigger_interval * 4.0 {
                return Err(anyhow::anyhow!(
                    "pulse_time shall be greater than trigger interval"
                ));
            }
        } else {
            config.aw_server.pulse_time = Some(config.trigger.interval_secs as f64 * 4.0);
        }
        Ok(config)
    }

    pub fn default_config() -> Self {
        let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        let exe_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));

        Self {
            trigger: TriggerConfig {
                interval_secs: 2,
                timeout_secs: Some(20),
            },
            capture: CaptureConfig {
                force_interval_secs: 60,
                dhash_threshold: 10,
            },
            cache: CacheConfig {
                cache_dir: exe_dir.join("cache").to_string_lossy().into_owned(),
                webp_quality: 75,
            },
            s3: S3Config::default(),
            aw_server: AwServerConfig::default(),
        }
    }
}
