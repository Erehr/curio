//! `curio://` links, which let a browser extension start Curio: the parser, the start options they leave, the per-user registration of the scheme, and on a Mac the links the system hands the running program.
//! Any page can make such a link, so a link only starts, shows and focuses; it never carries a token, a file or a flag.

/// What a `curio:` link asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Link {
    /// Start as a plain start does; with `background`, as `--tray` does, and not at all when Curio already answers.
    Open { background: bool },
    /// That library's window, by its id.
    Library(String),
}

const SCHEME: &str = "curio:";
const MAX_LEN: usize = 2048;

/// Whether a start argument is a link; the scheme is matched as browsers send it, in any case.
pub fn is_link(arg: &str) -> bool {
    arg.len() >= SCHEME.len() && arg.as_bytes()[..SCHEME.len()].eq_ignore_ascii_case(SCHEME.as_bytes())
}

/// The link an argument holds; anything malformed or unknown is `Open` without background.
pub fn parse(arg: &str) -> Option<Link> {
    if !is_link(arg) {
        return None;
    }
    let plain = Link::Open { background: false };
    if arg.len() > MAX_LEN {
        return Some(plain);
    }
    let rest = &arg[SCHEME.len()..];
    let rest = rest.split('#').next().unwrap_or("");
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let path = path.trim_matches('/');
    let lower = path.to_ascii_lowercase();
    if lower.is_empty() || lower == "open" {
        let background = query.split('&').any(|p| matches!(p.split_once('='), Some(("background", "1" | "true"))));
        return Some(Link::Open { background });
    }
    if let Some(id) = lower.strip_prefix("library/") {
        if !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
            return Some(Link::Library(path["library/".len()..].to_string()));
        }
    }
    Some(plain)
}

/// The arguments a start goes on with: when any is a link, the first link alone, since whoever made it also chose the rest (`--portable`, `--library`, `--screenshot` and file paths all have effects).
pub fn start_args(args: Vec<String>) -> (Option<Link>, Vec<String>) {
    match args.iter().find(|a| is_link(a)) {
        Some(link) => (parse(link), Vec::new()),
        None => (None, args),
    }
}

/// Whether this copy of the program may register the scheme for the user: never a portable copy (the sandbox is one), and a debug build only when asked.
pub fn allowed_for(portable: bool, debug: bool, asked: bool) -> bool {
    !portable && (!debug || asked)
}

pub fn allowed() -> bool {
    allowed_for(crate::core::system::app_dirs::portable().is_some(), cfg!(debug_assertions), std::env::var_os("CURIO_REGISTER_SCHEME").is_some_and(|v| v == "1"))
}

/// The program a link starts; for an AppImage, the image, since the program itself runs from a temporary mount.
#[cfg(any(windows, target_os = "linux"))]
fn program() -> Result<std::path::PathBuf, String> {
    if let Some(img) = std::env::var_os("APPIMAGE") {
        return Ok(std::path::PathBuf::from(img));
    }
    std::env::current_exe().map_err(|e| format!("can't find the program: {e}"))
}

#[cfg(windows)]
mod sys {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    const KEY: &str = r"Software\Classes\curio";

    fn command() -> Result<String, String> {
        Ok(format!("\"{}\" \"%1\"", super::program()?.display()))
    }

    pub fn registered() -> bool {
        let cmd = RegKey::predef(HKEY_CURRENT_USER).open_subkey(format!(r"{KEY}\shell\open\command")).and_then(|k| k.get_value::<String, _>(""));
        matches!((cmd, command()), (Ok(a), Ok(b)) if a == b)
    }

    pub fn set(on: bool) -> Result<(), String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if !on {
            return match hkcu.delete_subkey_all(KEY) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
                _ => Ok(()),
            };
        }
        if registered() {
            return Ok(());
        }
        let exe = super::program()?;
        let (key, _) = hkcu.create_subkey(KEY).map_err(|e| e.to_string())?;
        key.set_value("", &"URL:Curio").map_err(|e| e.to_string())?;
        key.set_value("URL Protocol", &"").map_err(|e| e.to_string())?;
        let (icon, _) = key.create_subkey("DefaultIcon").map_err(|e| e.to_string())?;
        icon.set_value("", &format!("\"{}\",0", exe.display())).map_err(|e| e.to_string())?;
        let (cmd, _) = key.create_subkey(r"shell\open\command").map_err(|e| e.to_string())?;
        cmd.set_value("", &command()?).map_err(|e| e.to_string())
    }
}

#[cfg(target_os = "linux")]
mod sys {
    use std::path::PathBuf;

    const NAME: &str = "curio-url.desktop";

    /// A package installs its own entry under /usr; a program anywhere else (an AppImage, a build) writes one for the user.
    fn packaged() -> bool {
        std::path::Path::new("/usr/share/applications").join(NAME).is_file()
    }

    fn entry() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("applications").join(NAME))
    }

    fn text() -> Result<String, String> {
        let exe = super::program()?.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"").replace('`', "\\`").replace('$', "\\$");
        Ok(format!("[Desktop Entry]\nType=Application\nName=Curio\nExec=\"{exe}\" %u\nIcon=curio\nTerminal=false\nNoDisplay=true\nMimeType=x-scheme-handler/curio;\n"))
    }

    pub fn registered() -> bool {
        packaged() || matches!((entry().map(std::fs::read_to_string), text()), (Some(Ok(a)), Ok(b)) if a == b)
    }

    pub fn set(on: bool) -> Result<(), String> {
        if packaged() {
            return Ok(());
        }
        let path = entry().ok_or("no data folder")?;
        if !on {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
                _ => Ok(()),
            };
        }
        if registered() {
            return Ok(());
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, text()?).map_err(|e| e.to_string())?;
        let _ = std::process::Command::new("xdg-mime").args(["default", NAME, "x-scheme-handler/curio"]).status();
        Ok(())
    }
}

/// A Mac registers the scheme from the bundle's `Info.plist` and hands links to the running program as an Apple event, never as an argument.
#[cfg(target_os = "macos")]
mod sys {
    pub fn set(_on: bool) -> Result<(), String> {
        Ok(())
    }
}

/// Links macOS hands this process. The first, arriving before the window has loaded in a start made with no arguments, is the link the start was made for; any other starts the program again with it, which shows or opens what it asks for as a start from a link does elsewhere.
#[cfg(target_os = "macos")]
pub(in crate::desktop) fn opened(app: &tauri::AppHandle, urls: Vec<tauri::Url>) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use tauri::Manager;
    static FIRST: AtomicBool = AtomicBool::new(true);
    for url in urls {
        let Some(link) = parse(url.as_str()) else { continue };
        let st = app.state::<crate::desktop::AppState>();
        let plain = std::env::args().skip(1).all(|a| a.starts_with("-psn_"));
        let own = FIRST.swap(false, Ordering::Relaxed) && plain && !st.window_ready.load(Ordering::Relaxed);
        if !own {
            start_again(&url);
            continue;
        }
        let cfg = crate::desktop::config::load_config();
        if cfg.no_browser_start {
            app.exit(0);
            return;
        }
        match link {
            Link::Open { background: false } => {}
            Link::Open { background: true } => {
                if let (Some(w), true) = (app.get_webview_window("main"), st.db_path.read().is_some()) {
                    let app = app.clone();
                    // claiming the tray may wait for the main thread, which runs this
                    std::thread::spawn(move || {
                        if crate::desktop::system::tray::claim(&app) != crate::desktop::system::tray::Tray::None {
                            let _ = w.hide();
                        }
                    });
                }
            }
            Link::Library(id) => {
                let known = cfg.library_ids.values().any(|v| v.eq_ignore_ascii_case(&id));
                if known && !st.library_id().is_some_and(|mine| mine.eq_ignore_ascii_case(&id)) {
                    start_again(&url);
                    app.exit(0);
                    return;
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn start_again(url: &tauri::Url) {
    let started = std::env::current_exe().and_then(|exe| std::process::Command::new(exe).arg(url.as_str()).spawn());
    if let Err(e) = started {
        crate::core::system::logging::warn(&format!("couldn't follow a curio:// link: {e}"));
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod sys {
    pub fn registered() -> bool {
        false
    }

    pub fn set(_on: bool) -> Result<(), String> {
        Ok(())
    }
}


/// Registers or removes the scheme, when this copy may.
pub fn set(on: bool) -> Result<(), String> {
    if !allowed() {
        return Ok(());
    }
    sys::set(on)
}
