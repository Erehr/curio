//! What every build needs about video and sound: which files they are, the thumbnail rules, saved
//! frames, subtitles and page captures. FFmpeg reads the media (`ffmpeg.rs`): the one that comes with Curio, or one of the person's own.
//! Video thumbnails:
//! * frame captured at 1.5 s, or a quarter of the way in when the clip is shorter than that
//! * at most 512 px wide

pub mod ffmpeg;

use std::path::{Path, PathBuf};

/// A thumbnail that waits for FFmpeg (thumb_state 4): made once FFmpeg is found again.
pub const NEEDS_FFMPEG: &str = "waiting for FFmpeg";

/// Whether video and sound can be read: FFmpeg's two programs are found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    Ffmpeg,
    Off,
}

pub fn engine() -> Engine {
    if crate::core::media::ffmpeg::ffmpeg().is_some() && crate::core::media::ffmpeg::ffprobe().is_some() {
        Engine::Ffmpeg
    } else {
        Engine::Off
    }
}

/// When the frame is taken …
pub const VIDEO_CAPTURE_AT: f64 = 1.5;
/// … or this share of a clip too short for that
pub const VIDEO_SHORT_FACTOR: f64 = 0.25;

/// Formats treated as video.
pub const VIDEO_EXTS: &[&str] = &[
    "3gp", "avi", "av1", "f4v", "flv", "hevc", "m4v", "mkv", "mov", "mp4", "mpeg", "mpg", "ogv", "rm", "rmvb", "ts", "vp8", "vp9", "webm",
    "wmv", "mts", "m2ts",
];

/// What a saved frame is called: the video's name, then the time to two places.
pub fn frame_file_name(stem: &str, time: f64) -> String {
    format!("{stem} frame {:.2}s.jpg", time.max(0.0))
}

const SUBTITLE_EXTS: [&str; 2] = ["vtt", "srt"];
const SUBTITLE_MAX_BYTES: u64 = 2 * 1024 * 1024;

fn read_subtitle(path: &Path) -> Result<String, String> {
    let md = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if md.len() > SUBTITLE_MAX_BYTES {
        return Err("the subtitle file is too large".into());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Subtitles lying next to a video under its name: .vtt first, then .srt.
pub fn sibling_subtitles(video: &Path) -> Vec<(String, String)> {
    SUBTITLE_EXTS
        .iter()
        .filter_map(|ext| {
            let path = video.with_extension(ext);
            read_subtitle(&path).ok().map(|text| (file_name(&path), text))
        })
        .collect()
}

/// Puts a chosen subtitle file next to the video under the video's name, so it comes back next time.
/// A different file already there is kept: then only the chosen text is returned, with `false`.
/// With `save` off (a read-only location) nothing is written and only the text comes back.
pub fn adopt_subtitle(video: &Path, picked: &Path, save: bool) -> Result<(String, String, bool), String> {
    let ext = picked.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !SUBTITLE_EXTS.contains(&ext.as_str()) {
        return Err("choose a .srt or .vtt file".into());
    }
    let text = read_subtitle(picked)?;
    let dest = video.with_extension(&ext);
    if dest.exists() {
        let same = read_subtitle(&dest).map(|t| t == text).unwrap_or(false);
        return Ok(if same { (file_name(&dest), text, true) } else { (file_name(picked), text, false) });
    }
    if !save {
        return Ok((file_name(picked), text, false));
    }
    Ok(match std::fs::copy(picked, &dest) {
        Ok(_) => (file_name(&dest), text, true),
        Err(_) => (file_name(picked), text, false),
    })
}

// ------------------------------------------------------------------ website thumbnails

/// A browser able to draw a page without a window (Edge ships with Windows).
fn page_browser() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("CURIO_BROWSER") {
        return Some(PathBuf::from(p));
    }
    #[cfg(windows)]
    for base in ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"] {
        if let Some(b) = std::env::var_os(base) {
            for rel in ["Microsoft/Edge/Application/msedge.exe", "Google/Chrome/Application/chrome.exe"] {
                let p = PathBuf::from(&b).join(rel);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    ["microsoft-edge", "google-chrome", "chromium", "chromium-browser"].iter().find_map(|n| crate::core::system::filesystem::which(n))
}

type PageCapturer = Box<dyn Fn(&str, &Path) -> Result<(), String> + Send + Sync>;

static PAGE_CAPTURER: std::sync::OnceLock<PageCapturer> = std::sync::OnceLock::new();

/// The app's own way to draw a page, in its web view, tried before a headless browser.
pub fn set_page_capturer(f: PageCapturer) {
    let _ = PAGE_CAPTURER.set(f);
}

/// Draw a web page to a PNG. Fails when no browser is available;
/// the caller then falls back to the page's own preview image.
pub fn capture_page(url: &str, out: &Path) -> Result<(), String> {
    use crate::core::consts::{PAGE_CAPTURE_SETTLED_SECS as SETTLED, PAGE_CAPTURE_TIMEOUT_SECS as T};
    // only web addresses: anything else could be read as a browser option
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("only web links can be captured".into());
    }
    if let Some(own) = PAGE_CAPTURER.get() {
        match own(url, out) {
            Ok(()) => return Ok(()),
            Err(e) => crate::core::system::logging::warn(&format!("page capture of {url} in Curio's web view: {e}")),
        }
    }
    let browser = page_browser().ok_or("no browser found for page capture")?;
    // the page as its scripts draw it, given a few seconds of quiet; a page that keeps loading never gets them, and is taken as it stands after a fixed time instead
    capture_with(&browser, url, out, "--virtual-time-budget=6000", SETTLED).or_else(|_| capture_with(&browser, url, out, "--timeout=8000", T))
}

fn capture_with(browser: &Path, url: &str, out: &Path, wait: &str, secs: u64) -> Result<(), String> {
    use crate::core::consts::{PAGE_CAPTURE_HEIGHT as H, PAGE_CAPTURE_WIDTH as W};
    let _ = std::fs::remove_file(out);
    let profile = out.with_extension("profile");
    // the browser fetches through Curio, which goes out to public addresses only; QUIC and WebRTC would go around a proxy
    let proxy = crate::core::system::net::PublicProxy::start().map_err(|e| e.to_string())?;
    let args = vec![
        "--headless=new".to_string(),
        format!("--proxy-server=http://127.0.0.1:{}", proxy.port),
        "--proxy-bypass-list=<-loopback>".into(),
        "--disable-quic".into(),
        "--force-webrtc-ip-handling-policy=disable_non_proxied_udp".into(),
        "--disable-gpu".into(),
        "--hide-scrollbars".into(),
        "--mute-audio".into(),
        "--no-first-run".into(),
        "--disable-extensions".into(),
        wait.into(),
        format!("--user-data-dir={}", profile.display()),
        format!("--window-size={W},{H}"),
        format!("--screenshot={}", out.display()),
        url.to_string(),
    ];
    let r = crate::core::system::process::run(&browser.to_string_lossy(), &args, None, std::time::Duration::from_secs(secs));
    let _ = std::fs::remove_dir_all(&profile);
    let r = r?;
    if out.exists() {
        Ok(())
    } else {
        let said = r.stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or_default();
        Err(format!("the browser wrote no picture (exit {:?}) {said}", r.status))
    }
}
