//! Leptos server functions – executed on the server, called from the UI.
//!
//! Functions that proxy to the capture container's HTTP API use `reqwest`.
//! Functions that access local data use the `server::db` and `server::fs`
//! modules directly.

use leptos::*;

use crate::model::{
    CaptureStatus, FileEntry, MeteorDetection, NightSummary, StationInfo,
};

// ── Helpers (SSR only) ──────────────────────────────────────────────

#[cfg(feature = "ssr")]
fn capture_api_url() -> String {
    std::env::var("CAPTURE_API_URL").unwrap_or_else(|_| "http://localhost:8089".into())
}

#[cfg(feature = "ssr")]
fn data_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(
        std::env::var("GAIA_DATA_DIR").unwrap_or_else(|_| "/data".into()),
    )
}

#[cfg(feature = "ssr")]
fn db_path() -> std::path::PathBuf {
    std::path::PathBuf::from(
        std::env::var("GAIA_DB_PATH").unwrap_or_else(|_| "/data/detections.db".into()),
    )
}

// ── Capture API proxies ─────────────────────────────────────────────

/// Fetch capture status from the capture container's HTTP API.
#[server(GetCaptureStatus, "/api")]
pub async fn get_capture_status() -> Result<CaptureStatus, ServerFnError> {
    let url = capture_api_url();
    let resp = reqwest::get(format!("{url}/api/status"))
        .await
        .map_err(|e| ServerFnError::new(format!("Cannot reach capture server: {e}")))?;

    let status: CaptureStatus = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Invalid status response: {e}")))?;
    Ok(status)
}

/// List FF/FS files from the capture server (current night).
#[server(GetCaptureFiles, "/api")]
pub async fn get_capture_files() -> Result<Vec<FileEntry>, ServerFnError> {
    let url = capture_api_url();
    let resp = reqwest::get(format!("{url}/api/files"))
        .await
        .map_err(|e| ServerFnError::new(format!("Cannot reach capture server: {e}")))?;

    let files: Vec<FileEntry> = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Invalid files response: {e}")))?;
    Ok(files)
}

// ── Night browsing (filesystem) ─────────────────────────────────────

/// List all capture nights from the filesystem.
#[server(GetNights, "/api")]
pub async fn get_nights() -> Result<Vec<NightSummary>, ServerFnError> {
    let data = data_dir();
    crate::server::fs::scan_nights(&data)
        .map_err(|e| ServerFnError::new(format!("Cannot scan nights: {e}")))
}

/// List files for a specific night directory.
#[server(GetNightFiles, "/api")]
pub async fn get_night_files(night_dir: String) -> Result<Vec<FileEntry>, ServerFnError> {
    let data = data_dir();
    crate::server::fs::list_night_files(&data, &night_dir)
        .map_err(|e| ServerFnError::new(format!("Cannot list night files: {e}")))
}

// ── Detections (SQLite) ─────────────────────────────────────────────

/// Fetch the most recent meteor detections.
#[server(GetRecentDetections, "/api")]
pub async fn get_recent_detections(
    limit: Option<u32>,
) -> Result<Vec<MeteorDetection>, ServerFnError> {
    let db = db_path();
    let limit = limit.unwrap_or(50);
    crate::server::db::recent_detections(&db, limit)
        .map_err(|e| ServerFnError::new(format!("Detection query failed: {e}")))
}

/// Fetch detections for a specific night.
#[server(GetNightDetections, "/api")]
pub async fn get_night_detections(
    night_dir: String,
) -> Result<Vec<MeteorDetection>, ServerFnError> {
    let db = db_path();
    crate::server::db::night_detections(&db, &night_dir)
        .map_err(|e| ServerFnError::new(format!("Night detection query failed: {e}")))
}

/// Get total and confirmed detection counts.
#[server(GetDetectionCounts, "/api")]
pub async fn get_detection_counts() -> Result<(u64, u64), ServerFnError> {
    let db = db_path();
    crate::server::db::detection_counts(&db)
        .map_err(|e| ServerFnError::new(format!("Count query failed: {e}")))
}

// ── Station info ────────────────────────────────────────────────────

/// Read station configuration from environment variables set by
/// gaia-core at container launch.
#[server(GetStationInfo, "/api")]
pub async fn get_station_info() -> Result<StationInfo, ServerFnError> {
    let station_id =
        std::env::var("STATION_ID").unwrap_or_else(|_| "Unknown".into());
    let latitude: f64 = std::env::var("LATITUDE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let longitude: f64 = std::env::var("LONGITUDE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let elevation: f64 = std::env::var("ELEVATION")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let width: u32 = std::env::var("WIDTH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1280);
    let height: u32 = std::env::var("HEIGHT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(720);
    let fps: f64 = std::env::var("FPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(25.0);
    let ff_nframes: u32 = std::env::var("FF_NFRAMES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(256);

    Ok(StationInfo {
        station_id,
        latitude,
        longitude,
        elevation,
        resolution: format!("{width}×{height}"),
        fps,
        ff_nframes,
    })
}
