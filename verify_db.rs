use anyhow::Result;
use rusqlite::Connection;

fn main() -> Result<()> {
    let conn = Connection::open("aw-watcher-screenshot.db")?;

    let mut stmt = conn.prepare("SELECT count(*) FROM events")?;
    let count: i32 = stmt.query_row([], |row| row.get(0))?;

    println!("Events count: {}", count);

    let mut stmt = conn.prepare("SELECT count(*) FROM focus_windows")?;
    let fw_count: i32 = stmt.query_row([], |row| row.get(0))?;

    println!("Focus windows count: {}", fw_count);

    Ok(())
}
