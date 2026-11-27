//! Blender keeps the thumbnail of a saved scene in a `TEST` block near the start of the file: width, height, then RGBA rows from the bottom.
//! Files saved compressed are gzip (read) or, from Blender 3.0, zstd (not read).

use std::io::Read;
use std::path::Path;

use image::{DynamicImage, RgbaImage};

/// The thumbnail comes before the scene data, so only the start of the file is needed.
const START: u64 = 16 << 20;
const MAX_SIDE: usize = 4096;

pub fn preview(path: &Path) -> Option<DynamicImage> {
    let raw = super::head(path, START)?;
    if raw.starts_with(&[0x1F, 0x8B]) {
        let mut plain = Vec::new();
        // a cut-off stream still yields its start, which holds the thumbnail
        let _ = flate2::read::GzDecoder::new(raw.as_slice()).take(START).read_to_end(&mut plain);
        return thumbnail(&plain);
    }
    thumbnail(&raw)
}

/// How blocks are laid out, from the file header.
struct Layout {
    first_block: usize,
    big_endian: bool,
    /// Where the block's length is, from the block's start, and its width in bytes.
    length_at: usize,
    length_size: usize,
    header_size: usize,
}

fn layout(d: &[u8]) -> Option<Layout> {
    let h = d.get(..12)?;
    if !h.starts_with(b"BLENDER") {
        return None;
    }
    if h[7].is_ascii_digit() {
        // "BLENDER17-01v0500": header size, file format 1: code, SDNA index, old address, length, count
        let size: usize = std::str::from_utf8(&h[7..9]).ok()?.parse().ok()?;
        return Some(Layout { first_block: size, big_endian: false, length_at: 16, length_size: 8, header_size: 32 });
    }
    // "BLENDER_v279": pointer size ('_' 4, '-' 8), byte order ('v' little, 'V' big): code, length, old address, SDNA index, count
    let ptr = match h[7] {
        b'_' => 4,
        b'-' => 8,
        _ => return None,
    };
    Some(Layout { first_block: 12, big_endian: h[8] == b'V', length_at: 4, length_size: 4, header_size: 16 + ptr })
}

fn thumbnail(d: &[u8]) -> Option<DynamicImage> {
    let l = layout(d)?;
    let int = |at: usize, width: usize| -> Option<u64> {
        let b = d.get(at..at.checked_add(width)?)?;
        let mut v = 0u64;
        for i in 0..width {
            let byte = if l.big_endian { b[i] } else { b[width - 1 - i] };
            v = v << 8 | u64::from(byte);
        }
        Some(v)
    };
    let mut at = l.first_block;
    for _ in 0..100_000 {
        let code = d.get(at..at + 4)?;
        let len = usize::try_from(int(at + l.length_at, l.length_size)?).ok()?;
        let body = at.checked_add(l.header_size)?;
        match code {
            b"ENDB" => return None,
            b"TEST" => {
                let (w, h) = (int(body, 4)? as usize, int(body + 4, 4)? as usize);
                if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE || len < 8 + w * h * 4 {
                    return None;
                }
                let px = d.get(body + 8..body + 8 + w * h * 4)?;
                let mut img = RgbaImage::from_raw(w as u32, h as u32, px.to_vec())?;
                image::imageops::flip_vertical_in_place(&mut img);
                return Some(DynamicImage::ImageRgba8(img));
            }
            _ => at = body.checked_add(len)?,
        }
    }
    None
}
