//! Files decoded to pictures: images, SVG, PDF and what handlers make.

use crate::core::thumbs::log;
use std::path::Path;
use std::time::Duration;
use image::{DynamicImage, ImageDecoder};
use crate::core::consts::*;
use crate::core::media;
use crate::core::model::AssetKind;
use crate::core::system::process;
use crate::core::thumbs::links::link_image;
use crate::core::thumbs::links::url_from_file;
use crate::core::thumbs::ThumbResult;
use crate::core::thumbs::ExternalHandler;

/// Decode any supported source into an image.
pub fn decode_source(src: &Path, ext: &str, kind: AssetKind, max: u32, handlers: &[ExternalHandler], tmp_dir: &Path) -> Result<DynamicImage, String> {
    decode_full(src, ext, kind, max, handlers, tmp_dir, None).map(|(i, _)| i)
}

/// An archive's first picture in reading order; a JPEG page is decoded at the fraction of its size that still covers `max`, a quarter or less of a full page's work.
fn archive_cover(src: &Path, max: u32) -> Result<DynamicImage, String> {
    let bytes = crate::core::archive::first_picture(src).ok_or("no image inside archive")?;
    if bytes.starts_with(&[0xFF, 0xD8]) {
        if let Ok(img) = crate::core::thumbs::previews::decode_jpeg_scaled_bytes(&bytes, max) {
            return Ok(img);
        }
    }
    image::load_from_memory(&bytes).map_err(|e| e.to_string())
}

pub(super) fn decode_full(
    src: &Path,
    ext: &str,
    kind: AssetKind,
    max: u32,
    handlers: &[ExternalHandler],
    tmp_dir: &Path,
    link: Option<&str>,
) -> Result<(DynamicImage, ThumbResult), String> {
    let _ = std::fs::create_dir_all(tmp_dir);
    let plain = |img: DynamicImage| (img, ThumbResult::default());
    // 1) plugin handlers
    for h in handlers.iter().filter(|h| h.exts.iter().any(|e| e == ext)) {
        let r = run_handler(h, src, tmp_dir, max);
        if let Some(report) = &h.report.0 {
            report(r.as_ref().err().map(String::as_str));
        }
        match r {
            Ok(img) => return Ok(plain(img)),
            Err(err) => log::warn(&format!("plugin {} thumbnail failed: {err}", h.plugin_id)),
        }
    }
    // 2) OS thumbnailer first for the formats it draws best
    if crate::core::thumbs::shell::NATIVE_FIRST.contains(&ext) {
        if let Ok(img) = crate::core::thumbs::shell::thumbnail(src, crate::core::thumbs::shell::NATIVE_THUMB_SIZE) {
            return Ok(plain(img));
        }
    }
    // the preview the file carries; PostScript's are small, so a render is tried before them
    if crate::core::thumbs::embedded::handles(ext) && !crate::core::thumbs::embedded::postscript(ext) {
        if let Some(img) = crate::core::thumbs::embedded::preview(src, ext) {
            return Ok(plain(img));
        }
    }
    // 3) links
    if matches!(ext, "url" | "webloc" | "website") || link.is_some() {
        let url = link.map(String::from).or_else(|| url_from_file(src)).ok_or("no URL in this link file")?;
        let (img, title) = link_image(&url, tmp_dir)?;
        return Ok((img, ThumbResult { title, ..Default::default() }));
    }
    // 4) videos: FFmpeg, or they wait for it
    if kind == AssetKind::Video || media::VIDEO_EXTS.contains(&ext) {
        if media::engine() != media::Engine::Ffmpeg {
            return Err(media::NEEDS_FFMPEG.into());
        }
        {
            let out = tmp_dir.join(format!("vf_{}.png", fastrand::u64(..)));
            let frame = crate::core::media::ffmpeg::video_frame(src, &out, max, None);
            let img = image::open(&out).map_err(|e| e.to_string());
            let _ = std::fs::remove_file(&out);
            let why = match (frame, img) {
                (Ok(p), Ok(img)) => {
                    let dims = match (p.width, p.height) {
                        (Some(w), Some(h)) => Some((w, h)),
                        _ => None,
                    };
                    return Ok((img, ThumbResult { dims, duration: p.duration, ..Default::default() }));
                }
                (Err(e), _) | (_, Err(e)) => e,
            };
            if let Ok(img) = crate::core::thumbs::shell::thumbnail(src, crate::core::thumbs::shell::NATIVE_THUMB_DEFAULT) {
                return Ok(plain(img));
            }
            // FFmpeg stopped at its time limit, as its first runs can be while a Mac checks the new program: the video waits for it and is made again when the library next opens
            if process::timed_out(&why) {
                return Err(media::NEEDS_FFMPEG.into());
            }
            return Err(format!("video thumbnail failed: {why}"));
        }
    }
    // audio: the picture stored in the file, read by lofty whatever the engine; without one the engines may still find it
    if kind == AssetKind::Audio {
        if let Some(img) = crate::core::metadata::audio_tags::cover(src) {
            return Ok((img, ThumbResult { duration: crate::core::metadata::audio_tags::duration(src), ..Default::default() }));
        }
        if media::engine() != media::Engine::Ffmpeg {
            return Err(media::NEEDS_FFMPEG.into());
        }
        {
            let out = tmp_dir.join(format!("cv_{}.png", fastrand::u64(..)));
            if crate::core::media::ffmpeg::audio_cover(src, &out).is_ok() {
                let img = image::open(&out).map_err(|e| e.to_string());
                let _ = std::fs::remove_file(&out);
                if let Ok(i) = img {
                    return Ok(plain(i));
                }
            }
        }
    }
    // 5) built-in decoders
    match ext {
        "svg" => return render_svg(src, max).map(plain),
        // comics too: their cover is their first page
        "zip" | "7z" | "cbz" | "cb7" => return archive_cover(src, max).map(plain),
        _ => {}
    }
    if kind == AssetKind::Image || ext == "ico" {
        if let Ok(img) = decode_with_orientation(src) {
            return Ok(plain(img));
        }
        // 6) FFmpeg reads AVIF, HEIC, PSD, EXR …; an AVIF waits for it
        if ext == "avif" && media::engine() != media::Engine::Ffmpeg {
            return Err(media::NEEDS_FFMPEG.into());
        }
        if media::engine() == media::Engine::Ffmpeg && crate::core::media::ffmpeg::ffmpeg().is_some() {
            let out = tmp_dir.join(format!("im_{}.png", fastrand::u64(..)));
            if crate::core::media::ffmpeg::image_to_png(src, &out, max.max(BIG_IMAGE_PREVIEW_SIZE)).is_ok() {
                let img = image::open(&out).map_err(|e| e.to_string());
                let _ = std::fs::remove_file(&out);
                if let Ok(i) = img {
                    return Ok(plain(i));
                }
            }
        }
    }
    // an Illustrator file saved PDF-compatible is a PDF inside
    if ext == "pdf" || (matches!(ext, "ai" | "ait") && pdf_inside(src)) {
        if let Some(pp) = crate::core::system::filesystem::which("pdftoppm") {
            let prefix = tmp_dir.join(format!("pdf_{}", fastrand::u64(..)));
            let args = vec![
                "-png".into(),
                "-f".into(),
                "1".into(),
                "-l".into(),
                "1".into(),
                "-scale-to".into(),
                format!("{max}"),
                "-singlefile".into(),
                src.to_string_lossy().to_string(),
                prefix.to_string_lossy().to_string(),
            ];
            let _ = process::run(&pp.to_string_lossy(), &args, None, Duration::from_millis(THUMBNAIL_TIMEOUT_MS));
            let out = prefix.with_extension("png");
            if out.exists() {
                let img = image::open(&out).map_err(|e| e.to_string());
                let _ = std::fs::remove_file(&out);
                return img.map(plain);
            }
        }
    }
    // 7) whatever thumbnail provider the OS has (utils/nativeThumb.js `size || 480`); for sound without a cover Quick Look draws a generic note, which Curio's own card beats
    let system_draws = !matches!(kind, AssetKind::Document) || ext == "pdf" || ext.starts_with("doc") || ext.starts_with("xls") || ext.starts_with("ppt");
    if system_draws && !(cfg!(target_os = "macos") && kind == AssetKind::Audio) {
        if let Ok(img) = crate::core::thumbs::shell::thumbnail(src, crate::core::thumbs::shell::NATIVE_THUMB_DEFAULT.max(max)) {
            return Ok(plain(img));
        }
    }
    if crate::core::thumbs::embedded::postscript(ext) {
        if let Some(img) = crate::core::thumbs::embedded::preview(src, ext) {
            return Ok(plain(img));
        }
    }
    Err(format!("no thumbnail generator for .{ext}"))
}

fn pdf_inside(src: &Path) -> bool {
    use std::io::Read;
    let mut head = [0u8; 5];
    std::fs::File::open(src).and_then(|mut f| f.read_exact(&mut head)).is_ok() && &head == b"%PDF-"
}

pub fn decode_with_orientation(src: &Path) -> Result<DynamicImage, String> {
    let reader = image::ImageReader::open(src).map_err(|e| e.to_string())?.with_guessed_format().map_err(|e| e.to_string())?;
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    img.apply_orientation(orientation);
    Ok(img)
}

const SVG_MAX_BYTES: u64 = 32 << 20;

const SVGZ_MAX_BYTES: u64 = 64 << 20;

fn svg_fonts() -> std::sync::Arc<resvg::usvg::fontdb::Database> {
    static FONTS: std::sync::OnceLock<std::sync::Arc<resvg::usvg::fontdb::Database>> = std::sync::OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = resvg::usvg::fontdb::Database::new();
            db.load_system_fonts();
            std::sync::Arc::new(db)
        })
        .clone()
}

/// Embedded `data:` images only: a path or URL in `href` could read local files, network shares (sending this computer's credentials) or devices that never end.
pub(crate) fn svg_options(fonts: bool) -> resvg::usvg::Options<'static> {
    let mut opt = resvg::usvg::Options {
        image_href_resolver: resvg::usvg::ImageHrefResolver { resolve_data: resvg::usvg::ImageHrefResolver::default_data_resolver(), resolve_string: Box::new(|_, _| None) },
        ..Default::default()
    };
    if fonts {
        opt.fontdb = svg_fonts();
    }
    opt
}

pub(crate) fn read_svg(src: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut data = Vec::new();
    std::fs::File::open(src).map_err(|e| e.to_string())?.take(SVG_MAX_BYTES + 1).read_to_end(&mut data).map_err(|e| e.to_string())?;
    if data.len() as u64 > SVG_MAX_BYTES {
        return Err("svg too large".into());
    }
    if data.starts_with(&[0x1f, 0x8b]) {
        let mut plain = Vec::new();
        flate2::read::GzDecoder::new(&data[..]).take(SVGZ_MAX_BYTES + 1).read_to_end(&mut plain).map_err(|e| e.to_string())?;
        if plain.len() as u64 > SVGZ_MAX_BYTES {
            return Err("svg too large".into());
        }
        return Ok(plain);
    }
    Ok(data)
}

fn render_svg(src: &Path, max: u32) -> Result<DynamicImage, String> {
    let data = read_svg(src)?;
    let tree = resvg::usvg::Tree::from_data(&data, &svg_options(true)).map_err(|e| e.to_string())?;
    let size = tree.size();
    let scale = (max as f32 / size.width().max(size.height())).min(4.0);
    let w = ((size.width() * scale).ceil() as u32).max(1);
    let h = ((size.height() * scale).ceil() as u32).max(1);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or("svg too large")?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let mut rgba = pixmap.take();
    for px in rgba.chunks_mut(4) {
        let a = px[3] as u32;
        if a > 0 && a < 255 {
            for c in px.iter_mut().take(3) {
                *c = ((*c as u32 * 255) / a).min(255) as u8;
            }
        }
    }
    let img = image::RgbaImage::from_raw(w, h, rgba).ok_or("svg buffer")?;
    Ok(DynamicImage::ImageRgba8(img))
}

fn run_handler(h: &ExternalHandler, src: &Path, tmp_dir: &Path, max: u32) -> Result<DynamicImage, String> {
    let out = tmp_dir.join(format!("plug_{}.png", fastrand::u64(..)));
    let argv = process::expand(
        &h.command,
        &[("input", src.to_string_lossy().to_string()), ("output", out.to_string_lossy().to_string()), ("size", max.to_string())],
    );
    let (prog, args) = argv.split_first().ok_or("empty command")?;
    let token = crate::core::api::programs::issue(&h.plugin_id);
    let img = match process::run_in(prog, args, None, Duration::from_millis(THUMBNAIL_TIMEOUT_MS), h.dir.as_deref(), &token.env) {
        Ok(r) if !out.exists() => Err(format!("handler wrote no image (exit {:?}): {}", r.status, r.stderr.trim())),
        Ok(_) => image::open(&out).map_err(|e| e.to_string()),
        Err(e) => Err(e),
    };
    let _ = std::fs::remove_file(&out);
    img
}

/// The colour profile a picture carries. Copies made from it keep it, or the window would show them in other colours than the file itself (a wide-gamut photo turns darker when zooming swaps the copy for the file).
pub fn source_icc(src: &Path) -> Option<Vec<u8>> {
    use image::ImageDecoder;
    let mut d = image::ImageReader::open(src).ok()?.with_guessed_format().ok()?.into_decoder().ok()?;
    d.icc_profile().ok().flatten().filter(|p| !p.is_empty())
}
