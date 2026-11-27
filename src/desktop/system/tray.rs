//! One tray icon for every window of the app. Each window is its own process, so the process holding `tray.lock` in the settings folder shows the icon and the others take over when it ends.

use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::image::Image;
use tauri::AppHandle;

use crate::core::system::logging;

static HELD: Mutex<Option<File>> = Mutex::new(None);
/// The system has no tray (a Linux desktop without an indicator service): this process stops trying.
static FAILED: AtomicBool = AtomicBool::new(false);
/// How often a window checks whether it should take the icon over, and the icon's owner which libraries are open.
const EVERY: Duration = Duration::from_secs(2);
const TRAY_ID: &str = "curio";
const LIB_PREFIX: &str = "lib:";

#[derive(PartialEq)]
pub enum Tray {
    Mine,
    Elsewhere,
    None,
}

/// Show the icon while no other window does, and keep its list of open libraries current, until this process ends.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("tray".into()).spawn(move || {
        let mut shown: Option<(Vec<(PathBuf, String)>, String)> = None;
        let mut drawn = Look::now();
        while !FAILED.load(Ordering::Relaxed) {
            if claim(&app) == Tray::Mine {
                let look = Look::now();
                if look != drawn {
                    match icon(look).and_then(|i| app.tray_by_id(TRAY_ID).map_or(Ok(()), |t| t.set_icon(Some(i)))) {
                        Ok(()) => drawn = look,
                        Err(e) => logging::error(&format!("tray icon: {e}")),
                    }
                }
                // the libraries open, and the language, which another library's window may have changed
                let now = (crate::desktop::app_windows::open_libraries(&app), crate::desktop::locale::current());
                if shown.as_ref() != Some(&now) {
                    match menu(&app, &now.0).and_then(|m| app.tray_by_id(TRAY_ID).map_or(Ok(()), |t| t.set_menu(Some(m)))) {
                        Ok(()) => shown = Some(now),
                        Err(e) => logging::error(&format!("tray menu: {e}")),
                    }
                }
            }
            std::thread::sleep(EVERY);
        }
    });
}

/// Which process shows the icon, taking it when nobody does.
pub fn claim(app: &AppHandle) -> Tray {
    let mut held = HELD.lock().unwrap_or_else(|e| e.into_inner());
    if held.is_some() {
        return Tray::Mine;
    }
    if FAILED.load(Ordering::Relaxed) {
        return Tray::None;
    }
    let dir = crate::core::system::app_dirs::config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let Ok(file) = OpenOptions::new().create(true).truncate(false).write(true).open(dir.join("tray.lock")) else { return Tray::None };
    match file.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Tray::Elsewhere,
        Err(_) => return Tray::None,
    }
    match build(app) {
        Ok(()) => {
            *held = Some(file);
            Tray::Mine
        }
        Err(e) => {
            logging::error(&format!("no tray icon: {e}"));
            FAILED.store(true, Ordering::Relaxed);
            Tray::None
        }
    }
}

/// One entry per open library, so a window hidden in the tray can be brought back by itself.
fn menu(app: &AppHandle, libs: &[(PathBuf, String)]) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    for (path, name) in libs {
        menu.append(&MenuItem::with_id(app, format!("{LIB_PREFIX}{}", path.display()), crate::desktop::locale::text("tray.show", &[("name", name)]), true, None::<&str>)?)?;
    }
    if libs.len() != 1 {
        menu.append(&MenuItem::with_id(app, "show", crate::desktop::locale::text("tray.showAll", &[]), true, None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "new", crate::desktop::locale::text("tray.newWindow", &[]), true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "prefs", crate::desktop::locale::text("tray.settings", &[]), true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", crate::desktop::locale::text("tray.quit", &[]), true, None::<&str>)?)?;
    Ok(menu)
}

/// What the tray icon is drawn for: whether the bar holding it is light, and the size the system shows it at.
#[derive(Clone, Copy, PartialEq)]
struct Look {
    light: bool,
    size: u32,
}

impl Look {
    fn now() -> Look {
        Look { light: light_bar(), size: tray_size() }
    }
}

/// Whether the taskbar or panel holding the tray is light.
fn light_bar() -> bool {
    #[cfg(windows)]
    {
        winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize").and_then(|k| k.get_value::<u32, _>("SystemUsesLightTheme")).is_ok_and(|v| v == 1)
    }
    #[cfg(target_os = "linux")]
    {
        // these desktops' panels are dark whatever the colour scheme; the others' follow it
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_ascii_lowercase();
        if desktop.split(':').any(|d| matches!(d, "gnome" | "unity" | "x-cinnamon" | "cinnamon")) {
            return false;
        }
        color_scheme() == Some(2)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    false
}

/// The desktop's colour scheme from the settings portal: 0 no preference, 1 dark, 2 light.
#[cfg(target_os = "linux")]
fn color_scheme() -> Option<u32> {
    use gtk::glib::prelude::*;
    use gtk::gio;
    let bus = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok()?;
    let read = |method: &str| bus.call_sync(Some("org.freedesktop.portal.Desktop"), "/org/freedesktop/portal/desktop", "org.freedesktop.portal.Settings", method, Some(&("org.freedesktop.appearance", "color-scheme").to_variant()), None, gio::DBusCallFlags::NONE, 500, gio::Cancellable::NONE).ok();
    // ReadOne answers (v); Read, all that older portals have, wraps the value in one more variant
    let mut value = read("ReadOne").or_else(|| read("Read"))?.child_value(0);
    while let Some(inner) = value.as_variant() {
        value = inner;
    }
    value.get::<u32>()
}

/// The size Windows shows tray icons at, by the taskbar's own scaling: 16 px at 100 %, 20 at 125 %, 24 at 150 %. Linux panels scale the icon to their own size; the macOS menu bar shows it 18 points high, 36 px on a Retina screen.
fn tray_size() -> u32 {
    #[cfg(windows)]
    {
        use windows::core::w;
        use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SM_CXSMICON};
        // the taskbar's scaling changes as soon as the setting does, unlike the system scaling this process started with
        let dpi = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.map(|bar| unsafe { GetDpiForWindow(bar) }).unwrap_or(0);
        let size = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, if dpi == 0 { 96 } else { dpi }) };
        if size > 0 { size as u32 } else { 16 }
    }
    #[cfg(target_os = "macos")]
    return 36;
    #[cfg(not(any(windows, target_os = "macos")))]
    32
}

/// The tray icon is one colour, as the system's own are: white on a dark bar, black on a light one. On macOS the black one is a template, which the menu bar colours itself.
fn icon(look: Look) -> tauri::Result<Image<'static>> {
    let ico: &[u8] = if look.light || cfg!(target_os = "macos") { include_bytes!("../../../web/icons/tray-black.ico") } else { include_bytes!("../../../web/icons/tray-white.ico") };
    let png = ico_entry(ico, look.size).ok_or_else(|| tauri::Error::AssetNotFound("tray icon".into()))?;
    let rgba = image::load_from_memory(png).map_err(|e| tauri::Error::AssetNotFound(format!("tray icon: {e}")))?.into_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(Image::new_owned(rgba.into_raw(), w, h))
}

/// The entry of an icon file drawn for `size`, else the next larger, else the largest. The small sizes are drawn apart, with wider gaps, so scaling one down would lose them.
fn ico_entry(ico: &[u8], size: u32) -> Option<&[u8]> {
    let count = u16::from_le_bytes(ico.get(4..6)?.try_into().ok()?) as usize;
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let e = ico.get(6 + 16 * i..22 + 16 * i)?;
        let width = if e[0] == 0 { 256 } else { e[0] as u32 };
        let len = u32::from_le_bytes(e[8..12].try_into().ok()?) as usize;
        let at = u32::from_le_bytes(e[12..16].try_into().ok()?) as usize;
        entries.push((width, ico.get(at..at + len)?));
    }
    entries.iter().filter(|(w, _)| *w >= size).min_by_key(|(w, _)| *w).or_else(|| entries.iter().max_by_key(|(w, _)| *w)).map(|(_, data)| *data)
}

fn build(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon(Look::now())?)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Curio")
        .menu(&menu(app, &[])?)
        .show_menu_on_left_click(false)
        // windows are made off the main thread, where these handlers run
        .on_menu_event(|app, e| {
            let app = app.clone();
            let id = e.id.as_ref().to_string();
            std::thread::spawn(move || match id.as_str() {
                "show" => crate::desktop::app_windows::show_all(&app),
                "new" => crate::desktop::app_windows::spawn_window(None).unwrap_or_else(|e| logging::error(&e)),
                "prefs" => crate::desktop::app_windows::show_prefs(&app, None).unwrap_or_else(|e| logging::error(&e)),
                "quit" => crate::desktop::app_windows::quit_all(&app),
                other => {
                    if let Some(path) = other.strip_prefix(LIB_PREFIX) {
                        crate::desktop::app_windows::show_library(&app, std::path::Path::new(path));
                    }
                }
            });
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                let app = tray.app_handle().clone();
                std::thread::spawn(move || crate::desktop::app_windows::show_all(&app));
            }
        })
        .build(app)?;
    Ok(())
}
