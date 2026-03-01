//! FTP (Four-frame Temporal Pixel) compression.
//!
//! Takes a block of N grayscale frames (typically 256) and compresses them
//! into four summary planes per pixel:
//!
//! - **maxpixel**: maximum intensity value
//! - **maxframe**: frame index where the maximum was observed (randomised
//!   tie-breaking so that meteor trails are not biased toward later frames)
//! - **avepixel**: mean intensity, excluding the top-4 brightest values
//!   (removes meteor/satellite wake contamination)
//! - **stdpixel**: standard deviation, likewise excluding the top-4
//!
//! Additionally produces per-field intensity sums used downstream for
//! cloud/weather detection.
//!
//! This is a faithful Rust translation of `RMS/CompressionCy.pyx`.

use chrono::{DateTime, Utc};
use gaia_gmn_common::ff::FfData;

/// Compress a block of grayscale frames into an [`FfData`] structure.
///
/// # Arguments
/// - `frames`: row-major `[nframes][height][width]` u8 pixel data
/// - `width`, `height`, `nframes`: dimensions
/// - `fps`: capture frame rate
/// - `station_id`: station code for the FF filename
/// - `deinterlace_order`: -1 = progressive, 0 = even-first, 1 = odd-first
/// - `start_time`: UTC timestamp of the first frame in this block
#[allow(clippy::too_many_arguments)]
pub fn compress_frames(
    frames: &[u8],
    width: u32,
    height: u32,
    nframes: u32,
    fps: f64,
    station_id: &str,
    deinterlace_order: i32,
    start_time: DateTime<Utc>,
) -> FfData {
    let w = width as usize;
    let h = height as usize;
    let n = nframes as usize;
    let npix = h * w;

    assert_eq!(frames.len(), n * npix, "frame buffer size mismatch");

    let mut maxpixel = vec![0u8; npix];
    let mut maxframe = vec![0u8; npix];
    let mut avepixel = vec![0u8; npix];
    let mut stdpixel = vec![0u8; npix];

    // Deinterlace multiplier: 2 if deinterlacing, 1 otherwise.
    let deinterlace_mult: usize = if deinterlace_order >= 0 { 2 } else { 1 };
    let mut field_sums = vec![0u32; n * deinterlace_mult];

    // Pre-compute random table (same PRNG as CompressionCy.pyx).
    let mut random_table = [0u8; 65536];
    {
        let mut arand: u32 = 0;
        for entry in &mut random_table {
            arand = (arand.wrapping_mul(32719).wrapping_add(3)) % 32749;
            *entry = (32767.0 / (1 + arand % 32767) as f64) as u8;
        }
    }

    let n_minus_4 = if n >= 4 { n - 4 } else { 1 };
    let n_minus_5 = if n >= 5 { n - 5 } else { 1 };

    let mut rand_count: u16 = 1;

    for y in 0..h {
        for x in 0..w {
            let pix_idx = y * w + x;

            let mut acc: u32 = 0;
            let mut var: u64 = 0;
            let mut max_val: u32 = 0;
            let mut max_val_2: u32 = 0;
            let mut max_val_3: u32 = 0;
            let mut max_val_4: u32 = 0;
            let mut max_frame_idx: u32 = 0;
            let mut num_equal: u32 = 0;

            for frame_idx in 0..n {
                let pixel = frames[frame_idx * npix + pix_idx] as u32;
                acc += pixel;
                var += (pixel as u64) * (pixel as u64);

                if pixel > max_val {
                    // Shift down the top-4 tracker
                    max_val_4 = max_val_3;
                    max_val_3 = max_val_2;
                    max_val_2 = max_val;
                    max_val = pixel;

                    max_frame_idx = frame_idx as u32;
                    num_equal = 1;
                } else {
                    // Randomised tie-breaking for the max frame
                    if pixel == max_val {
                        num_equal += 1;
                        rand_count = rand_count.wrapping_add(1);
                        if num_equal <= random_table[rand_count as usize] as u32 {
                            max_frame_idx = frame_idx as u32;
                        }
                    }

                    // Track top-4 max values (for wake exclusion)
                    if pixel > max_val_2 {
                        max_val_4 = max_val_3;
                        max_val_3 = max_val_2;
                        max_val_2 = pixel;
                    } else if pixel > max_val_3 {
                        max_val_4 = max_val_3;
                        max_val_3 = pixel;
                    } else if pixel > max_val_4 {
                        max_val_4 = pixel;
                    }
                }

                // Field intensity sums
                let fieldsum_idx = deinterlace_mult * frame_idx
                    + if deinterlace_mult == 2 {
                        ((y as i32 + deinterlace_order) % 2) as usize
                    } else {
                        0
                    };
                if fieldsum_idx < field_sums.len() {
                    field_sums[fieldsum_idx] += pixel;
                }
            }

            // Mean without top-4 max values
            let acc_adj = acc - (max_val + max_val_2 + max_val_3 + max_val_4);
            let mean = acc_adj / n_minus_4 as u32;

            // Stddev without top-4 max values
            let var_top4 = (max_val as u64) * (max_val as u64)
                + (max_val_2 as u64) * (max_val_2 as u64)
                + (max_val_3 as u64) * (max_val_3 as u64)
                + (max_val_4 as u64) * (max_val_4 as u64);
            let var_adj = var - var_top4;
            // var_adj - acc_adj * mean = var_adj - acc_adj * acc_adj / n_minus_4
            let variance = var_adj.saturating_sub((acc_adj as u64) * (mean as u64));
            let std = ((variance as f64 / n_minus_5 as f64).sqrt()) as u32;
            let std = if std == 0 { 1 } else { std };

            maxpixel[pix_idx] = max_val.min(255) as u8;
            maxframe[pix_idx] = max_frame_idx.min(255) as u8;
            avepixel[pix_idx] = mean.min(255) as u8;
            stdpixel[pix_idx] = std.min(255) as u8;
        }
    }

    FfData {
        nrows: height,
        ncols: width,
        nframes,
        fps,
        station_id: station_id.to_string(),
        deinterlace_order,
        start_time,
        maxpixel,
        maxframe,
        avepixel,
        stdpixel,
        field_sums,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_compress_constant_frames() {
        // All pixels are 100 → max=100, mean≈100, std≈1 (clamped)
        let w = 4u32;
        let h = 4u32;
        let n = 256u32;
        let frames = vec![100u8; (n * h * w) as usize];

        let ff = compress_frames(&frames, w, h, n, 25.0, "XX0001", -1, Utc::now());

        assert_eq!(ff.maxpixel.len(), (w * h) as usize);
        assert!(ff.maxpixel.iter().all(|&v| v == 100));
        // Mean should be close to 100 (top-4 exclusion of identical values
        // means we still average 252 values of 100)
        assert!(ff.avepixel.iter().all(|&v| v == 100));
        // Stddev should be 1 (clamped from 0)
        assert!(ff.stdpixel.iter().all(|&v| v == 1));
    }

    #[test]
    fn test_compress_ramp() {
        // Pixel values ramp from 0..255 across 256 frames
        let w = 1u32;
        let h = 1u32;
        let n = 256u32;
        let frames: Vec<u8> = (0..256).map(|i| i as u8).collect();

        let ff = compress_frames(&frames, w, h, n, 25.0, "XX0001", -1, Utc::now());

        assert_eq!(ff.maxpixel[0], 255);
        // Mean without top-4 (255,254,253,252) ≈ sum(0..252)/252
        let sum: u32 = (0u32..252).sum();
        let expected_mean = sum / 252;
        assert!((ff.avepixel[0] as i32 - expected_mean as i32).unsigned_abs() <= 1);
    }
}
