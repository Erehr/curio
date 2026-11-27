//! Where the app keeps its own files: the system's per-user folders, or in portable mode the `Curio data` folder next to the program.

use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use crate::core::system::filesystem;

/// Its presence next to the program turns portable mode on, so copies of the program on a USB drive need no flag.
pub const PORTABLE_DIR: &str = "Curio data";

struct Mode {
    portable: Option<PathBuf>,
    problem: Option<String>,
}

static MODE: OnceLock<Mode> = OnceLock::new();

/// Decide once, before any other path is asked for. `create` (the `--portable` flag) makes the folder when it is missing.
pub fn init(create: bool) {
    let _ = MODE.set(decide(create));
}

fn mode() -> &'static Mode {
    MODE.get_or_init(|| decide(false))
}

fn decide(create: bool) -> Mode {
    let Some(home) = program_home() else { return Mode { portable: None, problem: None } };
    let dir = home.join(PORTABLE_DIR);
    if !dir.is_dir() && !create {
        return Mode { portable: None, problem: None };
    }
    match std::fs::create_dir_all(&dir).and_then(|_| writable(&dir)) {
        Ok(()) => Mode { portable: Some(dir), problem: None },
        Err(e) => Mode { portable: None, problem: Some(format!("Portable mode is off: can't write to {} ({e}).", dir.display())) },
    }
}

/// The folder holding the program; for an AppImage, the folder holding the image, since the program itself runs from a read-only mount.
fn program_home() -> Option<PathBuf> {
    if let Some(img) = std::env::var_os("APPIMAGE") {
        return Path::new(&img).parent().map(Path::to_path_buf);
    }
    std::env::current_exe().ok()?.parent().map(Path::to_path_buf)
}

fn writable(dir: &Path) -> std::io::Result<()> {
    let probe = dir.join(format!(".write-test-{}", std::process::id()));
    std::fs::write(&probe, b"")?;
    std::fs::remove_file(probe)
}

/// The portable folder, when portable mode is on.
pub fn portable() -> Option<&'static Path> {
    mode().portable.as_deref()
}

/// Why portable mode was asked for but is off.
pub fn problem() -> Option<&'static str> {
    mode().problem.as_deref()
}

/// Settings: `app.json`, the window's settings, the log.
pub fn config_dir() -> PathBuf {
    portable().map(Path::to_path_buf).unwrap_or_else(|| dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("Curio"))
}

/// New libraries, plugins, FFmpeg.
pub fn data_dir() -> PathBuf {
    portable().map(Path::to_path_buf).unwrap_or_else(|| dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("Curio"))
}

/// The web view's own storage; `None` leaves it where the system puts it.
pub fn webview_dir() -> Option<PathBuf> {
    portable().map(|p| p.join("webview"))
}

/// `path` as saved in portable mode: relative to `base`, with `/` separators, when both are on one drive (whose letter may differ on another computer).
pub fn relative_to(base: &Path, path: &Path) -> Option<String> {
    if !path.is_absolute() || filesystem::volume_of(base) != filesystem::volume_of(path) {
        return None;
    }
    let same = |a: &Component, b: &Component| match (a, b) {
        (Component::Prefix(_), Component::Prefix(_)) => true,
        _ if cfg!(windows) => a.as_os_str().to_string_lossy().eq_ignore_ascii_case(&b.as_os_str().to_string_lossy()),
        _ => a == b,
    };
    let b: Vec<Component> = base.components().collect();
    let p: Vec<Component> = path.components().collect();
    let n = b.iter().zip(&p).take_while(|(x, y)| same(x, y)).count();
    let parts: Vec<String> = std::iter::repeat_n("..".to_string(), b.len() - n).chain(p[n..].iter().map(|c| c.as_os_str().to_string_lossy().to_string())).collect();
    Some(if parts.is_empty() { ".".into() } else { parts.join("/") })
}

/// A saved path made whole again: relative ones are taken from `base`.
pub fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let mut out = PathBuf::new();
    for c in base.join(path).components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}
