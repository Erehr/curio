//! Camera RAW files: cameras store a JPEG of the shot beside the sensor data, often in several sizes; the largest is shown.
//! The turn comes from the file's first IFD (TIFF-based formats, with ORF and RW2's own magic numbers), from CR3's `CMT1` block, or else from the JPEG's own EXIF (RAF, X3F).

use std::path::Path;

use image::metadata::Orientation;
use image::DynamicImage;

/// Larger files are not read for their preview.
const MAX_FILE: u64 = 512 << 20;
/// Canon's metadata box inside a CR3's `moov`.
const CANON_BOX: [u8; 16] = [0x85, 0xC0, 0xB6, 0x87, 0x82, 0x0F, 0x11, 0xE0, 0x81, 0x11, 0xF4, 0xCE, 0x46, 0x2B, 0x6A, 0x48];

pub fn handles(ext: &str) -> bool {
    crate::core::formats::image::RAW.contains(&ext)
}

pub fn preview(path: &Path) -> Option<DynamicImage> {
    if std::fs::metadata(path).ok()?.len() > MAX_FILE {
        return None;
    }
    let data = std::fs::read(path).ok()?;
    let jpeg = largest_jpeg(&data)?;
    let turn = file_orientation(&data);
    let reader = image::ImageReader::with_format(std::io::Cursor::new(jpeg), image::ImageFormat::Jpeg);
    let mut img = super::decoded(reader.into_decoder().ok()?, turn.is_none())?;
    if let Some(o) = turn {
        img.apply_orientation(o);
    }
    Some(img)
}

/// The JPEG stream with the most pixels, from its start to the end of the data (the decoder stops at its own end).
fn largest_jpeg(data: &[u8]) -> Option<&[u8]> {
    let mut best: Option<(u64, usize)> = None;
    let mut from = 0;
    while let Some(i) = super::position(&data[from..], &[0xFF, 0xD8, 0xFF]) {
        let at = from + i;
        if let Some(px) = frame_pixels(&data[at..]) {
            if best.is_none_or(|(b, _)| px > b) {
                best = Some((px, at));
            }
        }
        from = at + 3;
    }
    best.map(|(_, at)| &data[at..])
}

/// Width times height from a JPEG's frame header, read by walking its segments; only 8-bit baseline, extended and progressive frames count, as lossless ones hold sensor data.
fn frame_pixels(j: &[u8]) -> Option<u64> {
    let mut at = 2usize;
    for _ in 0..100 {
        if *j.get(at)? != 0xFF {
            return None;
        }
        let marker = *j.get(at + 1)?;
        if marker == 0xFF {
            at += 1;
            continue;
        }
        let len = u16::from_be_bytes([*j.get(at + 2)?, *j.get(at + 3)?]) as usize;
        match marker {
            0xC0 | 0xC1 | 0xC2 => {
                let precision = *j.get(at + 4)?;
                let h = u16::from_be_bytes([*j.get(at + 5)?, *j.get(at + 6)?]);
                let w = u16::from_be_bytes([*j.get(at + 7)?, *j.get(at + 8)?]);
                return (precision == 8 && w > 0 && h > 0).then(|| u64::from(w) * u64::from(h));
            }
            // other frame types (C4, C8 and CC are tables, not frames), or the data begins before any frame
            0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF | 0xD8..=0xDA => return None,
            _ if len >= 2 => at += 2 + len,
            _ => return None,
        }
    }
    None
}

/// The orientation the file itself states, outside its preview.
fn file_orientation(data: &[u8]) -> Option<Orientation> {
    let tiff = match data.get(..4)? {
        b"II*\0" | b"MM\0*" | b"IIRO" | b"IIRS" | b"MMOR" | b"IIU\0" => Some(data),
        _ if data.get(4..8) == Some(b"ftyp") => cr3_cmt1(data),
        _ => None,
    }?;
    Orientation::from_exif(tiff_orientation(tiff)? as u8)
}

/// Tag 0x0112 of a TIFF's first IFD; the magic number after the byte order isn't checked, as raw formats change it.
fn tiff_orientation(t: &[u8]) -> Option<u16> {
    let big = match t.get(..2)? {
        b"II" => false,
        b"MM" => true,
        _ => return None,
    };
    let u16_at = |at: usize| -> Option<u16> {
        let b = [*t.get(at)?, *t.get(at + 1)?];
        Some(if big { u16::from_be_bytes(b) } else { u16::from_le_bytes(b) })
    };
    let u32_at = |at: usize| -> Option<u32> {
        let b: [u8; 4] = t.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(if big { u32::from_be_bytes(b) } else { u32::from_le_bytes(b) })
    };
    let ifd = u32_at(4)? as usize;
    let count = u16_at(ifd)? as usize;
    (0..count.min(512)).find_map(|n| {
        let entry = ifd.checked_add(2 + n * 12)?;
        // SHORT, one value, stored in the entry
        (u16_at(entry)? == 0x0112 && u16_at(entry + 2)? == 3).then(|| u16_at(entry + 8)).flatten()
    }).filter(|o| (1..=8).contains(o))
}

/// Boxes of an ISO media file: type and contents.
fn iso_boxes(mut d: &[u8]) -> impl Iterator<Item = (&[u8], &[u8])> {
    std::iter::from_fn(move || {
        let size = u32::from_be_bytes(d.get(..4)?.try_into().ok()?) as usize;
        let kind = d.get(4..8)?;
        let (start, end) = match size {
            0 => (8, d.len()),
            1 => (16, usize::try_from(u64::from_be_bytes(d.get(8..16)?.try_into().ok()?)).ok()?),
            n => (8, n),
        };
        if end < start || end > d.len() {
            return None;
        }
        let body = &d[start..end];
        d = &d[end..];
        Some((kind, body))
    })
}

/// The TIFF of a CR3's `CMT1` box, which holds the camera's first IFD.
fn cr3_cmt1(data: &[u8]) -> Option<&[u8]> {
    let moov = iso_boxes(data).find(|(k, _)| *k == b"moov")?.1;
    let canon = iso_boxes(moov).find(|(k, b)| *k == b"uuid" && b.starts_with(&CANON_BOX))?.1;
    iso_boxes(&canon[16..]).find(|(k, _)| *k == b"CMT1").map(|(_, b)| b)
}
