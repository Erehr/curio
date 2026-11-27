//! Programs plugins use (FFmpeg, yt-dlp …), found once by Curio rather than searched for by each plugin.
//! Curio never downloads them; a missing one shows in Settings › Plugins with where to get it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use serde_json::{json, Value};

pub struct Tool {
    pub name: &'static str,
    pub title: &'static str,
    pub about: &'static str,
    /// The program's name without `.exe`.
    pub exe: &'static str,
    pub version_args: &'static [&'static str],
    pub homepage: &'static str,
}

pub const TOOLS: &[Tool] = &[
    Tool { name: "ffmpeg", title: "FFmpeg", about: "converts and reads video and sound", exe: "ffmpeg", version_args: &["-version"], homepage: "https://ffmpeg.org/download.html" },
    Tool { name: "ffprobe", title: "FFprobe", about: "reads what video and sound files hold; comes with FFmpeg", exe: "ffprobe", version_args: &["-version"], homepage: "https://ffmpeg.org/download.html" },
    Tool { name: "yt-dlp", title: "yt-dlp", about: "downloads video from YouTube and many other sites", exe: "yt-dlp", version_args: &["--version"], homepage: "https://github.com/yt-dlp/yt-dlp#installation" },
    Tool { name: "gallery-dl", title: "gallery-dl", about: "downloads pictures from galleries and social sites", exe: "gallery-dl", version_args: &["--version"], homepage: "https://github.com/mikf/gallery-dl#installation" },
    Tool { name: "exiftool", title: "ExifTool", about: "reads and writes metadata in pictures and video", exe: "exiftool", version_args: &["-ver"], homepage: "https://exiftool.org" },
    Tool { name: "magick", title: "ImageMagick", about: "converts and edits pictures", exe: "magick", version_args: &["-version"], homepage: "https://imagemagick.org/script/download.php" },
    Tool { name: "7z", title: "7-Zip", about: "packs and unpacks archives", exe: "7z", version_args: &[], homepage: "https://www.7-zip.org/download.html" },
];

pub fn tool(name: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|t| t.name == name)
}

fn overrides_file() -> PathBuf {
    crate::core::system::app_dirs::data_dir().join("tools.json")
}

/// Paths the person chose in Settings, by tool name.
pub fn overrides() -> HashMap<String, PathBuf> {
    std::fs::read(overrides_file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Sets or, with `None`, forgets the path chosen for a tool.
pub fn set_path(name: &str, path: Option<&Path>) -> Result<(), String> {
    let t = tool(name).ok_or_else(|| format!("Curio doesn't know a tool called {name}"))?;
    let mut all = overrides();
    match path {
        Some(p) if !p.is_file() => return Err(format!("{} isn't a file", p.display())),
        Some(p) => {
            all.insert(t.name.into(), p.to_path_buf());
        }
        None => {
            all.remove(t.name);
        }
    }
    let file = overrides_file();
    if let Some(d) = file.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    std::fs::write(&file, serde_json::to_vec_pretty(&all).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// Folders package managers put programs in, which a program started from the desktop may not have on its PATH.
pub(crate) fn usual_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if cfg!(windows) {
        if let Some(l) = std::env::var_os("LOCALAPPDATA") {
            v.push(PathBuf::from(&l).join("Microsoft").join("WinGet").join("Links"));
        }
        if let Some(h) = std::env::var_os("USERPROFILE") {
            v.push(PathBuf::from(&h).join("scoop").join("shims"));
        }
        v.push(PathBuf::from(r"C:\ProgramData\chocolatey\bin"));
        v.push(PathBuf::from(r"C:\Program Files\7-Zip"));
        v.push(PathBuf::from(r"C:\Program Files\ImageMagick"));
    } else {
        v.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/snap/bin"].map(PathBuf::from));
        if let Some(h) = std::env::var_os("HOME") {
            v.push(PathBuf::from(h).join(".local").join("bin"));
        }
    }
    v
}

/// Where a tool is: the path chosen in Settings, the PATH, then the usual install folders; FFmpeg's two programs as Curio finds its own. Looked up each time, so a program put in place is seen without restarting.
pub fn find(name: &str) -> Option<PathBuf> {
    let t = tool(name)?;
    if let Some(p) = overrides().get(t.name).filter(|p| p.is_file()) {
        return Some(p.clone());
    }
    let found = match t.name {
        "ffmpeg" | "ffprobe" => crate::core::media::ffmpeg::find(t.exe),
        _ => crate::core::system::filesystem::which(t.exe),
    };
    found.or_else(|| {
        let exe = if cfg!(windows) { format!("{}.exe", t.exe) } else { t.exe.to_string() };
        usual_dirs().into_iter().map(|d| d.join(&exe)).find(|p| p.is_file())
    })
}

static VERSIONS: Mutex<Option<HashMap<PathBuf, (SystemTime, Option<String>)>>> = Mutex::new(None);

/// The first line the program prints about its version, kept until the file changes.
pub fn version(t: &Tool, path: &Path) -> Option<String> {
    let changed = std::fs::metadata(path).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
    if let Some((at, v)) = VERSIONS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).get(path) {
        if *at == changed {
            return v.clone();
        }
    }
    let args: Vec<String> = t.version_args.iter().map(|s| s.to_string()).collect();
    let v = crate::core::system::process::run(&path.to_string_lossy(), &args, None, Duration::from_secs(5)).ok().and_then(|o| {
        let text = String::from_utf8_lossy(&o.stdout).to_string() + &o.stderr;
        text.lines().map(str::trim).find(|l| !l.is_empty()).map(short_version)
    });
    VERSIONS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(path.to_path_buf(), (changed, v.clone()));
    v
}

/// `ffmpeg version 7.1-full_build Copyright …` → `7.1-full_build`: the first word after "version" that starts with a digit.
fn short_version(line: &str) -> String {
    let lower = line.to_lowercase();
    let after = lower.find("version").map(|i| &line[i + "version".len()..]).unwrap_or(line);
    let words: Vec<&str> = after.split(|c: char| c.is_whitespace() || c == ':').filter(|w| !w.is_empty()).collect();
    let v = words.iter().find(|w| w.starts_with(|c: char| c.is_ascii_digit())).or(words.first()).copied().unwrap_or(line);
    v.chars().take(60).collect()
}

/// What Settings and `tool.list` show for a tool; `path` only when `paths` is set.
pub fn describe(t: &Tool, paths: bool) -> Value {
    let found = find(t.name);
    let version = found.as_deref().and_then(|p| version(t, p));
    let mut v = json!({"name": t.name, "title": t.title, "about": t.about, "found": found.is_some(), "version": version, "homepage": t.homepage, "chosen": overrides().contains_key(t.name)});
    if paths {
        v["path"] = json!(found.map(|p| p.to_string_lossy().to_string()));
    }
    v
}
