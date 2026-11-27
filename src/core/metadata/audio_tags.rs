//! Tags and cover art of sound files through lofty, for every format it reads (MP3, FLAC, Ogg, Opus, M4A, WAV, AIFF, APE, WavPack …), whichever media engine is on.

use std::path::Path;

use lofty::prelude::*;

use crate::core::model::RawValue;

fn read(path: &Path) -> Option<lofty::file::TaggedFile> {
    lofty::probe::Probe::open(path).ok()?.options(lofty::config::ParseOptions::new().read_cover_art(true)).read().ok()
}

/// Artist, album, album artist, title, track, disc, year, genre and composer as field values.
pub fn fields(path: &Path) -> Vec<(String, RawValue)> {
    let mut out = Vec::new();
    let Some(f) = read(path) else { return out };
    let Some(tag) = f.primary_tag().or_else(|| f.first_tag()) else { return out };
    let mut text = |key: &str, v: Option<std::borrow::Cow<str>>| {
        if let Some(v) = v.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) {
            out.push((key.to_string(), RawValue::text(v)));
        }
    };
    text("artist", tag.artist());
    text("album", tag.album());
    text("title", tag.title());
    text("album_artist", tag.get_string(ItemKey::AlbumArtist).map(Into::into));
    text("composer", tag.get_string(ItemKey::Composer).map(Into::into));
    if let Some(t) = tag.track() {
        out.push(("track".into(), RawValue::Num(t as f64)));
    }
    if let Some(d) = tag.disk() {
        out.push(("disc".into(), RawValue::Num(d as f64)));
    }
    if let Some(y) = tag.date().map(|d| d.year).filter(|y| *y > 0) {
        out.push(("year".into(), RawValue::Num(y as f64)));
    }
    // ID3 writes several genres as one text with slashes or semicolons, sometimes "(17)" codes before them
    if let Some(g) = tag.genre() {
        let list: Vec<RawValue> = g.split(['/', ';', '\0']).map(|s| s.trim().trim_start_matches(|c: char| c == '(' || c.is_ascii_digit() || c == ')').trim()).filter(|s| !s.is_empty()).map(RawValue::text).collect();
        if !list.is_empty() {
            out.push(("genre".into(), RawValue::List(list)));
        }
    }
    out
}

/// How a sound file is encoded: its codec, bit rate, channels and sample rate.
pub fn encoding(path: &Path) -> Vec<(String, RawValue)> {
    let Some(f) = read(path) else { return Vec::new() };
    let p = f.properties();
    use lofty::file::FileType::*;
    let codec = match f.file_type() {
        Mpeg => Some("mp3"),
        Flac => Some("flac"),
        Opus => Some("opus"),
        Vorbis => Some("vorbis"),
        Speex => Some("speex"),
        Wav | Aiff => Some("pcm"),
        Ape => Some("ape"),
        WavPack => Some("wavpack"),
        Aac => Some("aac"),
        Mpc => Some("musepack"),
        Mp4 => mp4_codec(path),
        _ => None,
    };
    let size = std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0);
    let d = p.duration().as_secs_f64();
    let mut out = crate::core::metadata::extract::encoding_fields(None, codec, None, p.channels().map(i64::from), p.sample_rate().map(i64::from), size, (d > 0.0).then_some(d));
    // the tags' own bit rate, where it says one, is the sound's alone
    if let Some(b) = p.overall_bitrate().filter(|b| *b > 0) {
        out.retain(|(k, _)| k != "bit_rate");
        out.push(("bit_rate".into(), RawValue::Num(b as f64)));
    }
    out
}

/// An M4A holds AAC or Apple's lossless codec, which only its own properties tell apart.
fn mp4_codec(path: &Path) -> Option<&'static str> {
    use lofty::mp4::{Mp4Codec, Mp4File};
    let mut file = std::fs::File::open(path).ok()?;
    let m = <Mp4File as lofty::file::AudioFile>::read_from(&mut file, lofty::config::ParseOptions::new().read_tags(false)).ok()?;
    Some(match m.properties().codec()? {
        Mp4Codec::AAC => "aac",
        Mp4Codec::ALAC => "alac",
        Mp4Codec::MP3 => "mp3",
        Mp4Codec::FLAC => "flac",
        _ => return None,
    })
}

/// The length in seconds.
pub fn duration(path: &Path) -> Option<f64> {
    let d = read(path)?.properties().duration().as_secs_f64();
    (d > 0.0).then_some(d)
}

/// The front cover stored in the file, else any picture it stores, else `cover`, `folder`, `front` or `albumart` beside it.
pub fn cover(path: &Path) -> Option<image::DynamicImage> {
    if let Some(f) = read(path) {
        let pics: Vec<&lofty::picture::Picture> = f.tags().iter().flat_map(|t| t.pictures()).collect();
        let front = pics.iter().find(|p| p.pic_type() == lofty::picture::PictureType::CoverFront).or(pics.first());
        if let Some(img) = front.and_then(|p| image::load_from_memory(p.data()).ok()) {
            return Some(img);
        }
    }
    let dir = path.parent()?;
    let names = ["cover", "folder", "front", "albumart"];
    let rd = std::fs::read_dir(dir).ok()?;
    let mut beside: Vec<std::path::PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| {
        let stem = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        let ext = p.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        names.contains(&stem.as_str()) && matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp")
    }).collect();
    beside.sort_by_key(|p| names.iter().position(|n| p.file_stem().is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(n))));
    beside.iter().find_map(|p| image::open(p).ok())
}
