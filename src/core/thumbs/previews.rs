//! The viewer's previews: decoded at the size asked, kept in memory, a few at a time.

use std::path::Path;
use std::sync::LazyLock;
use image::{DynamicImage, GenericImageView, ImageDecoder, ImageEncoder};
use crate::core::model::AssetKind;
use crate::core::thumbs::decode::source_icc;
use crate::core::thumbs::decode::decode_source;
use crate::core::thumbs::ExternalHandler;

/// The viewer's previews made lately, by item, version and size, so zooming back or stepping to the item before doesn't decode its file again. Memory only: nothing of a preview is written to disk.
const PREVIEW_MEMORY_BYTES: usize = 192 << 20;

#[derive(Default)]
struct PreviewMemory {
    held: std::collections::VecDeque<(String, std::sync::Arc<Vec<u8>>)>,
    bytes: usize,
}

static PREVIEW_MEMORY: LazyLock<parking_lot::Mutex<PreviewMemory>> = LazyLock::new(Default::default);

fn remembered(key: &str) -> Option<std::sync::Arc<Vec<u8>>> {
    let mut m = PREVIEW_MEMORY.lock();
    let i = m.held.iter().position(|(k, _)| k == key)?;
    let hit = m.held.remove(i)?;
    let bytes = hit.1.clone();
    m.held.push_back(hit);
    Some(bytes)
}

fn remember(key: String, bytes: std::sync::Arc<Vec<u8>>) {
    let mut m = PREVIEW_MEMORY.lock();
    // two requests that missed at once both make it; it is held once
    if let Some(i) = m.held.iter().position(|(k, _)| *k == key) {
        if let Some((_, old)) = m.held.remove(i) {
            m.bytes -= old.len();
        }
    }
    m.bytes += bytes.len();
    m.held.push_back((key, bytes));
    while m.bytes > PREVIEW_MEMORY_BYTES && m.held.len() > 1 {
        if let Some((_, old)) = m.held.pop_front() {
            m.bytes -= old.len();
        }
    }
}

/// A picture scaled to `max` on its long edge for the viewer, made when asked for: JPEG, or PNG when see-through. `key` names the item's version.
pub fn preview(key: &str, src: &Path, ext: &str, kind: AssetKind, max: u32, handlers: &[ExternalHandler], tmp_dir: &Path) -> Result<std::sync::Arc<Vec<u8>>, String> {
    let key = format!("{key}_{max}");
    if let Some(b) = remembered(&key) {
        return Ok(b);
    }
    let _slot = preview_slot();
    // the same preview may have been finished while waiting for a slot
    if let Some(b) = remembered(&key) {
        return Ok(b);
    }
    let by_plugin = handlers.iter().any(|h| h.exts.iter().any(|e| e == ext));
    let scaled = if !by_plugin && crate::core::formats::image::scalable_decode(ext) { decode_jpeg_scaled(src, max).ok() } else { None };
    let img = match scaled {
        Some(i) => i,
        None => decode_source(src, ext, kind, max, handlers, tmp_dir)?,
    };
    let icc = if by_plugin { None } else { source_icc(src) };
    let bytes = std::sync::Arc::new(encode_preview(fit_fast(img, max), PREVIEW_JPEG_QUALITY, icc)?);
    remember(key, bytes.clone());
    Ok(bytes)
}

/// A picture held in memory (an archive's page) at `max` on its long edge, as JPEG or, with transparency, PNG. `key` names the page of the item's version; `load` runs only when the page isn't among the previews made lately.
pub fn bytes_preview(key: &str, load: impl FnOnce() -> Result<Vec<u8>, String>, max: u32) -> Result<std::sync::Arc<Vec<u8>>, String> {
    let key = format!("{key}_{max}");
    if let Some(b) = remembered(&key) {
        return Ok(b);
    }
    let bytes = load()?;
    let _slot = preview_slot();
    let img = match bytes.starts_with(&[0xFF, 0xD8]).then(|| decode_jpeg_scaled_bytes(&bytes, max).ok()).flatten() {
        Some(i) => i,
        None => image::load_from_memory(&bytes).map_err(|e| format!("Not a picture Curio reads: {e}"))?,
    };
    let img = fit_fast(img, max);
    let mut out = std::io::Cursor::new(Vec::new());
    if img.color().has_alpha() {
        img.write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    } else {
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, PREVIEW_JPEG_QUALITY).encode_image(&img.to_rgb8()).map_err(|e| e.to_string())?;
    }
    let out = std::sync::Arc::new(out.into_inner());
    remember(key, out.clone());
    Ok(out)
}

/// How many previews may be decoded at once, so stepping quickly through very large images can't
/// use up the machine.
const PREVIEW_SLOTS: u32 = 3;

const PREVIEW_JPEG_QUALITY: u8 = 90;

static PREVIEW_GATE: (std::sync::Mutex<u32>, std::sync::Condvar) = (std::sync::Mutex::new(0), std::sync::Condvar::new());

struct PreviewSlot;

fn preview_slot() -> PreviewSlot {
    let (m, cv) = &PREVIEW_GATE;
    let mut n = m.lock().unwrap_or_else(|p| p.into_inner());
    while *n >= PREVIEW_SLOTS {
        n = cv.wait(n).unwrap_or_else(|p| p.into_inner());
    }
    *n += 1;
    PreviewSlot
}

impl Drop for PreviewSlot {
    fn drop(&mut self) {
        let (m, cv) = &PREVIEW_GATE;
        *m.lock().unwrap_or_else(|p| p.into_inner()) -= 1;
        cv.notify_one();
    }
}

/// A JPEG read at the smallest of 1/1, 1/2, 1/4 or 1/8 size that still covers `max`: a 7000 x 5000
/// photo meant for a 1400 px window costs a quarter of the full decode or less.
pub fn decode_jpeg_scaled(src: &Path, max: u32) -> Result<DynamicImage, String> {
    let open = || std::fs::File::open(src).map(std::io::BufReader::new).map_err(|e| e.to_string());
    jpeg_scaled(open()?, open()?, max)
}

/// The same for a JPEG held in memory, such as an archive's page.
pub fn decode_jpeg_scaled_bytes(bytes: &[u8], max: u32) -> Result<DynamicImage, String> {
    jpeg_scaled(std::io::Cursor::new(bytes), std::io::Cursor::new(bytes), max)
}

/// `pixels` and `header` read the same JPEG: the first is decoded, the second gives its stored turn.
fn jpeg_scaled<R: std::io::BufRead + std::io::Seek>(pixels: R, header: R, max: u32) -> Result<DynamicImage, String> {
    use jpeg_decoder::{Decoder, PixelFormat};
    let mut d = Decoder::new(pixels);
    d.read_info().map_err(|e| e.to_string())?;
    let info = d.info().ok_or("no JPEG header")?;
    let (w, h) = (info.width as u32, info.height as u32);
    let long = w.max(h).max(1);
    let (tw, th) = if long > max { ((w * max / long).max(1), (h * max / long).max(1)) } else { (w, h) };
    d.scale(tw as u16, th as u16).map_err(|e| e.to_string())?;
    let px = d.decode().map_err(|e| e.to_string())?;
    let info = d.info().ok_or("no JPEG header")?;
    let (w, h) = (info.width as u32, info.height as u32);
    let mut img = match info.pixel_format {
        PixelFormat::RGB24 => image::RgbImage::from_raw(w, h, px).map(DynamicImage::ImageRgb8),
        PixelFormat::L8 => image::GrayImage::from_raw(w, h, px).map(DynamicImage::ImageLuma8),
        // CMYK and 16-bit files take the ordinary route
        _ => None,
    }
    .ok_or("unsupported JPEG pixel format")?;
    // the turn stored in the file still applies
    let turn = image::ImageReader::new(header)
        .with_guessed_format()
        .ok()
        .and_then(|r| r.into_decoder().ok())
        .and_then(|mut d| d.orientation().ok())
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    img.apply_orientation(turn);
    Ok(img)
}

/// Shrink to `max` on the long edge with SIMD resampling; transparent pixels are handled properly.
pub fn fit_fast(img: DynamicImage, max: u32) -> DynamicImage {
    use fast_image_resize as fr;
    let (w, h) = img.dimensions();
    if w <= max && h <= max {
        return img;
    }
    let s = max as f64 / w.max(h) as f64;
    let (tw, th) = (((w as f64 * s).round() as u32).max(1), ((h as f64 * s).round() as u32).max(1));
    let alpha = img.color().has_alpha();
    let (buf, pt) = if alpha { (img.to_rgba8().into_raw(), fr::PixelType::U8x4) } else { (img.to_rgb8().into_raw(), fr::PixelType::U8x3) };
    let Ok(from) = fr::images::Image::from_vec_u8(w, h, buf, pt) else { return img };
    let mut to = fr::images::Image::new(tw, th, pt);
    let opts = fr::ResizeOptions::new().resize_alg(fr::ResizeAlg::Convolution(fr::FilterType::CatmullRom));
    if fr::Resizer::new().resize(&from, &mut to, &opts).is_err() {
        return img.resize(max, max, image::imageops::FilterType::Triangle);
    }
    let raw = to.into_vec();
    let out = if alpha { image::RgbaImage::from_raw(tw, th, raw).map(DynamicImage::ImageRgba8) } else { image::RgbImage::from_raw(tw, th, raw).map(DynamicImage::ImageRgb8) };
    out.unwrap_or(img)
}

/// JPEG unless something is actually see-through.
fn encode_preview(img: DynamicImage, quality: u8, icc: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let (w, h) = img.dimensions();
    let see_through = img.color().has_alpha() && img.to_rgba8().pixels().any(|p| p[3] < 250);
    if see_through {
        let mut enc = image::codecs::png::PngEncoder::new_with_quality(&mut out, image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::Adaptive);
        if let Some(p) = icc {
            let _ = enc.set_icc_profile(p);
        }
        enc.write_image(img.to_rgba8().as_raw(), w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())?;
    } else {
        let mut enc = jpeg_encoder::Encoder::new(&mut out, quality);
        if let Some(p) = icc {
            let _ = enc.add_icc_profile(&p);
        }
        enc.encode(img.to_rgb8().as_raw(), w as u16, h as u16, jpeg_encoder::ColorType::Rgb).map_err(|e| e.to_string())?;
    }
    Ok(out)
}
