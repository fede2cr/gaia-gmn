//! Gaia GMN Capture Server — captures video frames and compresses them
//! into FF (Four-frame Temporal Pixel) files for meteor detection.
//!
//! This binary:
//! 1. Reads configuration from `gmn.conf` (camera device injected by
//!    gaia-core from the hardware assignment DB via `VIDEO_DEVICE` env var)
//! 2. Starts frame capture via ffmpeg subprocess
//! 3. Compresses every N frames into an FF binary file
//! 4. Runs an axum HTTP server exposing files and a live JPEG preview
//! 5. Registers on mDNS for discovery by gaia-core and processing nodes

mod capture;
mod compression;
mod server;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::Utc;
use tracing::info;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // ── load config ──────────────────────────────────────────────────
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| gaia_gmn_common::config::Config::default_path().to_string());
    let config = gaia_gmn_common::config::load(&PathBuf::from(&config_path))
        .context("Config load failed")?;

    info!(
        "Gaia GMN Capture starting (device={}, {}x{}@{:.1}fps, listen={})",
        config.video_device, config.width, config.height, config.fps, config.listen_addr,
    );

    // Ensure data directories exist
    let captured_dir = config.captured_dir();
    std::fs::create_dir_all(&captured_dir).context("Cannot create CapturedFiles directory")?;

    // ── ctrl-c ───────────────────────────────────────────────────────
    ctrlc::set_handler(move || {
        SHUTDOWN.store(true, Ordering::Relaxed);
        info!("Shutdown signal received");
        std::process::exit(0);
    })
    .context("Cannot set Ctrl-C handler")?;

    // ── shared counters ──────────────────────────────────────────────
    let ff_count = Arc::new(AtomicU64::new(0));
    let frames_captured = Arc::new(AtomicU64::new(0));

    // ── start capture with retries ───────────────────────────────────
    const MAX_RETRIES: u32 = 5;
    const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(10);

    let mut capture_handle: Option<capture::CaptureHandle> = None;
    for attempt in 1..=MAX_RETRIES {
        match capture::start(&config) {
            Ok(h) => {
                info!("Video capture started on attempt {attempt}");
                capture_handle = Some(h);
                break;
            }
            Err(e) => {
                tracing::warn!("Capture attempt {attempt}/{MAX_RETRIES} failed: {e:#}");
                if attempt < MAX_RETRIES {
                    info!("Retrying in {}s…", RETRY_DELAY.as_secs());
                    std::thread::sleep(RETRY_DELAY);
                }
            }
        }
    }

    if capture_handle.is_none() {
        tracing::warn!(
            "All {MAX_RETRIES} capture attempts failed. \
             HTTP server will run without active capture."
        );
    }

    // ── mDNS registration ────────────────────────────────────────────
    let port: u16 = config
        .listen_addr
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8090);

    let discovery = match gaia_gmn_common::discovery::register(
        gaia_gmn_common::discovery::ServiceRole::Capture,
        port,
    ) {
        Ok(h) => {
            info!("mDNS: registered as {}", h.instance_name());
            Some(h)
        }
        Err(e) => {
            tracing::warn!("mDNS registration failed (non-fatal): {e:#}");
            None
        }
    };

    // ── start HTTP server ────────────────────────────────────────────
    let server_state = server::AppState {
        data_dir: config.data_dir.clone(),
        live_jpg_path: config.live_jpg_path(),
        start_time: Instant::now(),
        ff_count: ff_count.clone(),
        frames_captured: frames_captured.clone(),
    };
    let listen_addr = config.listen_addr.clone();

    let server_handle = tokio::spawn(async move {
        if let Err(e) = server::run(server_state, &listen_addr).await {
            tracing::error!("HTTP server error: {e:#}");
        }
    });

    // ── capture + compression loop ───────────────────────────────────
    if let Some(mut handle) = capture_handle {
        let capture_config = config.clone();
        let ff_count_clone = ff_count.clone();
        let frames_captured_clone = frames_captured.clone();

        let capture_thread = std::thread::Builder::new()
            .name("capture-loop".into())
            .spawn(move || {
                capture_loop(
                    &mut handle,
                    &capture_config,
                    &ff_count_clone,
                    &frames_captured_clone,
                );
            })
            .context("Cannot spawn capture thread")?;

        // Wait for either server or capture to finish
        tokio::select! {
            _ = server_handle => {},
            _ = tokio::task::spawn_blocking(move || { capture_thread.join().ok(); }) => {},
        }
    } else {
        // No capture — just run the server
        let _ = server_handle.await;
    }

    // ── cleanup ──────────────────────────────────────────────────────
    if let Some(dh) = discovery {
        dh.shutdown();
    }
    info!("Gaia GMN Capture stopped");
    Ok(())
}

/// Main capture loop: reads frames from ffmpeg, compresses every
/// `ff_nframes` frames into an FF file, saves a live.jpg preview.
fn capture_loop(
    handle: &mut capture::CaptureHandle,
    config: &gaia_gmn_common::config::Config,
    ff_count: &AtomicU64,
    frames_captured: &AtomicU64,
) {
    let nframes = config.ff_nframes as usize;
    let frame_size = handle.frame_size;
    let npix = frame_size; // grayscale: 1 byte per pixel

    // Create a night directory: {station}_{datetime}_UTC
    let now = Utc::now();
    let night_dir_name = format!("{}_{}_UTC", config.station_id, now.format("%Y%m%d_%H%M%S"),);
    let night_dir = config.captured_dir().join(&night_dir_name);
    if let Err(e) = std::fs::create_dir_all(&night_dir) {
        tracing::error!("Cannot create night dir: {e}");
        return;
    }
    info!("Night directory: {}", night_dir.display());

    // Main frame buffer: nframes × frame_size bytes
    let mut frame_buf = vec![0u8; nframes * npix];

    loop {
        if SHUTDOWN.load(Ordering::Relaxed) {
            break;
        }

        let block_start = Utc::now();
        let mut frames_read = 0usize;

        // Read nframes frames from ffmpeg
        for i in 0..nframes {
            let offset = i * npix;
            match handle.read_frame(&mut frame_buf[offset..offset + npix]) {
                Ok(true) => {
                    frames_read += 1;
                    frames_captured.fetch_add(1, Ordering::Relaxed);
                }
                Ok(false) => {
                    tracing::warn!("ffmpeg EOF after {frames_read} frames");
                    break;
                }
                Err(e) => {
                    tracing::error!("Frame read error: {e}");
                    break;
                }
            }

            // Save every 64th frame as live.jpg for preview
            if i % 64 == 0 {
                save_live_jpg(
                    &frame_buf[offset..offset + npix],
                    config.width,
                    config.height,
                    &config.live_jpg_path(),
                );
            }
        }

        if frames_read == 0 {
            tracing::error!("No frames captured — stopping");
            break;
        }

        // If we got fewer than nframes, pad with the last frame
        if frames_read < nframes {
            tracing::warn!("Only {frames_read}/{nframes} frames — padding with last frame");
            let last_offset = (frames_read - 1) * npix;
            for i in frames_read..nframes {
                let offset = i * npix;
                frame_buf.copy_within(last_offset..last_offset + npix, offset);
            }
        }

        // FTP compress
        info!("Compressing {nframes} frames ({frames_read} captured)…");
        let ff = compression::compress_frames(
            &frame_buf,
            config.width,
            config.height,
            config.ff_nframes,
            config.fps,
            &config.station_id,
            config.deinterlace_order,
            block_start,
        );

        // Write FF file
        match ff.write_to_dir(&night_dir) {
            Ok(name) => {
                ff_count.fetch_add(1, Ordering::Relaxed);
                info!("Wrote {name}");
            }
            Err(e) => tracing::error!("Failed to write FF file: {e:#}"),
        }

        // Write field sums
        match ff.write_field_sums(&night_dir) {
            Ok(name) => info!("Wrote {name}"),
            Err(e) => tracing::warn!("Failed to write field sums: {e:#}"),
        }

        // Check if ffmpeg is still alive
        if let Some(msg) = handle.check_alive() {
            tracing::error!("{msg} — stopping capture");
            break;
        }
    }
}

/// Encode a single grayscale frame as JPEG and save to disk.
fn save_live_jpg(frame: &[u8], width: u32, height: u32, path: &std::path::Path) {
    use image::codecs::jpeg::JpegEncoder;
    use image::GrayImage;

    let img = match GrayImage::from_raw(width, height, frame.to_vec()) {
        Some(img) => img,
        None => return,
    };

    // Write to a temp file first, then rename for atomicity.
    let tmp = path.with_extension("tmp");
    let file = match std::fs::File::create(&tmp) {
        Ok(f) => f,
        Err(_) => return,
    };
    let writer = std::io::BufWriter::new(file);
    let encoder = JpegEncoder::new_with_quality(writer, 75);
    if img.write_with_encoder(encoder).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}
