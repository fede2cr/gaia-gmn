//! Configuration parsing – reads a KEY=VALUE file (same format as RMS
//! `.config` simplified to the capture-relevant keys, plus Gaia-specific
//! additions).
//!
//! Environment variables override file values so that gaia-core can
//! inject hardware assignments (e.g. VIDEO_DEVICE) at container start.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::info;

/// Capture configuration.
#[derive(Debug, Clone)]
pub struct Config {
    // ── station identity ─────────────────────────────────────────────
    /// Station code (e.g. "US0001"), used in FF filenames.
    pub station_id: String,

    // ── camera ───────────────────────────────────────────────────────
    /// V4L2 device path (e.g. "/dev/video0") or RTSP URL.
    pub video_device: String,
    /// Frame width.
    pub width: u32,
    /// Frame height.
    pub height: u32,
    /// Frames per second.
    pub fps: f64,

    // ── capture ──────────────────────────────────────────────────────
    /// Number of frames compressed into a single FF file.
    pub ff_nframes: u32,
    /// Deinterlace order: -1 = progressive (no deinterlace),
    /// 0 = even-first, 1 = odd-first.
    pub deinterlace_order: i32,

    // ── directories ──────────────────────────────────────────────────
    /// Root data directory.
    pub data_dir: PathBuf,

    // ── network ──────────────────────────────────────────────────────
    /// Address the HTTP server listens on.
    pub listen_addr: String,

    // ── location (for sunrise/sunset scheduling) ─────────────────────
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
}

impl Config {
    pub fn default_path() -> &'static str {
        "/etc/gaia/gmn.conf"
    }

    /// The CapturedFiles subdirectory where FF files are written.
    pub fn captured_dir(&self) -> PathBuf {
        self.data_dir.join("CapturedFiles")
    }

    /// Path to the live preview JPEG.
    pub fn live_jpg_path(&self) -> PathBuf {
        self.data_dir.join("live.jpg")
    }
}

/// Parse a KEY=VALUE configuration file.  Lines starting with `#` or `;`
/// are comments.  Values may be optionally double-quoted.  Unknown keys
/// are silently ignored.  Environment variables override config values.
pub fn load(path: &Path) -> Result<Config> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read config: {}", path.display()))?;

    let map = parse_conf(&text);
    info!("Loaded config from {}", path.display());

    // Environment variables override config-file values.
    let get = |key: &str| -> Option<String> {
        std::env::var(key)
            .ok()
            .filter(|v| !v.is_empty())
            .or_else(|| map.get(key).cloned())
    };
    let get_f64 = |key: &str, default: f64| -> f64 {
        get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    };
    let get_u32 = |key: &str, default: u32| -> u32 {
        get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    };
    let get_i32 = |key: &str, default: i32| -> i32 {
        get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    };

    Ok(Config {
        station_id: get("STATION_ID").unwrap_or_else(|| "XX0001".into()),

        video_device: get("VIDEO_DEVICE").unwrap_or_else(|| "/dev/video0".into()),
        width: get_u32("WIDTH", 1280),
        height: get_u32("HEIGHT", 720),
        fps: get_f64("FPS", 25.0),

        ff_nframes: get_u32("FF_NFRAMES", 256),
        deinterlace_order: get_i32("DEINTERLACE_ORDER", -1),

        data_dir: PathBuf::from(get("DATA_DIR").unwrap_or_else(|| "/data".into())),

        listen_addr: get("CAPTURE_LISTEN_ADDR").unwrap_or_else(|| "0.0.0.0:8090".into()),

        latitude: get_f64("LATITUDE", 0.0),
        longitude: get_f64("LONGITUDE", 0.0),
        elevation: get_f64("ELEVATION", 0.0),
    })
}

/// Parse a simple KEY=VALUE config file into a HashMap.
fn parse_conf(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if let Some((key, val)) = trimmed.split_once('=') {
            let key = key.trim().to_string();
            let val = val.trim().trim_matches('"').to_string();
            map.insert(key, val);
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_conf() {
        let text = "# comment\nSTATION_ID=US0001\nWIDTH = 1920\nFPS=25.0\n";
        let map = parse_conf(text);
        assert_eq!(map.get("STATION_ID").unwrap(), "US0001");
        assert_eq!(map.get("WIDTH").unwrap(), "1920");
        assert_eq!(map.get("FPS").unwrap(), "25.0");
    }
}
