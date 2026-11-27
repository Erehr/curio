//! Starting with the system, straight to the tray: a per-user entry the app writes itself (the `Run` key on Windows, an XDG autostart file on Linux, a launch agent on macOS), so no installer is needed.

use std::path::PathBuf;

/// The start option an autostart entry passes.
pub const TRAY_ARG: &str = "--tray";

/// The program to start; for an AppImage, the image, since the program itself runs from a temporary mount.
fn program() -> Result<PathBuf, String> {
    if let Some(img) = std::env::var_os("APPIMAGE") {
        return Ok(PathBuf::from(img));
    }
    std::env::current_exe().map_err(|e| format!("can't find the program: {e}"))
}

#[cfg(windows)]
mod sys {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const NAME: &str = "Curio";

    pub fn enabled() -> bool {
        RegKey::predef(HKEY_CURRENT_USER).open_subkey(RUN_KEY).and_then(|k| k.get_value::<String, _>(NAME)).is_ok()
    }

    pub fn set(on: bool) -> Result<(), String> {
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY).map_err(|e| e.to_string())?;
        if on {
            let cmd = format!("\"{}\" {}", super::program()?.display(), super::TRAY_ARG);
            return key.set_value(NAME, &cmd).map_err(|e| e.to_string());
        }
        match key.delete_value(NAME) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }
}

#[cfg(target_os = "linux")]
mod sys {
    use std::path::PathBuf;

    /// The system's own autostart folder, also in portable mode: the desktop only looks there.
    fn entry() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("autostart").join("curio.desktop"))
    }

    /// Quoted as the desktop entry format asks: inside double quotes, `"`, `` ` ``, `$` and `\` are escaped.
    fn quoted(path: &std::path::Path) -> String {
        let mut s = String::from("\"");
        for c in path.to_string_lossy().chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                s.push('\\');
            }
            s.push(c);
        }
        s.push('"');
        s
    }

    pub fn enabled() -> bool {
        entry().is_some_and(|p| p.is_file())
    }

    pub fn set(on: bool) -> Result<(), String> {
        let path = entry().ok_or("no settings folder")?;
        if !on {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
                _ => Ok(()),
            };
        }
        let exec = format!("{} {}", quoted(&super::program()?), super::TRAY_ARG);
        let comment = crate::desktop::locale::text("autostart.comment", &[]);
        let text = format!("[Desktop Entry]\nType=Application\nName=Curio\nComment={comment}\nExec={exec}\nIcon=curio\nTerminal=false\nX-GNOME-Autostart-enabled=true\n");
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, text).map_err(|e| e.to_string())
    }
}

/// A launch agent rather than a login item: only an agent passes `--tray`, and it works from macOS 11. The system lists it under Login Items › Allow in the Background.
#[cfg(target_os = "macos")]
mod sys {
    use std::path::PathBuf;

    const LABEL: &str = "app.curio.desktop.autostart";

    fn entry() -> Option<PathBuf> {
        dirs::home_dir().map(|d| d.join("Library/LaunchAgents").join(format!("{LABEL}.plist")))
    }

    fn escaped(s: &str) -> String {
        s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
    }

    pub fn enabled() -> bool {
        entry().is_some_and(|p| p.is_file())
    }

    pub fn set(on: bool) -> Result<(), String> {
        let path = entry().ok_or("no home folder")?;
        if !on {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
                _ => Ok(()),
            };
        }
        let exe = escaped(&super::program()?.to_string_lossy());
        // Aqua: only in a session with a desktop, never for a login over SSH
        let text = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
    <string>{}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>LimitLoadToSessionType</key>
  <string>Aqua</string>
  <key>ProcessType</key>
  <string>Interactive</string>
</dict>
</plist>
", super::TRAY_ARG);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, text).map_err(|e| e.to_string())
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod sys {
    pub fn enabled() -> bool {
        false
    }

    pub fn set(_on: bool) -> Result<(), String> {
        Err("Start with system isn't supported here yet".into())
    }
}

pub use sys::{enabled, set};
