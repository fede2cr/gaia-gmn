//! FF binary file format writer (CAMS-compatible "new" format v2).
//!
//! An FF file contains four H×W uint8 image planes produced by the FTP
//! (Four-frame Temporal Pixel) compression of `nframes` video frames:
//!
//! | Plane    | Description                                           |
//! |----------|-------------------------------------------------------|
//! | maxpixel | Maximum intensity at each pixel across all frames      |
//! | maxframe | Frame index (0–255) in which the maximum was observed  |
//! | avepixel | Mean intensity (excluding top-4 values per pixel)      |
//! | stdpixel | Std-dev of intensity (excluding top-4 values)          |
//!
//! ## Binary layout (v2 / "new" format)
//!
//! ```text
//! i32  version_flag   = -1   (signals new format)
//! u32  nrows
//! u32  ncols
//! u32  nframes
//! u32  first_frame    (always 0 for us)
//! u32  camno          (numeric station id)
//! u32  decimation_fact
//! u32  interleave_flag (0=prog, 1=even/odd, 2=odd/even)
//! u32  fps_millis     (fps × 1000, stored as integer)
//! u8   maxpixel[nrows × ncols]
//! u8   maxframe[nrows × ncols]
//! u8   avepixel[nrows × ncols]
//! u8   stdpixel[nrows × ncols]
//! ```

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use chrono::{DateTime, Utc};

/// Result of the FTP compression for one block of frames.
pub struct FfData {
    pub nrows: u32,
    pub ncols: u32,
    pub nframes: u32,
    pub fps: f64,
    pub station_id: String,
    pub deinterlace_order: i32,
    /// Time of the first frame in this block.
    pub start_time: DateTime<Utc>,

    // Image planes (row-major, H × W).
    pub maxpixel: Vec<u8>,
    pub maxframe: Vec<u8>,
    pub avepixel: Vec<u8>,
    pub stdpixel: Vec<u8>,

    /// Per-field intensity sums.
    pub field_sums: Vec<u32>,
}

impl FfData {
    /// Construct the canonical FF filename.
    ///
    /// Format: `FF_{station}_{YYYYMMDD}_{HHMMSS}_{mmm}_0000000.bin`
    /// where `mmm` is the millisecond part of the start time.
    pub fn filename(&self) -> String {
        let dt = self.start_time;
        let ms = dt.timestamp_subsec_millis();
        // Strip underscores, spaces, colons from station id
        let station: String = self
            .station_id
            .chars()
            .filter(|c| *c != '_' && *c != ' ' && *c != ':')
            .collect();
        format!(
            "FF_{station}_{date}_{time}_{ms:03}_0000000.bin",
            date = dt.format("%Y%m%d"),
            time = dt.format("%H%M%S"),
        )
    }

    /// Write this FF file to the given directory.
    pub fn write_to_dir(&self, dir: &Path) -> Result<String> {
        let filename = self.filename();
        let path = dir.join(&filename);

        let mut f = std::fs::File::create(&path)
            .with_context(|| format!("Cannot create FF file: {}", path.display()))?;

        self.write_bin(&mut f)
            .with_context(|| format!("Error writing FF file: {}", path.display()))?;

        Ok(filename)
    }

    /// Write the binary (v2 / new format) representation.
    fn write_bin<W: Write>(&self, w: &mut W) -> Result<()> {
        let n = (self.nrows * self.ncols) as usize;
        assert_eq!(self.maxpixel.len(), n);
        assert_eq!(self.maxframe.len(), n);
        assert_eq!(self.avepixel.len(), n);
        assert_eq!(self.stdpixel.len(), n);

        // Extract numeric part from station id for camno field.
        let camno: u32 = self
            .station_id
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0);

        let interleave_flag: u32 = match self.deinterlace_order {
            0 => 1, // even-first
            1 => 2, // odd-first
            _ => 0, // progressive
        };

        let fps_millis = (self.fps * 1000.0) as u32;

        // Header
        w.write_i32::<LittleEndian>(-1)?; // version flag (new format)
        w.write_u32::<LittleEndian>(self.nrows)?;
        w.write_u32::<LittleEndian>(self.ncols)?;
        w.write_u32::<LittleEndian>(self.nframes)?;
        w.write_u32::<LittleEndian>(0)?; // first_frame
        w.write_u32::<LittleEndian>(camno)?;
        w.write_u32::<LittleEndian>(1)?; // decimation_fact
        w.write_u32::<LittleEndian>(interleave_flag)?;
        w.write_u32::<LittleEndian>(fps_millis)?;

        // Image planes
        w.write_all(&self.maxpixel)?;
        w.write_all(&self.maxframe)?;
        w.write_all(&self.avepixel)?;
        w.write_all(&self.stdpixel)?;

        w.flush()?;
        Ok(())
    }

    /// Write the field-sum companion file `FS_{...}_fieldsum.txt`.
    pub fn write_field_sums(&self, dir: &Path) -> Result<String> {
        let ff_name = self.filename();
        // Strip "FF_" prefix and ".bin" suffix for the FS filename.
        let base = ff_name
            .strip_prefix("FF_")
            .unwrap_or(&ff_name)
            .strip_suffix(".bin")
            .unwrap_or(&ff_name);
        let fs_name = format!("FS_{base}_fieldsum.txt");
        let path = dir.join(&fs_name);

        let deinterlace = self.deinterlace_order >= 0;
        let divisor: f64 = if deinterlace { 2.0 } else { 1.0 };

        let mut f = std::fs::File::create(&path)
            .with_context(|| format!("Cannot create FS file: {}", path.display()))?;

        writeln!(f, "{fs_name}\n")?;
        writeln!(f, "Frame, Intensity sum")?;
        for (i, &val) in self.field_sums.iter().enumerate() {
            let half_frame = i as f64 / divisor;
            writeln!(f, "{half_frame:.1}, {val}")?;
        }

        Ok(fs_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_ff_filename() {
        let ff = FfData {
            nrows: 720,
            ncols: 1280,
            nframes: 256,
            fps: 25.0,
            station_id: "US_0001".into(),
            deinterlace_order: -1,
            start_time: Utc.with_ymd_and_hms(2026, 3, 1, 20, 15, 30).unwrap(),
            maxpixel: vec![],
            maxframe: vec![],
            avepixel: vec![],
            stdpixel: vec![],
            field_sums: vec![],
        };
        assert_eq!(ff.filename(), "FF_US0001_20260301_201530_000_0000000.bin");
    }

    #[test]
    fn test_write_bin_roundtrip() {
        let n = 4usize; // 2×2
        let ff = FfData {
            nrows: 2,
            ncols: 2,
            nframes: 256,
            fps: 25.0,
            station_id: "1".into(),
            deinterlace_order: -1,
            start_time: Utc::now(),
            maxpixel: vec![100, 200, 150, 50],
            maxframe: vec![10, 20, 30, 40],
            avepixel: vec![80, 90, 70, 60],
            stdpixel: vec![5, 6, 7, 8],
            field_sums: vec![],
        };
        let mut buf = Vec::new();
        ff.write_bin(&mut buf).unwrap();
        // Header: 9 × 4 bytes = 36, then 4 planes × 4 bytes = 16 → total 52
        assert_eq!(buf.len(), 36 + n * 4);
        // Check version flag
        assert_eq!(&buf[0..4], &(-1i32).to_le_bytes());
    }
}
