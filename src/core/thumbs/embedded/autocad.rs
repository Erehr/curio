//! AutoCAD drawings (R13 and later) point at an image section from byte 13 of the file: a fixed 16-byte sentinel, the section's size, a count, then per picture a type, start and size.
//! Type 2 is a BMP without its file header, 6 a PNG; type 3, a Windows metafile, isn't read.

use std::fs::File;
use std::path::Path;

use image::DynamicImage;

const SENTINEL: [u8; 16] = [0x1F, 0x25, 0x6D, 0x07, 0xD4, 0x36, 0x28, 0x28, 0x9D, 0x57, 0xCA, 0x3F, 0x9D, 0x44, 0x10, 0x2B];

pub fn preview(path: &Path) -> Option<DynamicImage> {
    let start = super::head(path, 17)?;
    if !start.starts_with(b"AC10") {
        return None;
    }
    let mut f = File::open(path).ok()?;
    let section = u64::from(super::u32_le(&start, 13)?);
    let top = super::bytes_at(&mut f, section, 21)?;
    if top[..16] != SENTINEL {
        return None;
    }
    let count = u64::from(top[20]);
    let table = super::bytes_at(&mut f, section + 21, count * 9)?;
    for entry in table.chunks_exact(9) {
        let (kind, at, len) = (entry[0], super::u32_le(entry, 1)?, super::u32_le(entry, 5)?);
        let read = |f: &mut File| super::bytes_at(f, at.into(), len.into());
        let img = match kind {
            2 => read(&mut f).and_then(|b| dib(&b)),
            6 => read(&mut f).and_then(|b| super::picture(&b)),
            _ => None,
        };
        if img.is_some() {
            return img;
        }
    }
    None
}

fn dib(b: &[u8]) -> Option<DynamicImage> {
    let decoder = image::codecs::bmp::BmpDecoder::new_without_file_header(std::io::Cursor::new(b)).ok()?;
    super::decoded(decoder, false)
}
