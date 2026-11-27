//! Previews in PostScript files: the TIFF that a binary EPS header points to, an EPSI preview (`%%BeginPreview`), or an Illustrator thumbnail (`%AI7_Thumbnail`).

use std::fs::File;
use std::path::Path;

use image::{DynamicImage, GrayImage, RgbImage};

/// How much of the PostScript is searched for a preview: previews sit in the header comments.
const PS_HEAD: u64 = 8 << 20;
const MAX_PIXELS: usize = 16 << 20;

pub fn preview(path: &Path) -> Option<DynamicImage> {
    let start = super::head(path, 30)?;
    if start.starts_with(&[0xC5, 0xD0, 0xD3, 0xC6]) {
        // binary header: PostScript offset and length, WMF, then TIFF offset and length
        let mut f = File::open(path).ok()?;
        let tiff = super::u32_le(&start, 20).zip(super::u32_le(&start, 24)).and_then(|(at, len)| super::bytes_at(&mut f, at.into(), len.into())).and_then(|b| super::picture(&b));
        if tiff.is_some() {
            return tiff;
        }
        let (at, len) = (super::u32_le(&start, 4)?, super::u32_le(&start, 8)?);
        let ps = super::bytes_at(&mut f, at.into(), u64::from(len).min(PS_HEAD))?;
        return from_text(&ps);
    }
    from_text(&super::head(path, PS_HEAD)?)
}

fn from_text(ps: &[u8]) -> Option<DynamicImage> {
    let mut lines = ps.split(|&b| b == b'\n' || b == b'\r').map(<[u8]>::trim_ascii);
    while let Some(line) = lines.next() {
        if let Some(rest) = line.strip_prefix(b"%%BeginPreview:") {
            if let Some(img) = epsi(numbers(rest), lines.clone()) {
                return Some(img);
            }
        } else if let Some(rest) = line.strip_prefix(b"%AI7_Thumbnail:") {
            if let Some(img) = ai_thumbnail(numbers(rest), lines.clone()) {
                return Some(img);
            }
        }
    }
    None
}

fn numbers(s: &[u8]) -> Vec<usize> {
    String::from_utf8_lossy(s).split_whitespace().map_while(|t| t.parse().ok()).collect()
}

/// The hex digits of comment lines (`%` then hex) until a line starting with `end`, as bytes; `None` for an odd count or anything that isn't hex.
fn hex_lines<'a>(lines: impl Iterator<Item = &'a [u8]>, end: &[u8], limit: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut high: Option<u8> = None;
    for line in lines {
        if line.starts_with(end) {
            break;
        }
        let Some(hex) = line.strip_prefix(b"%") else { continue };
        for &c in hex.iter().filter(|c| !c.is_ascii_whitespace()) {
            let v = (c as char).to_digit(16)? as u8;
            match high.take() {
                Some(h) => out.push(h << 4 | v),
                None => high = Some(v),
            }
        }
        if out.len() > limit {
            return None;
        }
    }
    high.is_none().then_some(out)
}

/// `width height depth lines`, then rows of grey samples, each row starting on a byte, 0 black.
fn epsi<'a>(n: Vec<usize>, lines: impl Iterator<Item = &'a [u8]>) -> Option<DynamicImage> {
    let (&w, &h, &depth) = (n.first()?, n.get(1)?, n.get(2)?);
    if w == 0 || h == 0 || w.checked_mul(h)? > MAX_PIXELS || !matches!(depth, 1 | 2 | 4 | 8) {
        return None;
    }
    let row = (w * depth).div_ceil(8);
    let bytes = hex_lines(lines, b"%%EndPreview", row * h + row)?;
    if bytes.len() < row * h {
        return None;
    }
    let top = (1u16 << depth) - 1;
    Some(DynamicImage::ImageLuma8(GrayImage::from_fn(w as u32, h as u32, |x, y| {
        let bit = x as usize * depth;
        let byte = bytes[y as usize * row + bit / 8];
        let v = (u16::from(byte) >> (8 - depth - bit % 8)) & top;
        image::Luma([(v * 255 / top) as u8])
    })))
}

/// `width height 8`, then in a `%%BeginData` block: 256 RGB palette entries, `RLE`, and palette indexes where `FD n v` repeats `v` n times and `FD FD` stands for FD itself.
fn ai_thumbnail<'a>(n: Vec<usize>, lines: impl Iterator<Item = &'a [u8]>) -> Option<DynamicImage> {
    let (&w, &h, &depth) = (n.first()?, n.get(1)?, n.get(2)?);
    let px = w.checked_mul(h)?;
    if w == 0 || h == 0 || px > MAX_PIXELS || depth != 8 {
        return None;
    }
    let mut lines = lines.skip_while(|l| !l.starts_with(b"%%BeginData"));
    lines.next()?;
    let bytes = hex_lines(lines, b"%%EndData", 771 + px * 3)?;
    let (palette, rest) = (bytes.get(..768)?, bytes.get(768..)?);
    let runs = rest.strip_prefix(b"RLE")?;
    let mut idx = Vec::with_capacity(px);
    let mut i = 0;
    while i < runs.len() {
        let (value, count, step) = match runs[i] {
            0xFD if runs.get(i + 1) == Some(&0xFD) => (0xFD, 1, 2),
            0xFD => (*runs.get(i + 2)?, *runs.get(i + 1)? as usize, 3),
            v => (v, 1, 1),
        };
        if count == 0 || idx.len() + count > px {
            return None;
        }
        idx.extend(std::iter::repeat_n(value, count));
        i += step;
    }
    if idx.len() != px {
        return None;
    }
    let rgb = idx.iter().flat_map(|&p| palette[p as usize * 3..p as usize * 3 + 3].iter().copied()).collect();
    RgbImage::from_raw(w as u32, h as u32, rgb).map(DynamicImage::ImageRgb8)
}
