//! Turning a picture for good: the file on disk is rewritten, keeping the metadata it carried
//! (AI prompts in PNG text chunks, EXIF in JPEG), so turning never loses what the library read from it.
//! JPG, PNG, BMP and non-animated WebP can be turned.

use std::path::Path;

use image::{DynamicImage, GenericImageView, ImageEncoder};

const JPEG_QUALITY: u8 = 95;

pub fn can_rotate(ext: &str, animated: bool) -> bool {
    !animated && matches!(ext, "jpg" | "jpeg" | "jfif" | "jpe" | "png" | "bmp" | "webp")
}

/// Turn by `turns` x 90 degrees clockwise. Returns the new width and height.
pub fn rotate_in_place(path: &Path, ext: &str, turns: u32) -> Result<(u32, u32), String> {
    let original = std::fs::read(path).map_err(|e| e.to_string())?;
    let img = crate::core::thumbs::decode::decode_with_orientation(path)?;
    let img = match turns % 4 {
        1 => img.rotate90(),
        2 => img.rotate180(),
        3 => img.rotate270(),
        _ => img,
    };
    let (w, h) = img.dimensions();
    let out = match ext {
        "jpg" | "jpeg" | "jfif" | "jpe" => encode_jpeg(&img, &original)?,
        "png" => encode_png(&img, &original)?,
        "bmp" => encode_with(&img, image::ImageFormat::Bmp)?,
        // the encoder available here writes lossless WebP
        "webp" => encode_with(&DynamicImage::ImageRgba8(img.to_rgba8()), image::ImageFormat::WebP)?,
        other => return Err(format!(".{other} files can't be turned")),
    };
    // beside the original first, so a failure never leaves half a file
    let tmp = path.with_extension(format!("{}.turning", fastrand::u32(..)));
    let written = std::fs::File::create(&tmp).and_then(|mut f| {
        use std::io::Write;
        f.write_all(&out)?;
        f.sync_all()
    });
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })?;
    Ok((w, h))
}

fn encode_with(img: &DynamicImage, format: image::ImageFormat) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), format).map_err(|e| e.to_string())?;
    Ok(out)
}

fn encode_jpeg(img: &DynamicImage, original: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let (w, h) = img.dimensions();
    let mut enc = jpeg_encoder::Encoder::new(&mut out, JPEG_QUALITY);
    enc.set_sampling_factor(jpeg_encoder::SamplingFactor::R_4_4_4);
    enc.encode(img.to_rgb8().as_raw(), w as u16, h as u16, jpeg_encoder::ColorType::Rgb).map_err(|e| e.to_string())?;
    Ok(insert_jpeg_metadata(out, &jpeg_metadata(original)))
}

/// EXIF, ICC, Photoshop and comment segments of a JPEG. The stored turn is reset, since the pixels now carry it.
pub fn jpeg_metadata(b: &[u8]) -> Vec<u8> {
    if b.len() < 4 || b[0] != 0xFF || b[1] != 0xD8 {
        return Vec::new();
    }
    let mut keep = Vec::new();
    let mut i = 2;
    while i < b.len() && b[i] == 0xFF {
        while i < b.len() && b[i] == 0xFF {
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        let m = b[i];
        i += 1;
        if m == 0xDA || m == 0xD9 {
            break;
        }
        if m == 0x00 || m == 0x01 || (0xD0..=0xD8).contains(&m) {
            continue;
        }
        if i + 2 > b.len() {
            break;
        }
        let len = u16::from_be_bytes([b[i], b[i + 1]]) as usize;
        if len < 2 || i.checked_add(len).is_none_or(|end| end > b.len()) {
            break;
        }
        if matches!(m, 0xE1 | 0xE2 | 0xED | 0xFE) {
            let mut seg = Vec::with_capacity(2 + len);
            seg.push(0xFF);
            seg.push(m);
            seg.extend_from_slice(&b[i..i + len]);
            if m == 0xE1 && seg.len() > 10 && &seg[4..10] == b"Exif\0\0" {
                reset_orientation(&mut seg[10..]);
            }
            keep.extend_from_slice(&seg);
        }
        i += len;
    }
    keep
}

/// Set the Orientation tag of an EXIF block to "normal".
fn reset_orientation(tiff: &mut [u8]) {
    if tiff.len() < 8 {
        return;
    }
    let le = match &tiff[..2] {
        b"II" => true,
        b"MM" => false,
        _ => return,
    };
    let u16_at = |t: &[u8], at: usize| if le { u16::from_le_bytes([t[at], t[at + 1]]) } else { u16::from_be_bytes([t[at], t[at + 1]]) };
    let ifd = if le { u32::from_le_bytes([tiff[4], tiff[5], tiff[6], tiff[7]]) } else { u32::from_be_bytes([tiff[4], tiff[5], tiff[6], tiff[7]]) } as usize;
    if ifd.checked_add(2).is_none_or(|end| end > tiff.len()) {
        return;
    }
    let count = u16_at(tiff, ifd) as usize;
    for n in 0..count {
        let Some(at) = ifd.checked_add(2).and_then(|x| n.checked_mul(12).and_then(|o| x.checked_add(o))) else { return };
        if at.checked_add(12).is_none_or(|end| end > tiff.len()) {
            return;
        }
        if u16_at(tiff, at) == 0x0112 {
            let one = if le { [1, 0] } else { [0, 1] };
            tiff[at + 8..at + 10].copy_from_slice(&one);
            return;
        }
    }
}

/// Put segments after the opening of a new JPEG (and its JFIF header, which has to stay first).
fn insert_jpeg_metadata(new: Vec<u8>, meta: &[u8]) -> Vec<u8> {
    if meta.is_empty() || new.len() < 4 {
        return new;
    }
    let mut at = 2;
    if new[2] == 0xFF && new[3] == 0xE0 && new.len() > 6 {
        at = 4 + u16::from_be_bytes([new[4], new[5]]) as usize;
    }
    let mut out = Vec::with_capacity(new.len() + meta.len());
    out.extend_from_slice(&new[..at]);
    out.extend_from_slice(meta);
    out.extend_from_slice(&new[at..]);
    out
}

fn encode_png(img: &DynamicImage, original: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let (w, h) = img.dimensions();
    let enc = image::codecs::png::PngEncoder::new(&mut out);
    if img.color().has_alpha() {
        enc.write_image(img.to_rgba8().as_raw(), w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())?;
    } else {
        enc.write_image(img.to_rgb8().as_raw(), w, h, image::ExtendedColorType::Rgb8).map_err(|e| e.to_string())?;
    }
    Ok(insert_png_chunks(out, &png_metadata(original)))
}

/// Text, EXIF, colour profile and density chunks of a PNG; EXIF orientation is reset since the pixels now carry the turn.
pub fn png_metadata(b: &[u8]) -> Vec<u8> {
    if b.len() < 8 || &b[..8] != b"\x89PNG\r\n\x1a\n" {
        return Vec::new();
    }
    // encode_png writes RGB/RGBA; copying a grayscale ICC profile into an RGB PNG produces an invalid file
    let grayscale = b.len() >= 26 && &b[12..16] == b"IHDR" && matches!(b[25], 0 | 4);
    let mut keep = Vec::new();
    let mut i = 8;
    while i + 12 <= b.len() {
        let len = u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize;
        let Some(end) = i.checked_add(12).and_then(|x| x.checked_add(len)) else { break };
        if end > b.len() {
            break;
        }
        let ctype = &b[i + 4..i + 8];
        if ctype == b"eXIf" {
            let mut data = b[i + 8..i + 8 + len].to_vec();
            reset_orientation(&mut data);
            let mut crc = flate2::Crc::new();
            crc.update(ctype);
            crc.update(&data);
            keep.extend_from_slice(&(data.len() as u32).to_be_bytes());
            keep.extend_from_slice(ctype);
            keep.extend_from_slice(&data);
            keep.extend_from_slice(&crc.sum().to_be_bytes());
        } else if matches!(ctype, b"tEXt" | b"zTXt" | b"iTXt" | b"pHYs") || (ctype == b"iCCP" && !grayscale) {
            keep.extend_from_slice(&b[i..end]);
        }
        i = end;
    }
    keep
}

const PNG_HEADER_AND_IHDR: usize = 33;

fn insert_png_chunks(new: Vec<u8>, chunks: &[u8]) -> Vec<u8> {
    if chunks.is_empty() || new.len() < PNG_HEADER_AND_IHDR {
        return new;
    }
    let mut out = Vec::with_capacity(new.len() + chunks.len());
    out.extend_from_slice(&new[..PNG_HEADER_AND_IHDR]);
    out.extend_from_slice(chunks);
    out.extend_from_slice(&new[PNG_HEADER_AND_IHDR..]);
    out
}
