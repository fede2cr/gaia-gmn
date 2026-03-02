//! Domain types shared between server and client (compiled for both
//! native and wasm32 targets).

use serde::{Deserialize, Serialize};

// ── Capture Status ──────────────────────────────────────────────────

/// Real-time capture status returned by the capture container's
/// `/api/status` endpoint.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CaptureStatus {
    pub uptime_secs: u64,
    pub ff_files_written: u64,
    pub total_frames: u64,
}

// ── Files ───────────────────────────────────────────────────────────

/// An FF or FS file listed by the capture server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
}

impl FileEntry {
    /// Human-readable file size (B / KB / MB).
    pub fn human_size(&self) -> String {
        human_size(self.size)
    }

    /// `true` for FF binary files (compressed frame data).
    pub fn is_ff(&self) -> bool {
        self.name.starts_with("FF_")
    }

    /// `true` for FS field-sum text files.
    pub fn is_fs(&self) -> bool {
        self.name.starts_with("FS_")
    }
}

// ── Nights ──────────────────────────────────────────────────────────

/// Summary of a single capture night as seen on the filesystem.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NightSummary {
    /// Directory name, e.g. `"AU000A_20250301_210000_UTC"`.
    pub dir_name: String,
    /// Station ID extracted from the directory name.
    pub station_id: String,
    /// Start date/time formatted as `"2025-03-01 21:00:00 UTC"`.
    pub start_time: String,
    /// Number of FF files in the directory.
    pub ff_count: u32,
    /// Number of FS files in the directory.
    pub fs_count: u32,
    /// Sum of all file sizes in the directory.
    pub total_bytes: u64,
}

impl NightSummary {
    pub fn human_size(&self) -> String {
        human_size(self.total_bytes)
    }
}

// ── Detections ──────────────────────────────────────────────────────

/// A confirmed (or candidate) meteor detection stored in the local
/// SQLite database.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeteorDetection {
    pub id: i64,
    /// Night directory this detection belongs to.
    pub night_dir: String,
    /// UTC timestamp of the detection.
    pub timestamp: String,
    /// FF file where the meteor streak appears.
    pub ff_file: String,
    /// Right ascension (degrees), if solved.
    pub ra_deg: Option<f64>,
    /// Declination (degrees), if solved.
    pub dec_deg: Option<f64>,
    /// Apparent magnitude estimate.
    pub magnitude: Option<f64>,
    /// Duration of the streak in seconds.
    pub duration_secs: Option<f64>,
    /// Number of frames the meteor spans.
    pub num_frames: Option<u32>,
    /// Whether the detection has been confirmed by the solver.
    pub confirmed: bool,
}

// ── Station Info ────────────────────────────────────────────────────

/// Station configuration summary returned to the dashboard.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StationInfo {
    pub station_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
    pub resolution: String,
    pub fps: f64,
    pub ff_nframes: u32,
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Format bytes into a human-readable string.
pub fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
