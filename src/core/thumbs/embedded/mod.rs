//! Preview pictures that files carry inside them, for formats Curio can't draw itself: camera RAW, PostScript (EPS, AI), InDesign, Blender, SketchUp, AutoCAD, and the zip packages of Sketch, XMind, Keynote and IDML.
//! Files are untrusted: every read and decode is bounded, and a malformed file means no picture, never a panic.

mod autocad;
mod blender;
mod indesign;
mod packages;
mod postscript;
pub mod raw;
mod sketchup;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use image::{DynamicImage, ImageDecoder};

/// The most bytes one embedded picture may take in its file.
const MAX_PICTURE: u64 = 32 << 20;
const MAX_SIDE: u32 = 16_384;
const MAX_ALLOC: u64 = 256 << 20;

pub fn handles(ext: &str) -> bool {
    raw::handles(ext) || postscript(ext) || packages::handles(ext) || matches!(ext, "indd" | "indt" | "blend" | "skp" | "dwg")
}

/// PostScript files, whose previews are small: a render is tried before them.
pub fn postscript(ext: &str) -> bool {
    matches!(ext, "eps" | "ai" | "ait")
}

/// The picture inside the file, upright; `None` when it carries none Curio reads.
pub fn preview(path: &Path, ext: &str) -> Option<DynamicImage> {
    match ext {
        _ if raw::handles(ext) => raw::preview(path),
        _ if packages::handles(ext) => packages::preview(path, ext),
        "eps" | "ai" | "ait" => postscript::preview(path),
        "indd" | "indt" => indesign::preview(path),
        "blend" => blender::preview(path),
        "skp" => sketchup::preview(path),
        "dwg" => autocad::preview(path),
        _ => None,
    }
}

fn limits() -> image::Limits {
    let mut l = image::Limits::default();
    l.max_image_width = Some(MAX_SIDE);
    l.max_image_height = Some(MAX_SIDE);
    l.max_alloc = Some(MAX_ALLOC);
    l
}

/// A decoder's picture within the limits, turned by its own orientation when `turn` is set.
fn decoded(mut d: impl ImageDecoder, turn: bool) -> Option<DynamicImage> {
    d.set_limits(limits()).ok()?;
    let o = if turn { d.orientation().ok() } else { None };
    let mut img = DynamicImage::from_decoder(d).ok()?;
    if let Some(o) = o {
        img.apply_orientation(o);
    }
    Some(img)
}

/// An encoded picture of any format the image crate reads, upright.
fn picture(bytes: &[u8]) -> Option<DynamicImage> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().ok()?;
    decoded(reader.into_decoder().ok()?, true)
}

/// `len` bytes at `offset`, only when they lie inside the file and fit [`MAX_PICTURE`].
fn bytes_at(file: &mut File, offset: u64, len: u64) -> Option<Vec<u8>> {
    let size = file.metadata().ok()?.len();
    if len == 0 || len > MAX_PICTURE || offset.checked_add(len)? > size {
        return None;
    }
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = vec![0; len as usize];
    file.read_exact(&mut buf).ok()?;
    Some(buf)
}

/// The first `len` bytes of the file, or all of a shorter one.
fn head(path: &Path, len: u64) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    File::open(path).ok()?.take(len).read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn position(hay: &[u8], needle: &[u8]) -> Option<usize> {
    (!needle.is_empty()).then(|| hay.windows(needle.len()).position(|w| w == needle)).flatten()
}

fn u32_le(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at.checked_add(4)?)?.try_into().ok()?))
}

/// Base64 from an XML text node: whitespace and numeric character references (`&#xA;`, `&#13;`) between its lines are dropped; any other `&` makes it unreadable.
fn xml_base64(text: &[u8]) -> Option<Vec<u8>> {
    use base64::Engine;
    let mut clean = Vec::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        match text[i] {
            b'&' => {
                let end = text[i..].iter().take(10).position(|&c| c == b';')? + i;
                let num = text.get(i + 1..end)?.strip_prefix(b"#")?;
                let digits = match num.first() {
                    Some(b'x' | b'X') => &num[1..],
                    _ => num,
                };
                if digits.is_empty() || !digits.iter().all(u8::is_ascii_hexdigit) {
                    return None;
                }
                i = end + 1;
            }
            c if c.is_ascii_whitespace() => i += 1,
            c => {
                clean.push(c);
                i += 1;
            }
        }
    }
    base64::engine::general_purpose::STANDARD.decode(clean).ok()
}

/// Calls `found` with the text between each `open` and the next `close` in the file, read a chunk at a time; stops when `found` returns true, after `max_hits` texts, or once [`MAX_SCAN`] bytes are read.
/// A text longer than `max_text` is skipped.
fn each_between(path: &Path, open: &[u8], close: &[u8], max_text: usize, max_hits: usize, mut found: impl FnMut(&[u8]) -> bool) {
    const MAX_SCAN: u64 = 1 << 30;
    const CHUNK: usize = 4 << 20;
    let Ok(mut file) = File::open(path) else { return };
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = vec![0; CHUNK];
    let (mut read, mut hits) = (0u64, 0usize);
    loop {
        let n = match file.read(&mut chunk) {
            Ok(n) => n,
            Err(_) => return,
        };
        buf.extend_from_slice(&chunk[..n]);
        read += n as u64;
        // take every complete text in what's held
        loop {
            let Some(s) = position(&buf, open) else {
                let keep = buf.len().min(open.len() - 1);
                buf.drain(..buf.len() - keep);
                break;
            };
            buf.drain(..s);
            match position(&buf[open.len()..], close) {
                Some(e) => {
                    // an opening that was cut off: the text starts at the last opening before the close
                    if let Some(last) = buf[open.len()..open.len() + e].windows(open.len()).rposition(|w| w == open) {
                        buf.drain(..open.len() + last);
                        continue;
                    }
                    if e <= max_text {
                        hits += 1;
                        if found(&buf[open.len()..open.len() + e]) || hits >= max_hits {
                            return;
                        }
                    }
                    buf.drain(..open.len() + e + close.len());
                }
                None if buf.len() > open.len() + max_text + close.len() => {
                    // never closed within reach: look past this opening
                    buf.drain(..open.len());
                }
                None => break,
            }
        }
        if n == 0 || read >= MAX_SCAN {
            return;
        }
    }
}
