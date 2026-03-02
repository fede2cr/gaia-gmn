//! Filesystem scanning for night directories and captured files.
//!
//! The capture server writes FF/FS files into per-night subdirectories
//! under `{data_dir}/CapturedFiles/`.  Directory names follow the pattern
//! `{station_id}_{YYYYMMDD_HHMMSS}_UTC`.

use std::path::{Path, PathBuf};

use crate::model::{FileEntry, NightSummary};

/// Scan the `CapturedFiles/` directory for night subdirectories.
///
/// Returns summaries sorted by start time (most recent first).
pub fn scan_nights(data_dir: &Path) -> anyhow::Result<Vec<NightSummary>> {
    let captured_dir = data_dir.join("CapturedFiles");
    if !captured_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut nights = Vec::new();

    for entry in std::fs::read_dir(&captured_dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if !meta.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();

        // Parse the directory name: {station}_{YYYYMMDD}_{HHMMSS}_UTC
        let (station_id, start_time) = parse_night_dir_name(&dir_name);

        // Count files and total size
        let (ff_count, fs_count, total_bytes) = count_night_files(&entry.path());

        nights.push(NightSummary {
            dir_name,
            station_id,
            start_time,
            ff_count,
            fs_count,
            total_bytes,
        });
    }

    // Most recent first
    nights.sort_by(|a, b| b.start_time.cmp(&a.start_time));
    Ok(nights)
}

/// List FF/FS files inside a specific night directory.
pub fn list_night_files(data_dir: &Path, night_dir: &str) -> anyhow::Result<Vec<FileEntry>> {
    // Guard against path traversal
    if night_dir.contains("..") || night_dir.contains('/') || night_dir.contains('\\') {
        anyhow::bail!("Invalid night directory name");
    }

    let dir = data_dir.join("CapturedFiles").join(night_dir);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("FF_") || name.starts_with("FS_") {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            files.push(FileEntry { name, size });
        }
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

/// Check if a specific night directory exists and return its path.
pub fn night_dir_path(data_dir: &Path, night_dir: &str) -> Option<PathBuf> {
    if night_dir.contains("..") || night_dir.contains('/') || night_dir.contains('\\') {
        return None;
    }
    let dir = data_dir.join("CapturedFiles").join(night_dir);
    dir.is_dir().then_some(dir)
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Parse `{station}_{YYYYMMDD}_{HHMMSS}_UTC` into (station_id, formatted_time).
fn parse_night_dir_name(name: &str) -> (String, String) {
    // Split on '_' and try to parse the date/time parts.
    // Typical: "AU000A_20250301_210000_UTC"
    let parts: Vec<&str> = name.split('_').collect();

    if parts.len() >= 4 {
        let station_id = parts[0].to_string();
        let date_part = parts[1]; // YYYYMMDD
        let time_part = parts[2]; // HHMMSS

        if date_part.len() == 8 && time_part.len() == 6 {
            let formatted = format!(
                "{}-{}-{} {}:{}:{} UTC",
                &date_part[0..4],
                &date_part[4..6],
                &date_part[6..8],
                &time_part[0..2],
                &time_part[2..4],
                &time_part[4..6],
            );
            return (station_id, formatted);
        }
    }

    // Fallback: use the full name as both
    (name.to_string(), name.to_string())
}

/// Count FF files, FS files, and total bytes in a night directory.
fn count_night_files(dir: &Path) -> (u32, u32, u64) {
    let mut ff = 0u32;
    let mut fs = 0u32;
    let mut bytes = 0u64;

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            bytes += size;

            if name.starts_with("FF_") {
                ff += 1;
            } else if name.starts_with("FS_") {
                fs += 1;
            }
        }
    }

    (ff, fs, bytes)
}
