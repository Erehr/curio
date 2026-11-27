//! The fingerprints the duplicate finder compares, one per item and kind in the `fingerprints` table: how a thumbnail looks, frames through a video, the first minute of a sound, an archive's entries.
//! Only a full-content hash establishes that two files are the same; these find candidates for a person to look at.

use std::f64::consts::PI;
use std::path::Path;
use std::sync::LazyLock;

use image::{DynamicImage, GenericImageView, Rgb, RgbImage};

use crate::core::images::color;
use crate::core::consts::{DUPLICATE_ASPECT_TOLERANCE, DUPLICATE_COLOUR_GAP, DUPLICATE_GRADIENT_SLACK, DUPLICATE_SOUND_SECONDS, DUPLICATE_VIDEO_FRAMES, PHASH_IMAGE_SIZE, PHASH_LOW_FREQUENCIES};

pub const LOOK: i64 = 1;
pub const VIDEO: i64 = 2;
pub const SOUND: i64 = 3;
pub const ARCHIVE: i64 = 4;

/// The version of each kind's algorithm; a print of another version is made again.
pub fn version(kind: i64) -> i64 {
    match kind {
        LOOK | VIDEO | SOUND | ARCHIVE => 1,
        _ => 0,
    }
}

// ------------------------------------------------------------------ the DCT hash

const DCT_SIDE: usize = PHASH_IMAGE_SIZE;
const DCT_LOW: usize = PHASH_LOW_FREQUENCIES;

/// cos(π/32 · (x + ½) · u) for the eight lowest frequencies.
static DCT_COS: LazyLock<[[f64; DCT_SIDE]; DCT_LOW]> = LazyLock::new(|| {
    let mut t = [[0.0; DCT_SIDE]; DCT_LOW];
    for (u, row) in t.iter_mut().enumerate() {
        for (x, c) in row.iter_mut().enumerate() {
            *c = (PI / DCT_SIDE as f64 * (x as f64 + 0.5) * u as f64).cos();
        }
    }
    t
});

/// The classic DCT hash of 32 × 32 greys: the lowest 8 × 8 frequencies, one bit each for being above their median (the first, the overall brightness, left out of the median).
pub fn dct_hash(grey: &[u8]) -> u64 {
    debug_assert_eq!(grey.len(), DCT_SIDE * DCT_SIDE);
    let cos = &*DCT_COS;
    let mut rows = [[0.0f64; DCT_LOW]; DCT_SIDE];
    for (y, row) in rows.iter_mut().enumerate() {
        let line = &grey[y * DCT_SIDE..(y + 1) * DCT_SIDE];
        for (u, out) in row.iter_mut().enumerate() {
            *out = line.iter().zip(cos[u].iter()).map(|(p, c)| *p as f64 * c).sum();
        }
    }
    let mut coefficients = [0.0f64; DCT_LOW * DCT_LOW];
    for v in 0..DCT_LOW {
        for u in 0..DCT_LOW {
            coefficients[v * DCT_LOW + u] = (0..DCT_SIDE).map(|y| rows[y][u] * cos[v][y]).sum();
        }
    }
    let mut sorted = coefficients[1..].to_vec();
    sorted.sort_by(f64::total_cmp);
    let median = sorted[sorted.len() / 2];
    coefficients.iter().enumerate().filter(|(_, c)| **c > median).fold(0u64, |bits, (i, _)| bits | 1 << i)
}

pub fn bits_apart(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

// ------------------------------------------------------------------ looks

/// How a thumbnail looks: the DCT hash of its 32 × 32 greys, a gradient hash of its 9 × 8 greys, and the mean colour of each quarter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Look {
    pub dct: u64,
    pub grad: u64,
    pub quarters: [[u8; 3]; 4],
}

fn luma(p: &Rgb<u8>) -> u8 {
    (0.2126 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.0722 * p[2] as f64).round() as u8
}

/// Transparent pixels as white, as a copy saved without transparency usually has them.
fn flatten(img: &DynamicImage) -> RgbImage {
    if !img.color().has_alpha() {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    let mut out = RgbImage::new(rgba.width(), rgba.height());
    for (o, p) in out.pixels_mut().zip(rgba.pixels()) {
        let a = p[3] as u32;
        let mix = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
        *o = Rgb([mix(p[0]), mix(p[1]), mix(p[2])]);
    }
    out
}

impl Look {
    pub fn of(img: &DynamicImage) -> Look {
        let (w, h) = img.dimensions();
        if w == 0 || h == 0 {
            return Look { dct: 0, grad: 0, quarters: [[0; 3]; 4] };
        }
        let flat = flatten(img);
        let small = image::imageops::resize(&flat, DCT_SIDE as u32, DCT_SIDE as u32, image::imageops::FilterType::Triangle);
        let grey: Vec<u8> = small.pixels().map(luma).collect();
        let tiny = image::imageops::resize(&flat, 9, 8, image::imageops::FilterType::Triangle);
        let mut grad = 0u64;
        for y in 0..8 {
            for x in 0..8 {
                if luma(tiny.get_pixel(x, y)) > luma(tiny.get_pixel(x + 1, y)) {
                    grad |= 1 << (y * 8 + x);
                }
            }
        }
        let half = DCT_SIDE as u32 / 2;
        let mut quarters = [[0u8; 3]; 4];
        for (q, out) in quarters.iter_mut().enumerate() {
            let (x0, y0) = ((q as u32 % 2) * half, (q as u32 / 2) * half);
            let mut sum = [0u32; 3];
            for y in y0..y0 + half {
                for x in x0..x0 + half {
                    let p = small.get_pixel(x, y);
                    (0..3).for_each(|c| sum[c] += p[c] as u32);
                }
            }
            let n = half * half;
            *out = [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8];
        }
        Look { dct: dct_hash(&grey), grad, quarters }
    }

    /// What the `fingerprints` row keeps: the DCT hash as its key, the rest as its data.
    pub fn key(&self) -> i64 {
        self.dct as i64
    }

    pub fn data(&self) -> Vec<u8> {
        let mut d = self.grad.to_le_bytes().to_vec();
        d.extend(self.quarters.iter().flatten());
        d
    }

    pub fn from_row(key: i64, data: &[u8]) -> Option<Look> {
        if data.len() < 20 {
            return None;
        }
        let mut quarters = [[0u8; 3]; 4];
        for (q, out) in quarters.iter_mut().enumerate() {
            out.copy_from_slice(&data[8 + q * 3..11 + q * 3]);
        }
        Some(Look { dct: key as u64, grad: u64::from_le_bytes(data[..8].try_into().ok()?), quarters })
    }

    /// Mean CIE76 distance of the quarters' colours.
    pub fn colour_gap(&self, o: &Look) -> f64 {
        self.quarters.iter().zip(o.quarters.iter()).map(|(a, b)| color::de76(color::lab(*a), color::lab(*b))).sum::<f64>() / 4.0
    }

    /// The share of both hashes' bits two looks have in common.
    pub fn similarity(&self, o: &Look) -> f64 {
        1.0 - (bits_apart(self.dct, o.dct) + bits_apart(self.grad, o.grad)) as f64 / 128.0
    }
}

/// How many DCT bits may differ for a similarity from 0 to 1.
pub fn max_bits(similarity: f64) -> u32 {
    ((1.0 - similarity.clamp(0.0, 1.0)) * 64.0).floor() as u32
}

/// Looks that pass as the same picture: the DCT hashes within `max_bits`, the gradient hashes within a few bits more, the quarters' colours near and the shapes alike.
pub fn looks_match(a: &Look, b: &Look, aspects: (f32, f32), max_bits: u32) -> bool {
    let (ra, rb) = (aspects.0.max(0.01) as f64, aspects.1.max(0.01) as f64);
    bits_apart(a.dct, b.dct) <= max_bits && bits_apart(a.grad, b.grad) <= max_bits + DUPLICATE_GRADIENT_SLACK && (ra / rb).ln().abs() <= DUPLICATE_ASPECT_TOLERANCE && a.colour_gap(b) <= DUPLICATE_COLOUR_GAP
}

/// A thumbnail file's look, for prints older than their thumbnail.
pub fn look_of_file(path: &Path) -> Result<Look, String> {
    let img = image::ImageReader::open(path).map_err(|e| e.to_string())?.with_guessed_format().map_err(|e| e.to_string())?.decode().map_err(|e| e.to_string())?;
    Ok(Look::of(&img))
}

// ------------------------------------------------------------------ videos

pub const VIDEO_FRAMES: usize = DUPLICATE_VIDEO_FRAMES;
/// A frame's greys spread less than this (standard deviation) are a flat frame: black, white or a fade, whose DCT hash says nothing.
const FLAT_SPREAD: f64 = 4.0;
const FLAT_BRIGHTNESS_GAP: i32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    pub dct: u64,
    pub mean: u8,
    pub flat: bool,
}

pub fn frame_of(grey: &[u8]) -> Frame {
    let n = grey.len().max(1) as f64;
    let mean = grey.iter().map(|p| *p as f64).sum::<f64>() / n;
    let spread = (grey.iter().map(|p| (*p as f64 - mean).powi(2)).sum::<f64>() / n).sqrt();
    Frame { dct: if grey.len() == DCT_SIDE * DCT_SIDE { dct_hash(grey) } else { 0 }, mean: mean.round() as u8, flat: spread < FLAT_SPREAD }
}

/// Frames at even steps through a clip `duration` seconds long, read with FFmpeg.
pub fn video_frames(src: &Path, duration: f64) -> Result<Vec<Frame>, String> {
    if !(duration > 0.0) {
        return Err("The video's length isn't known yet".into());
    }
    (0..VIDEO_FRAMES).map(|i| crate::core::media::ffmpeg::gray_frame(src, duration * (i as f64 + 0.5) / VIDEO_FRAMES as f64, DCT_SIDE as u32).map(|g| frame_of(&g))).collect()
}

pub fn frames_data(frames: &[Frame]) -> Vec<u8> {
    let mut d = Vec::with_capacity(frames.len() * 10);
    for f in frames {
        d.extend(f.dct.to_le_bytes());
        d.push(f.mean);
        d.push(f.flat as u8);
    }
    d
}

pub fn frames_from(data: &[u8]) -> Vec<Frame> {
    data.chunks_exact(10).map(|c| Frame { dct: u64::from_le_bytes(c[..8].try_into().unwrap_or_default()), mean: c[8], flat: c[9] != 0 }).collect()
}

/// How alike two clips' frames are, when all but at most one pair of frames are within `max_bits` (flat frames by their brightness): the mean of the frames' similarity.
pub fn video_similarity(a: &[Frame], b: &[Frame], max_bits: u32) -> Option<f64> {
    let n = a.len().min(b.len());
    if n < 3 {
        return None;
    }
    let mut matched = 0;
    let mut sum = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        let sim = match (x.flat, y.flat) {
            (true, true) => {
                let gap = (x.mean as i32 - y.mean as i32).abs();
                matched += (gap <= FLAT_BRIGHTNESS_GAP) as usize;
                1.0 - gap as f64 / 255.0
            }
            (false, false) => {
                let bits = bits_apart(x.dct, y.dct);
                matched += (bits <= max_bits) as usize;
                1.0 - bits as f64 / 64.0
            }
            _ => 0.0,
        };
        sum += sim;
    }
    (matched + 1 >= n).then(|| sum / n as f64)
}

/// Two clips' lengths close enough to be one clip: within a second, or 2 % of the longer.
pub fn video_lengths_match(a: f64, b: f64) -> bool {
    (a - b).abs() <= (a.max(b) * 0.02).max(1.0)
}

// ------------------------------------------------------------------ sounds

pub const SOUND_RATE: u32 = 11025;
pub const SOUND_SECONDS: u32 = DUPLICATE_SOUND_SECONDS;
const SOUND_FRAME: usize = 4096;
const SOUND_HOP: usize = 1024;
const SOUND_BANDS: usize = 33;
const SOUND_LOW_HZ: f64 = 300.0;
const SOUND_HIGH_HZ: f64 = 2000.0;
/// How many frames one recording may start before the other: about three quarters of a second either way.
const SOUND_SHIFT: i64 = 8;
/// Frames two prints must overlap by to be compared, about 4.5 seconds.
const SOUND_MIN_OVERLAP: usize = 48;

/// One bit per pair of neighbouring bands and frame (Haitsma and Kalker): whether the energy difference between the two bands grew since the frame before. A change of loudness or of format leaves the signs as they are.
pub fn sound_print(pcm: &[i16]) -> Vec<u32> {
    let peak = pcm.iter().map(|s| (*s as i32).unsigned_abs()).max().unwrap_or(0);
    // what comes before the sound starts differs between copies (encoders' delays, trimmed silence)
    let quiet = (peak / 50).max(64);
    let Some(start) = pcm.iter().position(|s| (*s as i32).unsigned_abs() > quiet) else { return vec![] };
    let pcm = &pcm[start..];
    if pcm.len() < SOUND_FRAME * 2 {
        return vec![];
    }
    let window: Vec<f64> = (0..SOUND_FRAME).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f64 / SOUND_FRAME as f64).cos()).collect();
    let edges: Vec<usize> = (0..=SOUND_BANDS).map(|m| (SOUND_LOW_HZ * (SOUND_HIGH_HZ / SOUND_LOW_HZ).powf(m as f64 / SOUND_BANDS as f64) * SOUND_FRAME as f64 / SOUND_RATE as f64).round() as usize).collect();
    let fft = Fft::new(SOUND_FRAME);
    let (mut re, mut im) = (vec![0.0; SOUND_FRAME], vec![0.0; SOUND_FRAME]);
    let mut prev: Option<[f64; SOUND_BANDS]> = None;
    let mut out = Vec::new();
    let mut at = 0;
    while at + SOUND_FRAME <= pcm.len() {
        for i in 0..SOUND_FRAME {
            re[i] = pcm[at + i] as f64 * window[i];
            im[i] = 0.0;
        }
        fft.run(&mut re, &mut im);
        let mut energy = [0.0f64; SOUND_BANDS];
        for (m, e) in energy.iter_mut().enumerate() {
            *e = (edges[m]..edges[m + 1].max(edges[m] + 1)).map(|k| re[k] * re[k] + im[k] * im[k]).sum();
        }
        if let Some(p) = prev {
            let mut bits = 0u32;
            for m in 0..SOUND_BANDS - 1 {
                if (energy[m] - energy[m + 1]) - (p[m] - p[m + 1]) > 0.0 {
                    bits |= 1 << m;
                }
            }
            out.push(bits);
        }
        prev = Some(energy);
        at += SOUND_HOP;
    }
    out
}

pub fn sound_data(print: &[u32]) -> Vec<u8> {
    print.iter().flat_map(|b| b.to_le_bytes()).collect()
}

pub fn sound_from(data: &[u8]) -> Vec<u32> {
    data.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

/// How alike two recordings sound: 1 − 2 × the share of bits that differ, at the best of the shifts tried, so unrelated sounds come out near 0.
pub fn sound_similarity(a: &[u32], b: &[u32]) -> Option<f64> {
    let mut best: Option<f64> = None;
    for shift in -SOUND_SHIFT..=SOUND_SHIFT {
        let (x, y) = if shift >= 0 { (&a[shift as usize..], b) } else { (a, &b[(-shift) as usize..]) };
        let n = x.len().min(y.len());
        if n < SOUND_MIN_OVERLAP {
            continue;
        }
        let differ: u32 = x[..n].iter().zip(&y[..n]).map(|(p, q)| (p ^ q).count_ones()).sum();
        let sim = (1.0 - 2.0 * differ as f64 / (n as f64 * (SOUND_BANDS - 1) as f64)).max(0.0);
        if best.is_none_or(|b| sim > b) {
            best = Some(sim);
        }
    }
    best
}

/// Two recordings' lengths close enough to be one recording: within three seconds, or 3 % of the longer.
pub fn sound_lengths_match(a: f64, b: f64) -> bool {
    (a - b).abs() <= (a.max(b) * 0.03).max(3.0)
}

/// A sound file's print, read with FFmpeg.
pub fn sound_of_file(src: &Path) -> Result<Vec<u32>, String> {
    let print = sound_print(&crate::core::media::ffmpeg::pcm(src, SOUND_RATE, SOUND_SECONDS)?);
    if print.is_empty() {
        Err("The sound is silent or too short to compare".into())
    } else {
        Ok(print)
    }
}

/// A radix-2 FFT of one size, its twiddles worked out once.
struct Fft {
    n: usize,
    cos: Vec<f64>,
    sin: Vec<f64>,
}

impl Fft {
    fn new(n: usize) -> Fft {
        debug_assert!(n.is_power_of_two());
        Fft { n, cos: (0..n / 2).map(|k| (2.0 * PI * k as f64 / n as f64).cos()).collect(), sin: (0..n / 2).map(|k| -(2.0 * PI * k as f64 / n as f64).sin()).collect() }
    }

    fn run(&self, re: &mut [f64], im: &mut [f64]) {
        let n = self.n;
        let mut j = 0;
        for i in 1..n {
            let mut bit = n >> 1;
            while j & bit != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j |= bit;
            if i < j {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut len = 2;
        while len <= n {
            let step = n / len;
            for start in (0..n).step_by(len) {
                for k in 0..len / 2 {
                    let (wr, wi) = (self.cos[k * step], self.sin[k * step]);
                    let (a, b) = (start + k, start + k + len / 2);
                    let (xr, xi) = (re[b] * wr - im[b] * wi, re[b] * wi + im[b] * wr);
                    re[b] = re[a] - xr;
                    im[b] = im[a] - xi;
                    re[a] += xr;
                    im[a] += xi;
                }
            }
            len <<= 1;
        }
    }
}

// ------------------------------------------------------------------ archives

/// More entries than this and the rest are left out of the print.
const ARCHIVE_MAX_ENTRIES: usize = 10_000;

/// The entries of a ZIP, CBZ, EPUB, 7z or CB7 as (CRC-32, size), sorted, read from its directory alone; folders and what systems leave beside the content are left out.
pub fn archive_entries(path: &Path) -> Result<Vec<(u32, u64)>, String> {
    let mut out: Vec<(u32, u64)> = Vec::new();
    if crate::core::archive::is_7z(path) {
        let r = sevenz_rust::SevenZReader::open(path, sevenz_rust::Password::empty()).map_err(|e| format!("Not an archive Curio reads: {e}"))?;
        for f in r.archive().files.iter().filter(|f| !f.is_directory && f.has_stream && f.has_crc && !crate::core::archive::is_noise(&f.name)) {
            out.push((f.crc as u32, f.size));
        }
    } else {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut z = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(|e| format!("Not an archive Curio reads: {e}"))?;
        for i in 0..z.len() {
            let Ok(f) = z.by_index_raw(i) else { continue };
            if f.is_dir() || f.size() == 0 || crate::core::archive::is_noise(f.name()) {
                continue;
            }
            out.push((f.crc32(), f.size()));
        }
    }
    if out.is_empty() {
        return Err("The archive holds no files".into());
    }
    out.sort_unstable();
    out.truncate(ARCHIVE_MAX_ENTRIES);
    Ok(out)
}

pub fn entries_data(entries: &[(u32, u64)]) -> Vec<u8> {
    let mut d = Vec::with_capacity(entries.len() * 12);
    for (crc, size) in entries {
        d.extend(crc.to_le_bytes());
        d.extend(size.to_le_bytes());
    }
    d
}

pub fn entries_from(data: &[u8]) -> Vec<(u32, u64)> {
    data.chunks_exact(12).map(|c| (u32::from_le_bytes([c[0], c[1], c[2], c[3]]), u64::from_le_bytes(c[4..12].try_into().unwrap_or_default()))).collect()
}

/// Equal for archives holding the same entries, whatever their names, order or packing.
pub fn entries_key(entries: &[(u32, u64)]) -> i64 {
    let h = blake3::hash(&entries_data(entries));
    i64::from_le_bytes(h.as_bytes()[..8].try_into().unwrap_or_default())
}

/// The share of entries two archives have in common, |A ∩ B| / |A ∪ B|; both lists sorted.
pub fn archive_similarity(a: &[(u32, u64)], b: &[(u32, u64)]) -> f64 {
    let (mut i, mut j, mut common) = (0, 0, 0usize);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                common += 1;
                i += 1;
                j += 1;
            }
        }
    }
    let union = a.len() + b.len() - common;
    if union == 0 {
        0.0
    } else {
        common as f64 / union as f64
    }
}
