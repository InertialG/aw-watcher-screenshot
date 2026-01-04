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
}

#[derive(Deserialize, Debug, Clone)]
pub struct SqliteConfig {
    pub db_path: String,
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
            },
            sqlite: SqliteConfig {
                db_path: exe_dir
                    .join("aw-watcher-screenshot.db")
                    .to_string_lossy()
                    .into_owned(),
            },
        }
    }
}
