//! SketchUp models: since SketchUp 2021 a zip package with a thumbnail entry, before that a stream that holds the model's thumbnail as its first PNG, followed by material textures.

use std::path::Path;

use image::DynamicImage;

/// The thumbnail comes before the model and textures.
const START: u64 = 32 << 20;
/// A smaller first PNG is likely a texture swatch; a larger one later is preferred.
const MIN_SIDE: u32 = 64;

pub fn preview(path: &Path) -> Option<DynamicImage> {
    let d = super::head(path, START)?;
    if d.starts_with(b"PK\x03\x04") {
        return super::packages::preview(path, "skp");
    }
    let mut fallback = None;
    for png in pngs(&d).take(16) {
        let Some(img) = super::picture(png) else { continue };
        if img.width().min(img.height()) >= MIN_SIDE {
            return Some(img);
        }
        fallback.get_or_insert(img);
    }
    fallback
}

/// Each complete PNG in `d`, found by its signature and measured by walking its chunks to `IEND`.
fn pngs(d: &[u8]) -> impl Iterator<Item = &[u8]> {
    const SIG: &[u8] = b"\x89PNG\r\n\x1a\n";
    let mut from = 0;
    std::iter::from_fn(move || loop {
        let start = from + super::position(d.get(from..)?, SIG)?;
        from = start + SIG.len();
        let mut at = from;
        let end = loop {
            let len = u32::from_be_bytes(d.get(at..at + 4)?.try_into().ok()?) as usize;
            let kind = d.get(at + 4..at + 8)?;
            // length, type, data and CRC
            let next = at.checked_add(12)?.checked_add(len)?;
            if next > d.len() || len > super::MAX_PICTURE as usize {
                break None;
            }
            if kind == b"IEND" {
                break Some(next);
            }
            at = next;
        };
        // the next search starts inside this one, as a PNG cut short may have run into the next
        if let Some(end) = end {
            return Some(&d[start..end]);
        }
    })
}
