use crate::event::ImageEvent;
use crate::worker::TaskProcessor;
use anyhow::{Context, Error, Result};
use rusqlite::Connection;
use std::path::PathBuf;
use tracing::{error, info};

pub struct SqliteProcessor {
    db_path: Option<PathBuf>,
    conn: Option<Connection>,
}

impl TaskProcessor<ImageEvent, ImageEvent> for SqliteProcessor {
    fn init(&mut self) -> Result<(), Error> {
        let db_path = if let Some(ref path) = self.db_path {
            path.clone()
        } else {
            std::env::current_dir()?.join("aw-watcher-screenshot.db")
        };
        self.db_path = Some(db_path.clone());

        let conn = Connection::open(&db_path).context("Failed to open SQLite database")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL
            )",
            [],
        )
        .context("Failed to create events table")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS focus_windows (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id TEXT NOT NULL,
                title TEXT,
                app_name TEXT,
                window_id INTEGER,
                monitor_id INTEGER,
                FOREIGN KEY(event_id) REFERENCES events(id)
            )",
            [],
        )
        .context("Failed to create focus_windows table")?;

        self.conn = Some(conn);
        info!("SqliteProcessor initialized with db at {:?}", db_path);
        Ok(())
    }

    fn process(&mut self, event: ImageEvent) -> Result<ImageEvent, Error> {
        let conn = self
            .conn
            .as_ref()
            .context("Sqlite connection not initialized")?;

        let event_id = event.get_id().to_string();
        let timestamp = event.timestamp.to_rfc3339();

        conn.execute(
            "INSERT INTO events (id, timestamp) VALUES (?1, ?2)",
            [&event_id, &timestamp],
        )
        .context("Failed to insert event")?;

        if let Some(fw) = &event.focus_window {
            conn.execute(
                "INSERT INTO focus_windows (event_id, title, app_name, window_id, monitor_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                (
                    &event_id,
                    &fw.title,
                    &fw.app_name,
                    fw.id,
                    fw.current_monitor,
                ),
            )
            .context("Failed to insert focus window")?;
        }

        Ok(event)
    }
}

use crate::config::SqliteConfig;

impl SqliteProcessor {
    pub fn new(config: SqliteConfig) -> Self {
        let db_path = Some(PathBuf::from(config.db_path));
        Self {
            db_path,
            conn: None,
        }
    }
}
