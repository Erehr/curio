//! Thumbnail and preview generation. Thumbnails are disposable cache files kept apart from
//! originals, partitioned into sub-directories by the random tail of the asset id.
//!
//! Order of attempts (thumb.js: plugin handler first, then built-in):
//!   1. plugin format handlers
//!   2. OS thumbnailer for the formats whose own programs draw them best (shellthumb), then the preview a RAW or design file carries (embedded)
//!   3. links (.url): YouTube poster (i.ytimg.com) or the page's preview image
//!   4. videos and sound: FFmpeg
//!   5. built-in decoders (png, jpg, gif, webp, bmp, tiff, ico, tga, qoi, dds, hdr, svg, cbz/zip)
//!   6. what FFmpeg decodes (AVIF, HEIC, PSD, EXR …), and PDFs via pdftoppm
//!   7. OS thumbnailer as last resort (480 px), then the small preview inside EPS and AI files

pub mod decode;
pub mod embedded;
pub mod links;
pub mod previews;
pub mod shell;

use std::path::{Path, PathBuf};

use image::{DynamicImage, GenericImageView, ImageEncoder};

use crate::core::images::color;
use crate::core::consts::*;
use crate::core::model::{AssetKind, PaletteColor};
use crate::core::thumbs::decode::source_icc;
use crate::core::thumbs::decode::decode_full;

/// A format handler provided by a plugin: an external command that writes an image.
#[derive(Clone, Debug)]
pub struct ExternalHandler {
    pub plugin_id: String,
    pub exts: Vec<String>,
    /// argv template with {input}, {output}, {size}
    pub command: Vec<String>,
    /// the plugin's folder, where the command runs
    pub dir: Option<PathBuf>,
    pub report: HandlerReport,
}

/// Told whether each run of a handler worked (`None`) or why it failed, so a failing plugin is switched off like its other programs.
#[derive(Clone, Default)]
pub struct HandlerReport(pub Option<std::sync::Arc<dyn Fn(Option<&str>) + Send + Sync>>);

impl std::fmt::Debug for HandlerReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HandlerReport")
    }
}

#[derive(Default)]
pub struct ThumbResult {
    pub dims: Option<(i64, i64)>,
    /// aspect ratio of the generated thumbnail, for non-visual items (books, comics, PDFs, links) that carry no pixel dimensions of their own
    pub aspect: Option<f64>,
    pub palette: Vec<PaletteColor>,
    pub look: Option<crate::core::duplicates::fingerprints::Look>,
    pub has_thumb: bool,
    pub duration: Option<f64>,
    /// links: the page title, when the asset has no display name yet
    pub title: Option<String>,
}

/// The name an item's cached files go by. Uids are short ASCII codes, but they come from the library file, so anything else (a path, a separator) is replaced by its hash.
pub fn cache_key(uid: &str) -> std::borrow::Cow<'_, str> {
    if crate::core::model::uid_bytes(uid).is_some() {
        return uid.replace('-', "").to_ascii_lowercase().into();
    }
    if (2..=64).contains(&uid.len()) && uid.bytes().all(|b| b.is_ascii_alphanumeric()) {
        uid.into()
    } else {
        blake3::hash(uid.as_bytes()).to_hex()[..26].to_string().into()
    }
}

pub fn thumb_path(cache_dir: &Path, uid: &str, ver: i64) -> PathBuf {
    let key = cache_key(uid);
    cache_dir.join(&key[key.len() - 2..]).join(format!("{key}_{ver}.img"))
}

/// Removes the asset's thumbnails from before `ver`: its version can go up more than once before the next thumbnail is written (a rescan, then a plugin turned on), so not only the one before it.
pub fn remove_older(cache_dir: &Path, uid: &str, ver: i64) {
    // a first thumbnail has none before it
    if ver == 0 {
        return;
    }
    let key = cache_key(uid);
    let Ok(entries) = std::fs::read_dir(cache_dir.join(&key[key.len() - 2..])) else { return };
    for f in entries.flatten() {
        if f.file_name().to_str().and_then(|name| version_of(name, &key)).is_some_and(|n| n < ver) {
            let _ = std::fs::remove_file(f.path());
        }
    }
}

/// Thumbnails left behind by versions written before `remove_older` took every older one: each `<key>_<n>.img` whose asset is at a later version than `n`. `current` maps each asset's cache key to its version.
pub fn remove_old_versions(cache_dir: &Path, current: &std::collections::HashMap<String, i64>) -> usize {
    let mut n = 0;
    for dir in std::fs::read_dir(cache_dir).into_iter().flatten().flatten().filter(|d| d.file_name().len() == 2) {
        for f in std::fs::read_dir(dir.path()).into_iter().flatten().flatten() {
            let Some(name) = f.file_name().to_str().map(str::to_string) else { continue };
            let Some((key, _)) = name.split_once('_') else { continue };
            let old = current.get(key).is_some_and(|ver| version_of(&name, key).is_some_and(|v| v < *ver));
            if old && std::fs::remove_file(f.path()).is_ok() {
                n += 1;
            }
        }
    }
    n
}

/// The version in a thumbnail's name, `<key>_<version>.img`, when it is the thumbnail of `key`.
fn version_of(name: &str, key: &str) -> Option<i64> {
    let digits = name.strip_prefix(key)?.strip_prefix('_')?.strip_suffix(".img")?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Scaled previews of items and of archives' pages were once kept on disk as `<key>_*.img` in two-character folders; only files matching that pattern are removed.
pub fn remove_stored_previews(preview_dir: &Path) -> usize {
    let mut n = 0;
    for dir in std::fs::read_dir(preview_dir).into_iter().flatten().flatten().filter(|d| d.file_name().len() == 2 && d.file_name().to_str().is_some_and(|s| s.bytes().all(|b| b.is_ascii_alphanumeric()))) {
        for f in std::fs::read_dir(dir.path()).into_iter().flatten().flatten() {
            if f.path().is_file() && f.file_name().to_str().is_some_and(is_stored_preview_name) && std::fs::remove_file(f.path()).is_ok() {
                n += 1;
            }
        }
    }
    n
}

fn is_stored_preview_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".img") else { return false };
    let Some((key, rest)) = stem.split_once('_') else { return false };
    (2..=64).contains(&key.len()) && !rest.is_empty() && key.bytes().all(|b| b.is_ascii_alphanumeric()) && rest.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

// ------------------------------------------------------------------ decoding

fn write_image(rgba: &image::RgbaImage, dest: &Path, quality: u8, icc: Option<Vec<u8>>) -> Result<(), String> {
    write_bytes(&encode_image(rgba, quality, icc)?, dest)
}

fn encode_image(rgba: &image::RgbaImage, quality: u8, icc: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    if rgba.pixels().any(|p| p[3] < 250) {
        let mut enc = image::codecs::png::PngEncoder::new(&mut out);
        if let Some(p) = icc {
            let _ = enc.set_icc_profile(p);
        }
        enc.write_image(rgba.as_raw(), rgba.width(), rgba.height(), image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())?;
    } else {
        let rgb = DynamicImage::ImageRgba8(rgba.clone()).to_rgb8();
        // colour at half resolution: a quarter smaller than at full, at 512 px seen only side by side
        // the standard Huffman tables: fitted ones made a flat picture that the `image` crate reads back without its colour
        let mut enc = jpeg_encoder::Encoder::new(&mut out, quality);
        enc.set_sampling_factor(jpeg_encoder::SamplingFactor::R_4_2_0);
        if let Some(p) = icc {
            let _ = enc.add_icc_profile(&p);
        }
        enc.encode(rgb.as_raw(), rgb.width() as u16, rgb.height() as u16, jpeg_encoder::ColorType::Rgb).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// Writes a thumbnail file whole or not at all.
pub fn write_bytes(bytes: &[u8], dest: &Path) -> Result<(), String> {
    if let Some(p) = dest.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let tmp = dest.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())
}

/// Build the thumbnail file for one asset and compute palette and look from it.
/// `link` is the asset's source URL for link items.
pub fn generate_with(src: &Path, ext: &str, kind: AssetKind, dest: &Path, handlers: &[ExternalHandler], tmp_dir: &Path, link: Option<&str>) -> Result<ThumbResult, String> {
    let max = THUMBNAIL_SIZE;
    let (img, mut res) = decode_full(src, ext, kind, max, handlers, tmp_dir, link)?;
    let (w, h) = img.dimensions();
    if res.dims.is_none() && kind == AssetKind::Image {
        // images without a readable header take their dimensions from the decoded picture
        res.dims = Some((w as i64, h as i64));
    }
    let thumb = crate::core::thumbs::previews::fit_fast(img, max);
    let (tw, th) = thumb.dimensions();
    if res.dims.is_none() && tw > 0 && th > 0 {
        res.aspect = Some(tw as f64 / th as f64);
    }
    let rgba = thumb.to_rgba8();
    write_image(&rgba, dest, THUMBNAIL_QUALITY, if kind == AssetKind::Image { source_icc(src) } else { None })?;
    res.palette = color::palette_from_rgba(rgba.as_raw());
    res.look = Some(crate::core::duplicates::fingerprints::Look::of(&thumb));
    res.has_thumb = true;
    Ok(res)
}

pub fn generate(src: &Path, ext: &str, kind: AssetKind, dest: &Path, handlers: &[ExternalHandler], tmp_dir: &Path) -> Result<ThumbResult, String> {
    generate_with(src, ext, kind, dest, handlers, tmp_dir, None)
}

/// A picture chosen by the person as the thumbnail, sized and encoded.
pub struct CustomThumb {
    pub bytes: Vec<u8>,
    pub aspect: f64,
    pub palette: Vec<PaletteColor>,
    pub look: crate::core::duplicates::fingerprints::Look,
}

pub fn custom_thumbnail(img: &DynamicImage) -> Result<CustomThumb, String> {
    let max = THUMBNAIL_SIZE;
    let (w, h) = img.dimensions();
    let thumb = if w > max || h > max { img.thumbnail(max, max) } else { img.clone() };
    let (tw, th) = thumb.dimensions();
    let rgba = thumb.to_rgba8();
    Ok(CustomThumb { bytes: encode_image(&rgba, THUMBNAIL_QUALITY, None)?, aspect: tw as f64 / th.max(1) as f64, palette: color::palette_from_rgba(rgba.as_raw()), look: crate::core::duplicates::fingerprints::Look::of(&thumb) })
}

/// `custom_thumbnail` written to `dest`. Returns its colours, look and aspect ratio.
pub fn write_custom_thumbnail(img: &DynamicImage, dest: &Path) -> Result<(Vec<PaletteColor>, crate::core::duplicates::fingerprints::Look, f64), String> {
    let t = custom_thumbnail(img)?;
    write_bytes(&t.bytes, dest)?;
    Ok((t.palette, t.look, t.aspect))
}

/// Thumbnail cache size control: delete the oldest cache files until under `max_bytes`.
/// Remove the oldest files across `dirs` until they fit in `max_bytes` together. Returns the size left.
pub fn enforce_cache_limit(dirs: &[&Path], max_bytes: u64) -> u64 {
    let mut files: Vec<(std::time::SystemTime, u64, PathBuf)> = dirs
        .iter()
        .flat_map(|d| walkdir::WalkDir::new(d))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let md = e.metadata().ok()?;
            Some((md.modified().ok()?, md.len(), e.into_path()))
        })
        .collect();
    let total: u64 = files.iter().map(|f| f.1).sum();
    if total <= max_bytes {
        return total;
    }
    files.sort_by_key(|f| f.0);
    let mut cur = total;
    for (_, len, p) in files {
        if cur <= max_bytes {
            break;
        }
        if std::fs::remove_file(&p).is_ok() {
            cur -= len;
        }
    }
    cur
}

pub mod log {
    pub fn warn(msg: &str) {
        crate::core::system::logging::log("warn", msg);
    }
}
