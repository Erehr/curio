//! Metadata extraction. Header-only probes run during scanning; deeper extraction (text
//! chunks, EXIF, AI-generation parameters) runs in the background metadata queue.
//! Extraction only reads files.

use std::collections::HashMap;
use std::io::BufReader;
use std::path::Path;

use crate::core::model::{FieldDef, FieldType, RawValue};

// the entries of the kinds of files (`formats::plugin_id`), which read these fields from their own files
pub const EXIF_OWNER: &str = "builtin.format.image";
/// Fonts' facts belong to the Fonts entry.
pub const FONT_OWNER: &str = "builtin.format.font";
pub const AUDIO_OWNER: &str = "builtin.format.audio";
pub const VIDEO_OWNER: &str = "builtin.format.video";

/// Field schemas registered by the built-in extractors (they behave exactly like plugin schemas).
pub fn builtin_schemas() -> Vec<FieldDef> {
    let mk = |key: &str, label: &str, t: FieldType, owner: &str, widget: &str, desc: &str| {
        let mut f = FieldDef::new(key, label, t, owner);
        f.widget = widget.into();
        f.description = desc.into();
        f
    };
    let mut v = vec![
        mk("camera", "Camera", FieldType::Text, EXIF_OWNER, "row", ""),
        mk("lens", "Lens", FieldType::Text, EXIF_OWNER, "row", ""),
        mk("iso", "ISO", FieldType::Integer, EXIF_OWNER, "row", ""),
        mk("aperture", "Aperture", FieldType::Float, EXIF_OWNER, "row", "f-number"),
        mk("shutter", "Shutter", FieldType::Text, EXIF_OWNER, "row", "exposure time"),
        mk("focal_length", "Focal length", FieldType::Float, EXIF_OWNER, "row", "mm"),
        mk("taken_at", "Date taken", FieldType::DateTime, EXIF_OWNER, "row", ""),
        mk("artist", "Artist", FieldType::Text, EXIF_OWNER, "row", ""),
        mk("copyright", "Copyright", FieldType::Text, EXIF_OWNER, "row", ""),
        mk("gps_lat", "Latitude", FieldType::Float, EXIF_OWNER, "row", "Degrees, south negative"),
        mk("gps_lon", "Longitude", FieldType::Float, EXIF_OWNER, "row", "Degrees, west negative"),
        mk("gps_alt", "Altitude", FieldType::Float, EXIF_OWNER, "row", "Metres above sea level"),
        // how a video or a sound is encoded; sound files say theirs too
        mk("video_codec", "Video codec", FieldType::Text, VIDEO_OWNER, "row", "Such as H.264 or AV1"),
        mk("audio_codec", "Audio codec", FieldType::Text, VIDEO_OWNER, "row", "Such as AAC or Opus"),
        mk("frame_rate", "Frame rate", FieldType::Float, VIDEO_OWNER, "row", "Frames per second"),
        mk("bit_rate", "Bit rate (kb/s)", FieldType::Integer, VIDEO_OWNER, "row", "For the whole file"),
        mk("channels", "Channels", FieldType::Integer, VIDEO_OWNER, "row", "1 mono, 2 stereo, 6 surround"),
        mk("sample_rate", "Sample rate (Hz)", FieldType::Integer, VIDEO_OWNER, "row", ""),
        mk("audio_codec", "Audio codec", FieldType::Text, AUDIO_OWNER, "row", ""),
        mk("bit_rate", "Bit rate (kb/s)", FieldType::Integer, AUDIO_OWNER, "row", ""),
        mk("channels", "Channels", FieldType::Integer, AUDIO_OWNER, "row", ""),
        mk("sample_rate", "Sample rate (Hz)", FieldType::Integer, AUDIO_OWNER, "row", ""),
        mk("title", "Title", FieldType::Text, AUDIO_OWNER, "row", ""),
        mk("album", "Album", FieldType::Text, AUDIO_OWNER, "row", ""),
        mk("album_artist", "Album artist", FieldType::Text, AUDIO_OWNER, "row", ""),
        mk("track", "Track", FieldType::Integer, AUDIO_OWNER, "row", ""),
        mk("disc", "Disc", FieldType::Integer, AUDIO_OWNER, "row", ""),
        mk("year", "Year", FieldType::Integer, AUDIO_OWNER, "row", ""),
        mk("composer", "Composer", FieldType::Text, AUDIO_OWNER, "row", ""),
        mk("genre", "Genre", FieldType::MultiSelect, AUDIO_OWNER, "chips", ""),
    ];
    // some are only offered, kept once switched on in Settings › Fields
    #[allow(clippy::too_many_arguments)]
    let font = |key: &str, label: &str, t: FieldType, widget: &str, desc: &str, options: &[&str], indexed: bool, filter_kind: &str| {
        let mut f = mk(key, label, t, FONT_OWNER, widget, desc);
        f.options = options.iter().map(|o| o.to_string()).collect();
        f.indexed = indexed;
        f.filter_kind = filter_kind.into();
        f
    };
    v.extend([
        font("font_family", "Font family", FieldType::Text, "", "The family the font belongs to", &[], false, ""),
        font("font_style", "Font style", FieldType::Text, "", "The style within its family, such as Bold Italic", &[], true, ""),
        font("font_weight", "Font weight", FieldType::Integer, "", "100 (thin) to 900 (black); 400 is regular, 700 bold", &[], true, "list"),
        font("font_width", "Font width", FieldType::SingleSelect, "", "", &["Ultra condensed", "Extra condensed", "Condensed", "Semi condensed", "Normal", "Semi expanded", "Expanded", "Extra expanded", "Ultra expanded"], true, ""),
        font("font_italic", "Italic", FieldType::Boolean, "", "", &[], true, ""),
        font("font_class", "Font category", FieldType::SingleSelect, "", "Worked out from the font's own data and name; empty when neither tells", &crate::core::metadata::fonts::classify::ALL, true, ""),
        font("font_scripts", "Scripts", FieldType::MultiSelect, "chips", "Writing systems the font has the letters for", &crate::core::metadata::fonts::scripts::all(), true, ""),
        font("font_axes", "Variable axes", FieldType::MultiSelect, "chips", "What a variable font can change, such as its weight or width", &[], true, ""),
        font("glyphs", "Glyphs", FieldType::Integer, "", "", &[], false, ""),
        font("font_faces", "Faces", FieldType::Integer, "", "The fonts a collection file holds", &[], false, ""),
        font("designer", "Designer", FieldType::Text, "", "", &[], true, ""),
        font("foundry", "Foundry", FieldType::Text, "", "The company that made the font", &[], false, ""),
        font("font_version", "Font version", FieldType::Text, "", "", &[], false, ""),
        font("font_license", "Font license", FieldType::Text, "", "A common licence by its short name, such as OFL-1.1, else the first sentence of the font's own", &[], true, ""),
    ]);
    v
}

/// Header-only dimension probe (fast; used during scans).
pub fn probe_dims(path: &Path, ext: &str) -> Option<(i64, i64)> {
    if ext == "svg" {
        let data = crate::core::thumbs::decode::read_svg(path).ok()?;
        let tree = resvg::usvg::Tree::from_data(&data, &crate::core::thumbs::decode::svg_options(false)).ok()?;
        let s = tree.size();
        return Some((s.width().round() as i64, s.height().round() as i64));
    }
    match imagesize::size(path) {
        Ok(s) if s.width > 0 && s.height > 0 => Some((s.width as i64, s.height as i64)),
        _ => None,
    }
}

pub fn png_text_chunks(path: &Path) -> Result<HashMap<String, String>, String> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let decoder = png::Decoder::new(BufReader::new(f));
    let reader = decoder.read_info().map_err(|e| e.to_string())?;
    let info = reader.info();
    let mut map = HashMap::new();
    for t in &info.uncompressed_latin1_text {
        map.insert(t.keyword.clone(), t.text.clone());
    }
    for t in &info.compressed_latin1_text {
        if let Ok(s) = t.get_text() {
            map.insert(t.keyword.clone(), s);
        }
    }
    for t in &info.utf8_text {
        if let Ok(s) = t.get_text() {
            map.insert(t.keyword.clone(), s);
        }
    }
    Ok(map)
}

pub(crate) fn exif_fields(path: &Path) -> Vec<(String, RawValue)> {
    let mut out = Vec::new();
    let Ok(f) = std::fs::File::open(path) else { return out };
    let mut r = BufReader::new(f);
    let ex = match exif::Reader::new().read_from_container(&mut r) {
        Ok(ex) => ex,
        Err(_) => match webp_exif(path).and_then(|raw| exif::Reader::new().read_raw(without_child_links(raw)).ok()) {
            Some(ex) => ex,
            None => return out,
        },
    };
    let get = |tag: exif::Tag| ex.get_field(tag, exif::In::PRIMARY);
    let text = |tag: exif::Tag| get(tag).map(|f| f.display_value().to_string().trim_matches('"').trim().to_string());
    // ComfyUI saves WebP with its graphs as "prompt:{...}" and "workflow:{...}" in text tags, Make and Model among them: those aren't a camera
    let mut taken: Vec<exif::Tag> = vec![];
    for f in ex.fields().filter(|f| f.ifd_num == exif::In::PRIMARY) {
        let exif::Value::Ascii(parts) = &f.value else { continue };
        let Some(raw) = parts.first() else { continue };
        let t = String::from_utf8_lossy(raw);
        let Some((key, body)) = t.split_once(':') else { continue };
        let body = body.trim_start();
        if !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && (body.starts_with('{') || body.starts_with('[')) && serde_json::from_str::<serde_json::Value>(body).is_ok() {
            taken.push(f.tag);
        }
    }
    let camera_text = |tag: exif::Tag| if taken.contains(&tag) { None } else { text(tag) };
    let make = camera_text(exif::Tag::Make).unwrap_or_default();
    let model = camera_text(exif::Tag::Model).unwrap_or_default();
    let cam = if model.starts_with(&make) { model.clone() } else { format!("{make} {model}").trim().to_string() };
    if !cam.is_empty() {
        out.push(("camera".into(), RawValue::text(cam)));
    }
    if let Some(l) = text(exif::Tag::LensModel) {
        if !l.is_empty() {
            out.push(("lens".into(), RawValue::text(l)));
        }
    }
    if let Some(f) = get(exif::Tag::PhotographicSensitivity) {
        if let Some(v) = f.value.get_uint(0) {
            out.push(("iso".into(), RawValue::Num(v as f64)));
        }
    }
    let rational = |tag: exif::Tag| -> Option<f64> {
        match &get(tag)?.value {
            exif::Value::Rational(v) if !v.is_empty() => Some(v[0].to_f64()),
            _ => None,
        }
    };
    if let Some(v) = rational(exif::Tag::FNumber) {
        out.push(("aperture".into(), RawValue::Num((v * 10.0).round() / 10.0)));
    }
    if let Some(v) = rational(exif::Tag::FocalLength) {
        out.push(("focal_length".into(), RawValue::Num((v * 10.0).round() / 10.0)));
    }
    if let Some(f) = get(exif::Tag::ExposureTime) {
        out.push(("shutter".into(), RawValue::text(f.display_value().to_string())));
    }
    if let Some(s) = text(exif::Tag::DateTimeOriginal) {
        // "2024-01-02 10:11:12"
        let s = s.replacen(':', "-", 2);
        out.push(("taken_at".into(), RawValue::text(s)));
    }
    if let Some(a) = text(exif::Tag::Artist) {
        if !a.is_empty() {
            out.push(("artist".into(), RawValue::text(a)));
        }
    }
    if let Some(c) = text(exif::Tag::Copyright) {
        if !c.is_empty() {
            out.push(("copyright".into(), RawValue::text(c)));
        }
    }
    if let (Some(lat), Some(lon)) = (gps_degrees(get(exif::Tag::GPSLatitude), get(exif::Tag::GPSLatitudeRef), "S"), gps_degrees(get(exif::Tag::GPSLongitude), get(exif::Tag::GPSLongitudeRef), "W")) {
        // 0,0 is what cameras without a fix write
        if (lat != 0.0 || lon != 0.0) && lat.abs() <= 90.0 && lon.abs() <= 180.0 {
            out.push(("gps_lat".into(), RawValue::Num((lat * 1e6).round() / 1e6)));
            out.push(("gps_lon".into(), RawValue::Num((lon * 1e6).round() / 1e6)));
            if let Some(alt) = rational(exif::Tag::GPSAltitude).filter(|a| a.is_finite()) {
                let below = get(exif::Tag::GPSAltitudeRef).and_then(|f| f.value.get_uint(0)) == Some(1);
                out.push(("gps_alt".into(), RawValue::Num(((if below { -alt } else { alt }) * 10.0).round() / 10.0)));
            }
        }
    }
    out
}

/// Degrees, minutes and seconds as signed degrees; `negative` is the reference letter for south or west.
fn gps_degrees(f: Option<&exif::Field>, reference: Option<&exif::Field>, negative: &str) -> Option<f64> {
    let exif::Value::Rational(v) = &f?.value else { return None };
    let part = |i: usize| v.get(i).map(|r| r.to_f64()).filter(|x| x.is_finite()).unwrap_or(0.0);
    let deg = v.first()?.to_f64() + part(1) / 60.0 + part(2) / 3600.0;
    let flip = reference.is_some_and(|r| r.display_value().to_string().trim_matches('"').trim().eq_ignore_ascii_case(negative));
    deg.is_finite().then_some(if flip { -deg } else { deg })
}

/// The primary image's EXIF tags by name, for plugins that read what the built-in readers don't: text as text (comments in UTF-16 decoded), anything else as EXIF shows it; `None` without EXIF. Stops adding tags past `max_bytes` of text.
pub fn exif_tags(path: &Path, max_bytes: usize) -> Option<serde_json::Map<String, serde_json::Value>> {
    let f = std::fs::File::open(path).ok()?;
    let ex = match exif::Reader::new().read_from_container(&mut BufReader::new(f)) {
        Ok(ex) => ex,
        Err(_) => exif::Reader::new().read_raw(without_child_links(webp_exif(path)?)).ok()?,
    };
    let utf16 = |bytes: &[u8], le: bool| String::from_utf16_lossy(&bytes.chunks(2).map(|c| if le { u16::from_le_bytes([c[0], *c.get(1).unwrap_or(&0)]) } else { u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)]) }).collect::<Vec<u16>>());
    let mut out = serde_json::Map::new();
    let mut size = 0;
    for f in ex.fields().filter(|f| f.ifd_num == exif::In::PRIMARY) {
        let text = match (&f.value, f.tag.number()) {
            (exif::Value::Ascii(parts), _) => parts.iter().map(|p| String::from_utf8_lossy(p).to_string()).collect::<Vec<_>>().join("\n"),
            (exif::Value::Undefined(bytes, _), 0x9286) => {
                let body = bytes.get(8..).unwrap_or_default();
                if bytes.starts_with(b"UNICODE\0") {
                    // the byte order isn't recorded; the reading that yields ASCII text wins
                    let be = utf16(body, false);
                    if be.chars().filter(|c| c.is_ascii()).count() * 2 > be.chars().count() { be } else { utf16(body, true) }
                } else {
                    String::from_utf8_lossy(body).to_string()
                }
            }
            (exif::Value::Byte(bytes), 0x9c9b..=0x9c9f) => utf16(bytes, true),
            _ => f.display_value().with_unit(&ex).to_string(),
        };
        let text = text.trim_end_matches('\0').to_string();
        size += text.len();
        if size > max_bytes {
            break;
        }
        let name = match f.tag.number() {
            0x9c9b => "XPTitle".to_string(),
            0x9c9c => "XPComment".to_string(),
            0x9c9d => "XPAuthor".to_string(),
            0x9c9e => "XPKeywords".to_string(),
            0x9c9f => "XPSubject".to_string(),
            n if f.tag.description().is_none() => format!("0x{n:04x}"),
            _ => f.tag.to_string(),
        };
        out.insert(name, serde_json::Value::String(text));
    }
    Some(out)
}

/// The EXIF chunk of a WebP file, as TIFF data.
fn webp_exif(path: &Path) -> Option<Vec<u8>> {
    let d = std::fs::read(path).ok()?;
    if d.len() < 12 || &d[..4] != b"RIFF" || &d[8..12] != b"WEBP" {
        return None;
    }
    let mut i = 12;
    while i + 8 <= d.len() {
        let n = u32::from_le_bytes(d[i + 4..i + 8].try_into().ok()?) as usize;
        let body = d.get(i + 8..i + 8 + n)?;
        if &d[i..i + 4] == b"EXIF" {
            let tiff = body.strip_prefix(b"Exif\0\0").unwrap_or(body);
            return Some(tiff.to_vec());
        }
        i += 8 + n + (n & 1);
    }
    None
}

/// TIFF data whose Exif and GPS sub-IFDs claim a following IFD, which they can't have. Pillow writes such files (ComfyUI's WebP among them) and the EXIF reader refuses them whole, so those links are cut.
fn without_child_links(mut t: Vec<u8>) -> Vec<u8> {
    let be = t.starts_with(b"MM");
    let u16_at = |t: &[u8], i: usize| t.get(i..i + 2).map(|b| if be { u16::from_be_bytes([b[0], b[1]]) } else { u16::from_le_bytes([b[0], b[1]]) });
    let u32_at = |t: &[u8], i: usize| t.get(i..i + 4).map(|b| if be { u32::from_be_bytes([b[0], b[1], b[2], b[3]]) } else { u32::from_le_bytes([b[0], b[1], b[2], b[3]]) });
    let Some(ifd0) = u32_at(&t, 4).map(|o| o as usize) else { return t };
    let Some(count) = u16_at(&t, ifd0) else { return t };
    let mut children = vec![];
    for k in 0..count as usize {
        let e = ifd0 + 2 + 12 * k;
        if matches!(u16_at(&t, e), Some(0x8769) | Some(0x8825)) {
            if let Some(o) = u32_at(&t, e + 8) {
                children.push(o as usize);
            }
        }
    }
    for c in children {
        if let Some(n) = u16_at(&t, c) {
            let at = c + 2 + 12 * n as usize;
            if let Some(slot) = t.get_mut(at..at + 4) {
                slot.fill(0);
            }
        }
    }
    t
}

/// Duration via ffprobe, under the FFmpeg plugin. (Under Mediabunny the window reports it with the thumbnail.)
pub fn probe_duration(path: &Path) -> Option<f64> {
    crate::core::media::ffmpeg::probe(path).ok()?.duration
}

/// A codec as people know it, whichever reader named it: Mediabunny's `avc` and ffprobe's `h264` are both H.264, so one filter lists it once.
pub fn codec_label(raw: &str) -> String {
    let c = raw.trim().to_lowercase();
    let known = match c.as_str() {
        "avc" | "avc1" | "h264" => "H.264",
        "hevc" | "hvc1" | "hev1" | "h265" => "H.265",
        "vp8" => "VP8",
        "vp9" => "VP9",
        "av1" => "AV1",
        "mpeg4" => "MPEG-4",
        "mpeg2video" => "MPEG-2",
        "mpeg1video" => "MPEG-1",
        "prores" => "ProRes",
        "dnxhd" => "DNxHD",
        "theora" => "Theora",
        "mjpeg" => "Motion JPEG",
        "vc1" => "VC-1",
        "wmv1" | "wmv2" | "wmv3" => "WMV",
        "h263" => "H.263",
        "aac" | "mp4a" => "AAC",
        "mp3" | "mp3float" => "MP3",
        "mp2" => "MP2",
        "opus" => "Opus",
        "vorbis" => "Vorbis",
        "flac" => "FLAC",
        "alac" => "ALAC",
        "ac3" => "AC-3",
        "eac3" => "E-AC-3",
        "dts" => "DTS",
        "truehd" => "TrueHD",
        "wmav1" | "wmav2" | "wmapro" => "WMA",
        "ape" => "APE",
        "wavpack" => "WavPack",
        "speex" => "Speex",
        "musepack" | "mpc" => "Musepack",
        _ if c.starts_with("pcm") || c == "ulaw" || c == "alaw" => "PCM",
        _ => return raw.trim().to_uppercase(),
    };
    known.into()
}

/// How a video or sound is encoded, as field values; the bit rate is the whole file's, from its size and length, the same whichever reader measured the length.
pub fn encoding_fields(video_codec: Option<&str>, audio_codec: Option<&str>, fps: Option<f64>, channels: Option<i64>, sample_rate: Option<i64>, size: i64, duration: Option<f64>) -> Vec<(String, RawValue)> {
    let mut out = Vec::new();
    if let Some(c) = video_codec.filter(|c| !c.trim().is_empty()) {
        out.push(("video_codec".to_string(), RawValue::text(codec_label(c))));
    }
    if let Some(c) = audio_codec.filter(|c| !c.trim().is_empty()) {
        out.push(("audio_codec".to_string(), RawValue::text(codec_label(c))));
    }
    if let Some(f) = fps.filter(|f| f.is_finite() && *f > 0.0 && *f < 1000.0) {
        out.push(("frame_rate".to_string(), RawValue::Num((f * 1000.0).round() / 1000.0)));
    }
    if let Some(c) = channels.filter(|c| *c > 0) {
        out.push(("channels".to_string(), RawValue::Num(c as f64)));
    }
    if let Some(r) = sample_rate.filter(|r| *r > 0) {
        out.push(("sample_rate".to_string(), RawValue::Num(r as f64)));
    }
    if let Some(d) = duration.filter(|d| d.is_finite() && *d > 0.0) {
        if size > 0 {
            out.push(("bit_rate".to_string(), RawValue::Num((size as f64 * 8.0 / d / 1000.0).round())));
        }
    }
    out
}

// ------------------------------------------------------------------ animated images

/// Does this GIF, WebP or PNG have more than one frame? Only reads what it has to.
pub fn is_animated(path: &Path, ext: &str) -> bool {
    if !matches!(ext, "gif" | "webp" | "png" | "apng") {
        return false;
    }
    let mut head = [0u8; 12];
    if let Ok(mut f) = std::fs::File::open(path) {
        use std::io::Read;
        if f.read_exact(&mut head).is_ok() {
            if &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP" {
                return webp_is_animated(path).unwrap_or(false);
            }
            if &head[0..4] == b"GIF8" {
                return gif_has_frames(path).unwrap_or(false);
            }
            if &head[0..4] == b"\x89PNG" {
                return png_is_animated(path).unwrap_or(false);
            }
        }
    }
    match ext {
        "gif" => gif_has_frames(path).unwrap_or(false),
        "webp" => webp_is_animated(path).unwrap_or(false),
        "png" | "apng" => png_is_animated(path).unwrap_or(false),
        _ => false,
    }
}

fn gif_has_frames(path: &Path) -> Option<bool> {
    use image::AnimationDecoder;
    let f = std::io::BufReader::new(std::fs::File::open(path).ok()?);
    let dec = image::codecs::gif::GifDecoder::new(f).ok()?;
    Some(dec.into_frames().take(2).filter(|r| r.is_ok()).count() > 1)
}

fn webp_is_animated(path: &Path) -> Option<bool> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    let mut head = [0u8; 12];
    f.read_exact(&mut head).ok()?;
    if &head[0..4] != b"RIFF" || &head[8..12] != b"WEBP" {
        return Some(false);
    }
    for _ in 0..32 {
        let mut ch = [0u8; 8];
        if f.read_exact(&mut ch).is_err() {
            break;
        }
        let len = u32::from_le_bytes([ch[4], ch[5], ch[6], ch[7]]) as i64;
        match &ch[0..4] {
            b"ANIM" | b"ANMF" => return Some(true),
            b"VP8X" if len >= 1 => {
                let mut flags = [0u8; 1];
                f.read_exact(&mut flags).ok()?;
                if flags[0] & 0x02 != 0 {
                    return Some(true);
                }
                let pad = len + (len & 1) - 1;
                if f.seek(SeekFrom::Current(pad)).is_err() {
                    break;
                }
                continue;
            }
            b"VP8 " | b"VP8L" => return Some(false),
            _ => {}
        }
        let skip = len + (len & 1);
        if f.seek(SeekFrom::Current(skip)).is_err() {
            break;
        }
    }
    Some(false)
}

fn png_is_animated(path: &Path) -> Option<bool> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    let mut sig = [0u8; 8];
    f.read_exact(&mut sig).ok()?;
    // the animation chunk has to come before the first image data
    loop {
        let mut head = [0u8; 8];
        f.read_exact(&mut head).ok()?;
        match &head[4..8] {
            b"acTL" => return Some(true),
            b"IDAT" | b"IEND" => return Some(false),
            _ => {}
        }
        let len = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as i64;
        f.seek(SeekFrom::Current(len + 4)).ok()?;
    }
}
