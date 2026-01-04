use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub trigger: TriggerConfig,
    pub capture: CaptureConfig,
    pub cache: CacheConfig,
    pub sqlite: SqliteConfig,
    pub s3: Option<S3Config>,
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
pub struct CacheConfig {
    pub cache_dir: String,
    /// WebP quality (1-100). Use 100 for lossless, lower values for lossy compression.
    /// Default is 75 which provides good balance between quality and file size.
    #[serde(default = "default_webp_quality")]
    pub webp_quality: u8,
}

fn default_webp_quality() -> u8 {
    75
}

#[derive(Deserialize, Debug, Clone)]
pub struct SqliteConfig {
    pub db_path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct S3Config {
    pub enabled: bool,
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    #[serde(default = "default_region")]
    pub region: String,
    #[serde(default)]
    pub key_prefix: Option<String>,
}

fn default_region() -> String {
    "auto".to_string()
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).context("Failed to read config file")?;
        let config: Config = toml::from_str(&content).context("Failed to parse config file")?;
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
            sqlite: SqliteConfig {
                db_path: exe_dir
                    .join("aw-watcher-screenshot.db")
                    .to_string_lossy()
                    .into_owned(),
            },
            s3: None,
        }
    }
}
