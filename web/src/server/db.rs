//! SQLite database for meteor detection storage.
//!
//! The capture pipeline writes FF files; the (future) processing pipeline
//! will populate the `detections` table with confirmed/candidate meteors.
//! This module creates the schema and provides read/write helpers.

use std::path::Path;

use rusqlite::{params, Connection, OpenFlags};

use crate::model::MeteorDetection;

// ── Schema ──────────────────────────────────────────────────────────

/// Ensure the database file exists and the schema is up to date.
pub fn ensure_schema(path: &Path) -> anyhow::Result<()> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS detections (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            night_dir     TEXT    NOT NULL,
            timestamp     TEXT    NOT NULL,
            ff_file       TEXT    NOT NULL,
            ra_deg        REAL,
            dec_deg       REAL,
            magnitude     REAL,
            duration_secs REAL,
            num_frames    INTEGER,
            confirmed     INTEGER NOT NULL DEFAULT 0,
            created_at    TEXT    NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_det_night   ON detections(night_dir);
        CREATE INDEX IF NOT EXISTS idx_det_ts       ON detections(timestamp);
        CREATE INDEX IF NOT EXISTS idx_det_confirmed ON detections(confirmed);",
    )?;

    tracing::info!("Detection DB schema ready at {}", path.display());
    Ok(())
}

// ── Queries ─────────────────────────────────────────────────────────

fn open_ro(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(std::time::Duration::from_secs(2))?;
    Ok(conn)
}

/// Fetch the most recent detections across all nights.
pub fn recent_detections(path: &Path, limit: u32) -> anyhow::Result<Vec<MeteorDetection>> {
    let conn = open_ro(path)?;
    let mut stmt = conn.prepare(
        "SELECT id, night_dir, timestamp, ff_file,
                ra_deg, dec_deg, magnitude, duration_secs,
                num_frames, confirmed
         FROM detections
         ORDER BY timestamp DESC
         LIMIT ?1",
    )?;

    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(MeteorDetection {
                id: row.get(0)?,
                night_dir: row.get(1)?,
                timestamp: row.get(2)?,
                ff_file: row.get(3)?,
                ra_deg: row.get(4)?,
                dec_deg: row.get(5)?,
                magnitude: row.get(6)?,
                duration_secs: row.get(7)?,
                num_frames: row.get(8)?,
                confirmed: row.get::<_, i32>(9)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// Fetch detections for a specific night directory.
pub fn night_detections(path: &Path, night_dir: &str) -> anyhow::Result<Vec<MeteorDetection>> {
    let conn = open_ro(path)?;
    let mut stmt = conn.prepare(
        "SELECT id, night_dir, timestamp, ff_file,
                ra_deg, dec_deg, magnitude, duration_secs,
                num_frames, confirmed
         FROM detections
         WHERE night_dir = ?1
         ORDER BY timestamp ASC",
    )?;

    let rows = stmt
        .query_map(params![night_dir], |row| {
            Ok(MeteorDetection {
                id: row.get(0)?,
                night_dir: row.get(1)?,
                timestamp: row.get(2)?,
                ff_file: row.get(3)?,
                ra_deg: row.get(4)?,
                dec_deg: row.get(5)?,
                magnitude: row.get(6)?,
                duration_secs: row.get(7)?,
                num_frames: row.get(8)?,
                confirmed: row.get::<_, i32>(9)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// Count total detections and confirmed detections.
pub fn detection_counts(path: &Path) -> anyhow::Result<(u64, u64)> {
    let conn = open_ro(path)?;
    let total: u64 = conn.query_row("SELECT COUNT(*) FROM detections", [], |r| r.get(0))?;
    let confirmed: u64 = conn.query_row(
        "SELECT COUNT(*) FROM detections WHERE confirmed = 1",
        [],
        |r| r.get(0),
    )?;
    Ok((total, confirmed))
}

/// Count detections for a given night.
pub fn night_detection_count(path: &Path, night_dir: &str) -> anyhow::Result<u32> {
    let conn = open_ro(path)?;
    let count: u32 = conn.query_row(
        "SELECT COUNT(*) FROM detections WHERE night_dir = ?1",
        params![night_dir],
        |r| r.get(0),
    )?;
    Ok(count)
}
