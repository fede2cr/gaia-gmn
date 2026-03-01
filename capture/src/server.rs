//! HTTP server exposing captured FF files, live preview, and health
//! check to the processing server and gaia-core dashboard.
//!
//! Routes:
//!   GET  /api/health            → health check
//!   GET  /api/files             → list FF/FS files in the current night dir
//!   GET  /api/files/:name       → download a file
//!   GET  /api/live.jpg          → latest camera frame as JPEG
//!   GET  /api/status            → capture statistics

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::info;

/// Shared state for route handlers.
#[derive(Clone)]
pub struct AppState {
    pub data_dir: PathBuf,
    pub live_jpg_path: PathBuf,
    pub start_time: Instant,
    pub ff_count: Arc<std::sync::atomic::AtomicU64>,
    pub frames_captured: Arc<std::sync::atomic::AtomicU64>,
}

/// Start the HTTP server.  Blocks until shutdown.
pub async fn run(state: AppState, listen_addr: &str) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/files", get(list_files))
        .route("/api/files/{name}", get(download_file))
        .route("/api/live.jpg", get(live_jpg))
        .route("/api/status", get(status))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = TcpListener::bind(listen_addr).await?;
    info!("Capture HTTP server listening on {listen_addr}");

    axum::serve(listener, app).await?;
    Ok(())
}

// ── Handlers ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    uptime_secs: u64,
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        uptime_secs: state.start_time.elapsed().as_secs(),
    })
}

#[derive(Serialize)]
struct FileInfo {
    name: String,
    size: u64,
}

async fn list_files(State(state): State<AppState>) -> Result<Json<Vec<FileInfo>>, StatusCode> {
    let captured_dir = state.data_dir.join("CapturedFiles");
    let mut files = Vec::new();

    // List FF and FS files from the most recent night directory.
    let night_dir = match most_recent_night_dir(&captured_dir).await {
        Some(d) => d,
        None => return Ok(Json(files)),
    };

    let mut entries = tokio::fs::read_dir(&night_dir)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("FF_") || name.starts_with("FS_") {
            let meta = match entry.metadata().await {
                Ok(m) => m,
                Err(_) => continue,
            };
            files.push(FileInfo {
                name,
                size: meta.len(),
            });
        }
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(files))
}

async fn download_file(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    // Prevent path traversal
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let captured_dir = state.data_dir.join("CapturedFiles");
    let night_dir = most_recent_night_dir(&captured_dir)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let path = night_dir.join(&name);

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let content_type = if name.ends_with(".bin") {
        "application/octet-stream"
    } else {
        "text/plain"
    };

    Ok((
        [(axum::http::header::CONTENT_TYPE, content_type)],
        Body::from(bytes),
    ))
}

async fn live_jpg(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    let bytes = tokio::fs::read(&state.live_jpg_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok((
        [(axum::http::header::CONTENT_TYPE, "image/jpeg")],
        Body::from(bytes),
    ))
}

#[derive(Serialize)]
struct CaptureStatus {
    uptime_secs: u64,
    ff_files_written: u64,
    total_frames: u64,
}

async fn status(State(state): State<AppState>) -> Json<CaptureStatus> {
    Json(CaptureStatus {
        uptime_secs: state.start_time.elapsed().as_secs(),
        ff_files_written: state.ff_count.load(Ordering::Relaxed),
        total_frames: state.frames_captured.load(Ordering::Relaxed),
    })
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Find the most recently created directory under `CapturedFiles/`.
async fn most_recent_night_dir(captured_dir: &std::path::Path) -> Option<PathBuf> {
    let mut entries = tokio::fs::read_dir(captured_dir).await.ok()?;
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;

    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Ok(meta) = entry.metadata().await {
            if meta.is_dir() {
                let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                if newest.as_ref().is_none_or(|(_, t)| modified > *t) {
                    newest = Some((entry.path(), modified));
                }
            }
        }
    }
    newest.map(|(p, _)| p)
}
