//! Frame capture via ffmpeg subprocess.
//!
//! Launches ffmpeg to read from a V4L2 device (or RTSP stream) and pipe
//! raw grayscale frames to stdout.  The caller reads exactly `nframes`
//! frames per block, hands them to the compressor, and loops.
//!
//! This is the "incremental" approach: ffmpeg handles all the camera /
//! codec / V4L2 / GStreamer complexity, and we get clean raw bytes.

use std::io::Read;
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use gaia_gmn_common::config::Config;

/// Handle to the running ffmpeg capture process.
pub struct CaptureHandle {
    child: Child,
    /// Stdout pipe for reading raw frames.
    reader: std::io::BufReader<std::process::ChildStdout>,
    /// Size of one frame in bytes.
    pub frame_size: usize,
}

impl CaptureHandle {
    /// Read exactly one frame (width × height bytes) from the pipe.
    ///
    /// Returns `Ok(true)` if a full frame was read, `Ok(false)` on EOF.
    pub fn read_frame(&mut self, buf: &mut [u8]) -> Result<bool> {
        assert_eq!(buf.len(), self.frame_size);
        match self.reader.read_exact(buf) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// Check whether the child has exited.
    pub fn check_alive(&mut self) -> Option<String> {
        match self.child.try_wait() {
            Ok(Some(status)) => Some(format!("ffmpeg exited with {status}")),
            Ok(None) => None,
            Err(e) => Some(format!("Cannot check ffmpeg: {e}")),
        }
    }

    /// Kill the child process.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Start the ffmpeg capture process.
///
/// ffmpeg reads from the configured video device and outputs raw
/// grayscale (gray8) frames to stdout at the configured resolution
/// and frame rate.
pub fn start(config: &Config) -> Result<CaptureHandle> {
    let frame_size = (config.width * config.height) as usize;
    let is_rtsp =
        config.video_device.starts_with("rtsp://") || config.video_device.starts_with("rtsps://");

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-loglevel", "warning", "-nostdin"]);

    if is_rtsp {
        // RTSP input
        cmd.args([
            "-rtsp_transport",
            "tcp",
            "-timeout",
            "10000000",
            "-i",
            &config.video_device,
        ]);
    } else {
        // V4L2 local camera
        cmd.args([
            "-f",
            "v4l2",
            "-video_size",
            &format!("{}x{}", config.width, config.height),
            "-framerate",
            &format!("{}", config.fps),
            "-i",
            &config.video_device,
        ]);
    }

    // Output: raw grayscale frames to pipe
    cmd.args([
        "-an", // no audio
        "-vf",
        &format!("scale={}:{},format=gray", config.width, config.height),
        "-f",
        "rawvideo",
        "-pix_fmt",
        "gray",
        "-",
    ]);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    info!(
        "Starting ffmpeg: device={}, {}x{}@{:.1}fps, frame_size={}",
        config.video_device, config.width, config.height, config.fps, frame_size,
    );

    let mut child = cmd.spawn().context("Failed to spawn ffmpeg")?;

    let stdout = child.stdout.take().context("ffmpeg stdout not captured")?;

    // Drain stderr in background so we see warnings and the pipe doesn't block.
    if let Some(stderr) = child.stderr.take() {
        std::thread::Builder::new()
            .name("ffmpeg-stderr".into())
            .spawn(move || {
                use std::io::BufRead;
                let reader = std::io::BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(l) if l.is_empty() => {}
                        Ok(l) => warn!("[ffmpeg] {l}"),
                        Err(_) => break,
                    }
                }
                debug!("ffmpeg stderr stream ended");
            })
            .ok();
    }

    // Give ffmpeg a moment to fail on bad config.
    std::thread::sleep(std::time::Duration::from_millis(500));
    match child.try_wait() {
        Ok(Some(status)) => {
            anyhow::bail!("ffmpeg exited immediately with {status} — check VIDEO_DEVICE");
        }
        Ok(None) => {} // still running
        Err(e) => warn!("Cannot check ffmpeg status: {e}"),
    }

    info!("ffmpeg started (pid={})", child.id());

    Ok(CaptureHandle {
        child,
        reader: std::io::BufReader::with_capacity(frame_size * 4, stdout),
        frame_size,
    })
}
