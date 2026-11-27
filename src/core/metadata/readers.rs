//! Everything that reads fields from files: Curio's own readers and plugins' extractors, one entry each, with the extensions it reads and the keys it fills.

use std::path::Path;

use crate::core::metadata::extract::{self, AUDIO_OWNER, EXIF_OWNER, FONT_OWNER, VIDEO_OWNER};
use crate::core::model::RawValue;
use crate::core::plugins::manifest::ExtractorDecl;

pub type Values = Vec<(String, RawValue)>;

/// What one reader made of one file.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Read(Values),
    /// It can't read here (no ffprobe): nothing it fills is known to be absent.
    Skipped,
    Failed(String),
}

/// The file being read, as the library knows it.
pub struct FileInfo<'a> {
    pub ext: &'a str,
    pub size: i64,
    pub duration: Option<f64>,
    /// the files grouped into it that are there, in the order they were grouped
    pub parts: &'a [PartFile],
}

/// A file grouped into the item being read.
#[derive(Clone, Debug, serde::Serialize)]
pub struct PartFile {
    pub role: String,
    pub name: String,
    pub path: std::path::PathBuf,
}

/// The files grouped into an item that are there, for its readers.
pub fn parts_of(db: &crate::core::db::Db, id: crate::core::model::AssetId) -> Vec<PartFile> {
    let Ok(parts) = db.members_of(id) else { return vec![] };
    parts.into_iter().filter(|p| p.status == crate::core::model::Availability::Ok).filter_map(|p| db.asset(p.id).ok().flatten().map(|m| PartFile { role: p.role, name: p.name, path: m.abs_path.into() })).collect()
}

/// What the readers of one file share: the EXIF block two of them read, and the length the video reader measured.
#[derive(Default)]
pub struct Cache {
    exif: Option<Values>,
    pub duration: Option<f64>,
    /// A video's width and height as FFmpeg reads them, turned as it plays.
    pub dims: Option<(i64, i64)>,
}

impl Cache {
    fn exif(&mut self, path: &Path) -> &Values {
        self.exif.get_or_insert_with(|| extract::exif_fields(path))
    }
}

pub type ReadFn = fn(&Path, &FileInfo, &mut Cache) -> Outcome;

#[derive(Clone, Debug)]
pub enum Source {
    Builtin(ReadFn),
    /// A plugin's extractor, run for the plugin named by the reader's owner.
    Extractor(ExtractorDecl),
}

#[derive(Clone, Debug)]
pub struct Reader {
    /// `<owner>/<name>`, or `<plugin id>/x<n>` for a plugin's extractor
    pub id: String,
    pub owner: String,
    /// lower case, without dots; empty reads every file
    pub exts: Vec<String>,
    pub keys: Vec<String>,
    pub version: u32,
    pub source: Source,
    /// needs ffprobe, so it is not run where a file is read without the media programs
    pub ffprobe: bool,
}

/// What the database keeps of a reader, for the parts of Curio that see only the library: which fields are read from files, and which files a reader is queued for.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReaderInfo {
    pub id: String,
    pub owner: String,
    pub exts: Vec<String>,
    pub keys: Vec<String>,
    pub active: bool,
}

impl ReaderInfo {
    pub fn fills(&self, key: &str) -> bool {
        self.keys.iter().any(|k| k == key)
    }
}

/// How a chosen reader reads a file again: replacing what it read before and clearing what it no longer finds, or filling only what has no value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    All = 0,
    Missing = 1,
}

impl Mode {
    pub fn from_db(n: i64) -> Mode {
        if n == 1 { Mode::Missing } else { Mode::All }
    }
}

impl Reader {
    pub fn info(&self, active: bool) -> ReaderInfo {
        ReaderInfo { id: self.id.clone(), owner: self.owner.clone(), exts: self.exts.clone(), keys: self.keys.clone(), active }
    }

    pub fn applies(&self, ext: &str) -> bool {
        self.exts.is_empty() || self.exts.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }

    pub fn fills(&self, key: &str) -> bool {
        self.keys.iter().any(|k| k == key)
    }

    /// The page that reads files for this plugin's extractor, in the window rather than on the metadata thread.
    pub fn page(&self) -> Option<&str> {
        match &self.source {
            Source::Extractor(x) => x.page.as_deref(),
            Source::Builtin(_) => None,
        }
    }
}

const EXIF_KEYS: &[&str] = &["camera", "lens", "iso", "aperture", "shutter", "focal_length", "taken_at", "artist", "copyright", "gps_lat", "gps_lon", "gps_alt"];
const PICTURE_EXTS: &[&str] = &["png", "jpg", "jpeg", "jfif", "tif", "tiff", "webp", "heic", "heif", "avif", "dng", "nef", "cr2", "arw"];
const SONG_KEYS: &[&str] = &["artist", "album", "title", "album_artist", "composer", "track", "disc", "year", "genre"];
const SOUND_KEYS: &[&str] = &["audio_codec", "channels", "sample_rate", "bit_rate"];
const FONT_KEYS: &[&str] = &["font_family", "font_style", "font_weight", "font_width", "font_italic", "font_class", "font_scripts", "font_axes", "glyphs", "font_faces", "designer", "foundry", "font_version", "font_license"];
const VIDEO_KEYS: &[&str] = &["video_codec", "audio_codec", "frame_rate", "bit_rate", "channels", "sample_rate"];

// A reader's version goes up when what it reads from the same file changes; each library then reads that reader's files again once.
const EXIF_VERSION: u32 = 1;
const ADDRESS_VERSION: u32 = 1;
const SONG_VERSION: u32 = 1;
const SOUND_VERSION: u32 = 1;
const VIDEO_VERSION: u32 = 1;
const FONTS_VERSION: u32 = 1;

fn image_exif(path: &Path, _: &FileInfo, c: &mut Cache) -> Outcome {
    Outcome::Read(c.exif(path).clone())
}

fn address(path: &Path, _: &FileInfo, _: &mut Cache) -> Outcome {
    Outcome::Read(crate::core::thumbs::links::url_from_file(path).map(|u| ("__source_url".to_string(), RawValue::text(u))).into_iter().collect())
}

fn song(path: &Path, _: &FileInfo, _: &mut Cache) -> Outcome {
    Outcome::Read(crate::core::metadata::audio_tags::fields(path))
}

fn sound(path: &Path, _: &FileInfo, _: &mut Cache) -> Outcome {
    Outcome::Read(crate::core::metadata::audio_tags::encoding(path))
}

fn video(path: &Path, f: &FileInfo, c: &mut Cache) -> Outcome {
    if crate::core::media::engine() != crate::core::media::Engine::Ffmpeg {
        return Outcome::Skipped;
    }
    match crate::core::media::ffmpeg::probe(path) {
        Ok(p) => {
            let d = f.duration.or(p.duration);
            if f.duration.is_none() {
                c.duration = d;
            }
            c.dims = p.width.zip(p.height);
            Outcome::Read(extract::encoding_fields(p.video_codec.as_deref(), p.audio_codec.as_deref(), p.fps, p.channels, p.sample_rate, f.size, d))
        }
        Err(e) => Outcome::Failed(e),
    }
}

/// A font file's first face.
fn font(path: &Path, _: &FileInfo, _: &mut Cache) -> Outcome {
    use crate::core::metadata::fonts::{facts, source::FileSource};
    match FileSource::open(path).and_then(|f| facts::read(f, 0, false)) {
        Ok(facts) => Outcome::Read(facts.fields().iter().map(|(k, v)| (k.clone(), crate::core::plugins::run::json_to_raw(v))).collect()),
        Err(e) => Outcome::Failed(e),
    }
}

fn builtin_reader(id: &str, owner: &str, exts: &[&str], keys: &[&str], version: u32, read: ReadFn) -> Reader {
    Reader { id: format!("{owner}/{id}"), owner: owner.into(), exts: exts.iter().map(|e| e.to_string()).collect(), keys: keys.iter().map(|k| k.to_string()).collect(), version, source: Source::Builtin(read), ffprobe: false }
}

/// Curio's own readers, in the order their values win when two fill the same key.
pub fn builtin() -> Vec<Reader> {
    use crate::core::formats::{audio, font as font_family, video as video_family};
    let mut v = vec![
        builtin_reader("address", "builtin.format.document", &["url", "webloc"], &[], ADDRESS_VERSION, address),
        builtin_reader("exif", EXIF_OWNER, PICTURE_EXTS, EXIF_KEYS, EXIF_VERSION, image_exif),
        builtin_reader("fonts", FONT_OWNER, font_family::FAMILY.exts, FONT_KEYS, FONTS_VERSION, font),
        builtin_reader("tags", AUDIO_OWNER, audio::FAMILY.exts, SONG_KEYS, SONG_VERSION, song),
        builtin_reader("encoding", AUDIO_OWNER, audio::FAMILY.exts, SOUND_KEYS, SOUND_VERSION, sound),
        builtin_reader("encoding", VIDEO_OWNER, video_family::FAMILY.exts, VIDEO_KEYS, VIDEO_VERSION, video),
    ];
    v.last_mut().unwrap().ffprobe = true;
    v
}

/// One reader for each extractor a plugin declares. A program's keys are the `fields` it lists; without them it fills none that could be cleared.
pub fn extractor_readers(plugin: &str, decls: &[ExtractorDecl]) -> Vec<Reader> {
    decls
        .iter()
        .enumerate()
        .map(|(n, x)| {
            let mut keys: Vec<String> = x.png_text.values().cloned().chain(x.regex.iter().map(|r| r.field.clone())).chain(x.fields.iter().cloned()).chain(x.json.iter().flat_map(|j| j.fields.keys().cloned())).collect();
            keys.sort();
            keys.dedup();
            let mut exts: Vec<String> = x.exts.iter().map(|e| e.trim_start_matches('.').to_lowercase()).collect();
            if exts.is_empty() && x.command.is_empty() && x.json.is_none() && x.page.is_none() {
                exts.push("png".into());
            }
            let name = x.id.clone().unwrap_or_else(|| format!("x{}", n + 1));
            Reader { id: format!("{plugin}/{name}"), owner: plugin.into(), exts, keys, version: x.version, source: Source::Extractor(x.clone()), ffprobe: false }
        })
        .collect()
}

/// Keys a reader fills into the item itself, its source, notes and shown name, when they are empty.
pub const ITEM_KEYS: [&str; 3] = ["__source_url", "__annotation", "__title"];

/// Runs one reader on one file; a plugin's values come as JSON and are turned into raw values here.
pub fn run(r: &Reader, path: &Path, f: &FileInfo, c: &mut Cache) -> Outcome {
    // a page's reader is read in the window, and the metadata thread applies its answer
    if !r.applies(f.ext) || r.page().is_some() {
        return Outcome::Skipped;
    }
    match &r.source {
        Source::Builtin(read) => match std::fs::File::open(path) {
            Ok(_) => match read(path, f, c) {
                Outcome::Read(v) => Outcome::Read(v.into_iter().map(|(k, v)| (k, capped(v))).collect()),
                o => o,
            },
            Err(e) => Outcome::Failed(e.to_string()),
        },
        Source::Extractor(x) => match crate::core::plugins::run::run_extractor(&r.owner, x, path, f.ext, f.parts) {
            Ok(v) => Outcome::Read(v.into_iter().map(|(k, j)| (k, crate::core::plugins::run::json_to_raw(&j))).collect()),
            Err(e) => Outcome::Failed(e),
        },
    }
}

/// The longest text Curio's own readers keep, in bytes: multi-KB values in `field_values` (`WITHOUT ROWID` + `idx_fv_text`) slow every field filter sharply. Plugins' readers are left to their authors.
pub const MAX_TEXT: usize = 16 * 1024;

fn capped(v: RawValue) -> RawValue {
    match v {
        RawValue::Text(mut s) if s.len() > MAX_TEXT => {
            let mut end = MAX_TEXT;
            while !s.is_char_boundary(end) {
                end -= 1;
            }
            s.truncate(end);
            RawValue::Text(s)
        }
        RawValue::List(l) => RawValue::List(l.into_iter().map(capped).collect()),
        o => o,
    }
}

/// One reader's part in reading a file: `mode` is set for the readers the read is for, and left out for those run beside them because they fill a key of theirs.
pub struct Ran<'a> {
    pub reader: &'a Reader,
    pub outcome: Outcome,
    pub mode: Option<Mode>,
}

/// What a read writes, before the library's own rules (fields not indexed, values set by hand) apply.
#[derive(Debug, Default, PartialEq)]
pub struct Plan {
    pub values: Values,
    /// keys written only where the item has no value
    pub only_empty: std::collections::HashSet<String>,
    /// keys a reader that read the file fills, which no reader found
    pub clear: Vec<String>,
}

/// The values of every reader that ran, the first reader's winning a key two fill; of those, the keys of the readers the read is for, and whatever they returned. A key is cleared when a reader reading everything fills it, read the file, and no reader returned it or failed to say.
pub fn plan(runs: &[Ran]) -> Plan {
    use std::collections::HashSet;
    let returned = |k: &str, r: &Ran| matches!(&r.outcome, Outcome::Read(v) if v.iter().any(|(x, _)| x == k));
    let primary: Vec<&Ran> = runs.iter().filter(|r| r.mode.is_some()).collect();
    let in_scope = |k: &str| primary.iter().any(|r| r.reader.fills(k) || returned(k, r));
    let all_for = |k: &str| primary.iter().any(|r| r.mode == Some(Mode::All) && (r.reader.fills(k) || returned(k, r)));
    let values: Values = merge(runs.iter().filter_map(|r| match &r.outcome {
        Outcome::Read(v) => Some(v.clone()),
        _ => None,
    }))
    .into_iter()
    .filter(|(k, _)| in_scope(k))
    .collect();
    let only_empty: HashSet<String> = values.iter().filter(|(k, _)| !all_for(k)).map(|(k, _)| k.clone()).collect();
    let mut clear = Vec::new();
    let unsure = |k: &str| runs.iter().any(|r| r.reader.fills(k) && !matches!(r.outcome, Outcome::Read(_)));
    for r in primary.iter().filter(|r| r.mode == Some(Mode::All) && matches!(r.outcome, Outcome::Read(_))) {
        for k in &r.reader.keys {
            if !clear.contains(k) && !runs.iter().any(|x| returned(k, x)) && !unsure(k) {
                clear.push(k.clone());
            }
        }
    }
    Plan { values, only_empty, clear }
}

/// Values in reader order, the first reader's winning a key two fill.
pub fn merge(outcomes: impl IntoIterator<Item = Values>) -> Values {
    let mut seen = std::collections::HashSet::new();
    let mut out: Values = outcomes.into_iter().flatten().collect();
    out.retain(|(k, _)| seen.insert(k.clone()));
    out
}
