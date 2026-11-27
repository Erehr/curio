//! Reads video and sound with the ffmpeg and ffprobe programs: the FFmpeg that comes with Curio, or one of the person's own; docs/features/viewer/media.md says where it is looked for. Plugins get the same one.
//! Thumbnail rules are shared with the Mediabunny plugin, see `media.rs`:
//! * probe with `ffprobe -v error -print_format json -show_format -show_streams`
//! * rotation from the stream's side_data_list (else tags.rotate); -90 → 270; 90/270 swap w/h
//! * frame captured at 1.5 s, or at duration × 0.25 when the clip is shorter than 1.5 s

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::core::media::{VIDEO_CAPTURE_AT, VIDEO_SHORT_FACTOR};
use crate::core::system::process;

fn exe_name(n: &str) -> String {
    if cfg!(windows) {
        format!("{n}.exe")
    } else {
        n.to_string()
    }
}

/// The folder for an FFmpeg of the person's own, shown in Settings with a button to open it.
pub fn plugin_folder() -> Option<PathBuf> {
    Some(crate::core::system::app_dirs::data_dir().join("plugins").join("ffmpeg"))
}

/// An FFmpeg the person put in place: $CURIO_FFMPEG_DIR, then the folder Settings shows.
fn own_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(d) = std::env::var("CURIO_FFMPEG_DIR") {
        dirs.push(PathBuf::from(d));
    }
    dirs.extend(plugin_folder());
    dirs
}

/// The FFmpeg that comes with Curio: next to the program (`resources/ffmpeg/`, in an .app its `Resources`), or the source tree's in development builds.
fn bundled_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())) {
        dirs.push(exe_dir.join("resources").join("ffmpeg"));
        dirs.push(exe_dir.join("ffmpeg"));
        // an .app keeps its resources in Contents/Resources, beside Contents/MacOS
        #[cfg(target_os = "macos")]
        dirs.push(exe_dir.join("..").join("Resources").join("resources").join("ffmpeg"));
    }
    #[cfg(debug_assertions)]
    // dev/ sits beside the app's source
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("dev").join("ffmpeg"));
    dirs
}

fn in_dirs(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    dirs.iter().map(|d| d.join(exe_name(name))).find(|p| p.is_file())
}

/// `ffmpeg` or `ffprobe`, for Curio and for plugins alike: the one chosen in Settings › Tools, one of the person's own, the one that comes with Curio, the PATH, then the usual install folders. Looked up each time, so programs put in place are seen without restarting.
pub fn find(name: &str) -> Option<PathBuf> {
    crate::core::plugins::tools::overrides().get(name).filter(|p| p.is_file()).cloned().or_else(|| in_dirs(&own_dirs(), name)).or_else(|| in_dirs(&bundled_dirs(), name)).or_else(|| crate::core::system::filesystem::which(name)).or_else(|| in_dirs(&crate::core::plugins::tools::usual_dirs(), name))
}

pub fn ffmpeg() -> Option<PathBuf> {
    find("ffmpeg")
}

pub fn ffprobe() -> Option<PathBuf> {
    find("ffprobe")
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Probe {
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration: Option<f64>,
    pub rotation: i64,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub container: Option<String>,
    pub fps: Option<f64>,
    pub bitrate: Option<i64>,
    pub channels: Option<i64>,
    pub sample_rate: Option<i64>,
}

pub fn probe(src: &Path) -> Result<Probe, String> {
    let fp = ffprobe().ok_or("ffprobe is not available")?;
    let args: Vec<String> =
        ["-v", "error", "-print_format", "json", "-show_format", "-show_streams"].iter().map(|s| s.to_string()).chain([src.to_string_lossy().to_string()]).collect();
    let out = process::run(&fp.to_string_lossy(), &args, None, Duration::from_secs(30))?;
    if out.status != Some(0) {
        return Err(format!("ffprobe failed: {}", out.stderr.trim()));
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    let streams = v["streams"].as_array().cloned().unwrap_or_default();
    let video = streams.iter().find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"].as_i64() != Some(1));
    let audio = streams.iter().find(|s| s["codec_type"] == "audio");
    let mut p = Probe {
        duration: v["format"]["duration"].as_str().and_then(|d| d.parse().ok()),
        container: v["format"]["format_name"].as_str().map(String::from),
        bitrate: v["format"]["bit_rate"].as_str().and_then(|d| d.parse().ok()),
        audio_codec: audio.and_then(|a| a["codec_name"].as_str()).map(String::from),
        channels: audio.and_then(|a| a["channels"].as_i64()),
        sample_rate: audio.and_then(|a| a["sample_rate"].as_str()).and_then(|r| r.parse().ok()),
        ..Default::default()
    };
    if let Some(vs) = video {
        p.video_codec = vs["codec_name"].as_str().map(String::from);
        p.width = vs["width"].as_i64();
        p.height = vs["height"].as_i64();
        p.fps = vs["avg_frame_rate"].as_str().and_then(|r| {
            let (a, b) = r.split_once('/')?;
            let (a, b): (f64, f64) = (a.parse().ok()?, b.parse().ok()?);
            (b > 0.0).then(|| a / b)
        });
        let mut rot = vs["side_data_list"].as_array().and_then(|l| l.iter().find_map(|d| d["rotation"].as_i64())).unwrap_or(0);
        if rot == 0 {
            rot = vs["tags"]["rotate"].as_str().and_then(|r| r.parse().ok()).unwrap_or(0);
        }
        if rot == -90 {
            rot = 270;
        }
        p.rotation = rot;
        if rot.abs() == 90 || rot == 270 {
            std::mem::swap(&mut p.width, &mut p.height);
        }
        if p.duration.is_none() {
            p.duration = vs["duration"].as_str().and_then(|d| d.parse().ok());
        }
    }
    Ok(p)
}

fn hhmmss(sec: f64) -> String {
    let ms = (sec.fract() * 1000.0).floor() as u64;
    let s = sec.floor() as u64;
    format!("{:02}:{:02}:{:02}.{:03}", s / 3600, (s % 3600) / 60, s % 60, ms)
}

/// Video frame → PNG at `out`, scaled to at most `max` px wide. Returns the probe.
pub fn video_frame(src: &Path, out: &Path, max: u32, at: Option<f64>) -> Result<Probe, String> {
    let ff = ffmpeg().ok_or("ffmpeg is not available")?;
    let pr = probe(src).unwrap_or_default();
    let dur = pr.duration.unwrap_or(0.0);
    let mut t = if dur > 0.0 && dur < VIDEO_CAPTURE_AT { dur * VIDEO_SHORT_FACTOR } else { VIDEO_CAPTURE_AT };
    if let Some(a) = at {
        if dur == 0.0 || a <= dur {
            t = a;
        }
    }
    let run = |seek: Option<f64>| -> Result<(), String> {
        let mut args: Vec<String> = vec!["-y".into(), "-v".into(), "error".into()];
        if let Some(s) = seek {
            args.extend(["-ss".into(), hhmmss(s)]);
        }
        args.extend([
            "-i".into(),
            src.to_string_lossy().to_string(),
            "-frames:v".into(),
            "1".into(),
            "-vf".into(),
            format!("scale='min({max},iw)':-2"),
            out.to_string_lossy().to_string(),
        ]);
        let o = process::run(&ff.to_string_lossy(), &args, None, Duration::from_secs(60))?;
        if o.status == Some(0) { Ok(()) } else { Err(o.stderr.trim().lines().last().unwrap_or("ffmpeg failed").to_string()) }
    };
    let mut why = run(Some(t)).err();
    if !out.exists() {
        // very short or odd files: take the first frame
        why = run(None).err().or(why);
    }
    if out.exists() {
        Ok(pr)
    } else {
        Err(why.unwrap_or_else(|| "ffmpeg could not extract a frame".into()))
    }
}

/// Decode any still image ffmpeg understands (AVIF, JPEG XL, HEIC tiles, PSD, EXR …) to PNG.
pub fn image_to_png(src: &Path, out: &Path, max: u32) -> Result<(), String> {
    let ff = ffmpeg().ok_or("ffmpeg is not available")?;
    let args: Vec<String> = vec![
        "-y".into(),
        "-v".into(),
        "error".into(),
        "-i".into(),
        src.to_string_lossy().to_string(),
        "-frames:v".into(),
        "1".into(),
        "-vf".into(),
        format!("scale='min({max},iw)':-2"),
        out.to_string_lossy().to_string(),
    ];
    process::run(&ff.to_string_lossy(), &args, None, Duration::from_secs(60))?;
    if out.exists() {
        Ok(())
    } else {
        Err("ffmpeg could not decode the image".into())
    }
}

/// Containers and codecs the window's player plays directly, as it said it reads them (`set_player_codecs`).
pub fn natively_playable(ext: &str, p: &Probe) -> bool {
    // MP4 and its kin (mov, which Chromium reads as MP4), MP3, WAV, FLAC and AAC play everywhere; WebM and Ogg only where the player reads their codecs
    let container_ok = match ext {
        "mp4" | "m4v" | "3gp" | "mov" | "mp3" | "wav" | "m4a" | "flac" | "aac" => true,
        "webm" => plays("VP9") || plays("VP8"),
        "ogv" | "ogm" | "ogg" | "oga" | "opus" => plays("Theora") || plays("Opus") || plays("Vorbis"),
        _ => false,
    };
    let ok = |c: Option<&str>| c.is_none_or(|c| player_name(c).is_some_and(plays));
    container_ok && ok(p.video_codec.as_deref()) && ok(p.audio_codec.as_deref())
}

/// The player's name for a codec FFmpeg names, as `mediaEngine.js` asks the player about it.
fn player_name(codec: &str) -> Option<&'static str> {
    Some(match codec {
        "h264" => "H.264",
        "hevc" => "H.265",
        "av1" => "AV1",
        "vp9" => "VP9",
        "vp8" => "VP8",
        "theora" => "Theora",
        "aac" => "AAC",
        "mp3" => "MP3",
        "opus" => "Opus",
        "vorbis" => "Vorbis",
        "flac" => "FLAC",
        "pcm_s16le" | "pcm_s24le" | "pcm_f32le" => "WAV",
        _ => return None,
    })
}

/// What the window's player says it reads: Chromium's until it says (WebView2's on Windows); a Mac's WebKit reads AV1 only with the hardware for it, and Ogg Theora not at all.
static PLAYER: std::sync::LazyLock<parking_lot::RwLock<Vec<String>>> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(["H.264", "AV1", "VP9", "VP8", "Theora", "AAC", "MP3", "Opus", "Vorbis", "FLAC", "WAV"].map(String::from).to_vec()));

fn plays(name: &str) -> bool {
    PLAYER.read().iter().any(|p| p == name)
}

pub fn set_player_codecs(plays: Vec<String>) {
    *PLAYER.write() = plays;
}

/// Make a web-playable copy in the cache: remux when the codecs are fine (fast), otherwise transcode to H.264/AAC. Returns the cached file.
pub fn playable_copy(src: &Path, cache: &Path, key: &str) -> Result<PathBuf, String> {
    let ff = ffmpeg().ok_or("ffmpeg is not available - install it or put ffmpeg next to the program")?;
    std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    let out = cache.join(format!("{key}.mp4"));
    if out.exists() {
        return Ok(out);
    }
    // a second request for the same video waits for the first conversion instead of starting its own
    let lock = TRANSCODES.lock().entry(cache.join(key)).or_default().clone();
    let held = lock.lock();
    let r = if out.exists() { Ok(out) } else { transcode(&ff, src, cache, key, out) };
    drop(held);
    TRANSCODES.lock().retain(|_, l| !std::sync::Arc::ptr_eq(l, &lock) || std::sync::Arc::strong_count(l) > 2);
    r
}

type KeyLocks = std::collections::HashMap<PathBuf, std::sync::Arc<parking_lot::Mutex<()>>>;
static TRANSCODES: std::sync::LazyLock<parking_lot::Mutex<KeyLocks>> = std::sync::LazyLock::new(Default::default);

/// H.264 encoders for playable copies, the computer's own first: the GPU's through the system, then x264.
fn h264_encoders() -> Vec<(&'static str, &'static [&'static str])> {
    let mut v: Vec<(&'static str, &'static [&'static str])> = Vec::new();
    if cfg!(windows) {
        v.push(("h264_mf", &["-hw_encoding", "1", "-rate_control", "quality", "-quality", "70", "-pix_fmt", "nv12"]));
    }
    if cfg!(target_os = "macos") {
        v.push(("h264_videotoolbox", &["-q:v", "65", "-pix_fmt", "yuv420p"]));
    }
    v.push(("libx264", &["-preset", "veryfast", "-crf", "22", "-pix_fmt", "yuv420p"]));
    v
}

fn transcode(ff: &Path, src: &Path, cache: &Path, key: &str, out: PathBuf) -> Result<PathBuf, String> {
    let p = probe(src).unwrap_or_default();
    let audio_only = p.video_codec.is_none();
    let copy_video = matches!(p.video_codec.as_deref(), Some("h264"));
    let copy_audio = matches!(p.audio_codec.as_deref(), Some("aac") | Some("mp3"));
    let audio: Vec<String> = if copy_audio { vec!["-c:a".into(), "copy".into()] } else { vec!["-c:a".into(), "aac".into(), "-b:a".into(), "192k".into()] };
    let videos: Vec<Vec<String>> = if audio_only {
        vec![vec!["-vn".into()]]
    } else if copy_video {
        vec![vec!["-c:v".into(), "copy".into()]]
    } else {
        h264_encoders().into_iter().map(|(name, opts)| ["-c:v", name].iter().chain(opts.iter()).map(|s| s.to_string()).collect()).collect()
    };
    let mut last = String::new();
    for video in videos {
        let tmp = cache.join(format!("{key}.{:016x}.part.mp4", fastrand::u64(..)));
        let mut args: Vec<String> = vec!["-y".into(), "-v".into(), "error".into(), "-i".into(), src.to_string_lossy().to_string()];
        if !audio_only {
            args.extend(["-map".into(), "0:v:0".into(), "-map".into(), "0:a:0?".into()]);
        }
        args.extend(video);
        args.extend(audio.iter().cloned());
        args.extend(["-movflags".into(), "+faststart".into(), tmp.to_string_lossy().to_string()]);
        match process::run(&ff.to_string_lossy(), &args, None, Duration::from_secs(3600)) {
            Ok(r) if r.status == Some(0) && tmp.exists() => {
                if let Err(e) = std::fs::rename(&tmp, &out) {
                    let _ = std::fs::remove_file(&tmp);
                    return Err(e.to_string());
                }
                return Ok(out);
            }
            Ok(r) => last = r.stderr.trim().to_string(),
            Err(e) => last = e,
        }
        let _ = std::fs::remove_file(&tmp);
    }
    Err(format!("ffmpeg could not convert the video: {last}"))
}

/// How many peaks a waveform keeps; thumbnails and the audio player share them.
pub const WAVE_POINTS: usize = 480;
/// Low sample rate: only the loudness is wanted, and measuring stays quick.
const WAVE_RATE: &str = "4000";

/// The picture attached to an audio file (its cover), written to `out`.
pub fn audio_cover(src: &Path, out: &Path) -> Result<(), String> {
    let ff = ffmpeg().ok_or("ffmpeg is not available")?;
    let args: Vec<String> = vec!["-y".into(), "-v".into(), "error".into(), "-i".into(), src.to_string_lossy().to_string(), "-map".into(), "0:v:0".into(), "-frames:v".into(), "1".into(), out.to_string_lossy().to_string()];
    let r = process::run(&ff.to_string_lossy(), &args, None, Duration::from_secs(20))?;
    if r.status == Some(0) && out.exists() {
        Ok(())
    } else {
        Err("no cover".into())
    }
}

/// Loudness peaks (0-255, loudest = 255) measured once and kept in the cache.
pub fn waveform(src: &Path, cache: &Path, key: &str) -> Result<Vec<u8>, String> {
    let out = cache.join(format!("{key}.wave"));
    if let Ok(b) = std::fs::read(&out) {
        if !b.is_empty() {
            return Ok(b);
        }
    }
    let ff = ffmpeg().ok_or("ffmpeg is not available")?;
    let args: Vec<String> = vec!["-v".into(), "error".into(), "-i".into(), src.to_string_lossy().to_string(), "-vn".into(), "-ac".into(), "1".into(), "-ar".into(), WAVE_RATE.into(), "-f".into(), "s16le".into(), "-".into()];
    let mut acc = Peaks::default();
    let r = process::run_streamed(&ff.to_string_lossy(), &args, Duration::from_secs(180), &mut |chunk| acc.feed(chunk))?;
    if acc.is_empty() {
        return Err(format!("could not read the sound: {}", r.stderr.trim()));
    }
    let peaks = acc.finish(WAVE_POINTS);
    std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    let _ = std::fs::write(&out, &peaks);
    Ok(peaks)
}

/// One frame at `at` seconds as `side` × `side` grey pixels, the picture squeezed to that square.
pub fn gray_frame(src: &Path, at: f64, side: u32) -> Result<Vec<u8>, String> {
    let ff = ffmpeg().ok_or("ffmpeg is not available")?;
    let args: Vec<String> = vec![
        "-v".into(),
        "error".into(),
        "-ss".into(),
        hhmmss(at.max(0.0)),
        "-i".into(),
        src.to_string_lossy().to_string(),
        "-frames:v".into(),
        "1".into(),
        "-an".into(),
        "-vf".into(),
        format!("scale={side}:{side}:flags=area,format=gray"),
        "-f".into(),
        "rawvideo".into(),
        "pipe:1".into(),
    ];
    let r = process::run(&ff.to_string_lossy(), &args, None, Duration::from_secs(60))?;
    if r.stdout.len() == (side * side) as usize {
        Ok(r.stdout)
    } else {
        Err(format!("ffmpeg could not read a frame: {}", r.stderr.trim()))
    }
}

/// The first `seconds` of a sound as mono 16-bit samples at `rate`.
pub fn pcm(src: &Path, rate: u32, seconds: u32) -> Result<Vec<i16>, String> {
    let ff = ffmpeg().ok_or("ffmpeg is not available")?;
    let args: Vec<String> = vec!["-v".into(), "error".into(), "-i".into(), src.to_string_lossy().to_string(), "-vn".into(), "-t".into(), seconds.to_string(), "-ac".into(), "1".into(), "-ar".into(), rate.to_string(), "-f".into(), "s16le".into(), "-".into()];
    let r = process::run(&ff.to_string_lossy(), &args, None, Duration::from_secs(120))?;
    if r.stdout.len() < 2 {
        return Err(format!("could not read the sound: {}", r.stderr.trim()));
    }
    Ok(r.stdout.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect())
}

/// Blocks kept while a sound streams in; past this, neighbours are merged and each block covers twice the samples.
const PEAK_BLOCKS: usize = 8192;

/// The loudest sample of each block of 16-bit audio, for a sound of unknown length read as it is decoded.
#[derive(Default)]
pub struct Peaks {
    blocks: Vec<u16>,
    per: usize,
    fill: usize,
    carry: Option<u8>,
}

impl Peaks {
    pub fn feed(&mut self, mut pcm: &[u8]) {
        if let Some(lo) = self.carry.take() {
            let Some((&hi, rest)) = pcm.split_first() else {
                self.carry = Some(lo);
                return;
            };
            self.sample(i16::from_le_bytes([lo, hi]));
            pcm = rest;
        }
        let mut it = pcm.chunks_exact(2);
        for b in &mut it {
            self.sample(i16::from_le_bytes([b[0], b[1]]));
        }
        self.carry = it.remainder().first().copied();
    }

    fn sample(&mut self, v: i16) {
        let v = v.unsigned_abs();
        match self.blocks.last_mut() {
            Some(last) if self.fill > 0 => *last = (*last).max(v),
            _ => self.blocks.push(v),
        }
        self.fill += 1;
        if self.fill >= self.per.max(1) {
            self.fill = 0;
            if self.blocks.len() >= PEAK_BLOCKS {
                self.blocks = self.blocks.chunks(2).map(|p| p.iter().copied().max().unwrap_or(0)).collect();
                self.per = self.per.max(1) * 2;
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn finish(&self, points: usize) -> Vec<u8> {
        let n = self.blocks.len();
        let points = points.max(1).min(n.max(1));
        let mut top = vec![0u32; points];
        for (i, v) in self.blocks.iter().enumerate() {
            let slot = i * points / n.max(1);
            top[slot] = top[slot].max(*v as u32);
        }
        let max = top.iter().copied().max().unwrap_or(0).max(1);
        top.into_iter().map(|v| (v * 255 / max) as u8).collect()
    }
}
