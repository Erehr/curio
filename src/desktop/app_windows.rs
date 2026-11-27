//! The program's windows: the main, item and Settings windows, showing them behind others in a background run, and quitting.

use crate::desktop::AppState;
use crate::desktop::background;
use crate::desktop::R;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager};
use crate::core::library::instance;
use crate::core::system::logging;
use crate::core::model::*;
use crate::desktop::process::HOLDING;
use crate::desktop::process::RELAUNCH_WAIT;
use crate::desktop::protocol::browser_args;
use crate::desktop::config::own_data;
use crate::desktop::config::save_config;
use crate::desktop::config::load_config;
use crate::desktop::libraries::library_title;
use crate::desktop::settings::saved_look;

/// Settings is a window of its own; asking again brings it forward at the given page.
#[tauri::command]
pub(super) async fn open_prefs_window(app: tauri::AppHandle, tab: Option<String>) -> R<()> {
    show_prefs(&app, tab)
}

pub(crate) fn show_prefs(app: &tauri::AppHandle, tab: Option<String>) -> R<()> {
    let tab = tab.unwrap_or_else(|| "general".into());
    if let Some(w) = app.get_webview_window(PREFS_WINDOW) {
        raise(&w);
        let _ = app.emit("prefs-tab", tab);
        return Ok(());
    }
    let url = format!("index.html?prefs={tab}");
    let w = crate::desktop::system::material::sized(crate::desktop::system::material::frame(own_data(tauri::WebviewWindowBuilder::new(app, PREFS_WINDOW, tauri::WebviewUrl::App(url.into())))), app, PREFS_WIDTH, PREFS_HEIGHT)
        .title(crate::desktop::locale::text("window.settings", &[]))
        .additional_browser_args(&browser_args())
        .min_inner_size(640.0, 420.0)
        .resizable(true)
        .focused(!background())
        .always_on_bottom(background())
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    crate::desktop::system::material::apply(&w, saved_look());
    reveal_fallback(&w);
    Ok(())
}

const PREFS_WINDOW: &str = "prefs";

const PREFS_WIDTH: f64 = 1080.0;

const PREFS_HEIGHT: f64 = 760.0;

/// The main window says what it shows, for the API's `view.changed` and viewer events.
#[tauri::command]
pub(super) async fn window_shown(window: tauri::WebviewWindow, state: tauri::State<'_, AppState>, what: String, value: serde_json::Value) -> R<()> {
    if window.label() != "main" {
        return Ok(());
    }
    let Some(lib) = state.library_id() else { return Ok(()) };
    let w = state.me.lock().window.clone();
    match what.as_str() {
        "view" => {
            let mut data = value;
            data["window"] = serde_json::json!(w);
            state.hub.publish("view.changed", Some(&lib), data);
        }
        "viewer" => {
            let uid = |v: &serde_json::Value| -> R<Option<String>> {
                Ok(match v.as_i64() {
                    Some(i) => state.read(|db| db.uids(&[i]))?.into_iter().next().map(|n| n.1),
                    None => None,
                })
            };
            let shown = match uid(&value["id"])? {
                Some(id) => Some((id.clone(), uid(&value["file"])?.unwrap_or(id))),
                None => None,
            };
            let before = std::mem::replace(&mut *state.viewer_shown.lock(), shown.clone());
            match (shown, before) {
                (Some((id, file)), before) if before.as_ref() != Some(&(id.clone(), file.clone())) => state.hub.publish("viewer.opened", Some(&lib), serde_json::json!({"window": w, "id": id, "file": file})),
                (None, Some((id, _))) => state.hub.publish("viewer.closed", Some(&lib), serde_json::json!({"window": w, "id": id})),
                _ => {}
            }
        }
        _ => return Err(format!("unknown: {what}")),
    }
    Ok(())
}

/// Closing a window with a library open only hides it, while the setting is on.
pub(super) fn keeps_running(app: &tauri::AppHandle) -> bool {
    let st = app.state::<AppState>();
    st.db_path.read().is_some() && load_config().close_to_tray
}

/// The other processes of this app holding a library: with `windows`, only those with a window rather than background holders.
pub(super) fn other_holders(windows: bool) -> Vec<instance::Holder> {
    let cfg = load_config();
    let me = std::process::id();
    let mut libs = cfg.libraries;
    libs.extend(cfg.current_library);
    libs.dedup();
    libs.iter().filter_map(|l| instance::holder(l)).filter(|h| h.pid != me && h.port != 0 && !(windows && h.background)).collect()
}

/// Libraries open in some window of the app right now, with their names, in the order of the library list.
pub(crate) fn open_libraries(app: &tauri::AppHandle) -> Vec<(PathBuf, String)> {
    let mine = app.state::<AppState>().db_path.read().as_ref().and_then(|p| p.parent()).map(Path::to_path_buf);
    let cfg = load_config();
    let mut libs = cfg.libraries;
    libs.extend(cfg.current_library);
    libs.extend(mine.clone());
    let mut seen = std::collections::HashSet::new();
    libs.retain(|l| seen.insert(l.clone()));
    libs.into_iter().filter(|l| mine.as_deref() == Some(l.as_path()) || instance::holder(l).is_some_and(|h| h.port != 0 && !h.background)).map(|l| (l.clone(), instance::library_name(&l))).collect()
}

/// Bring back the window of one library.
pub(crate) fn show_library(app: &tauri::AppHandle, lib: &Path) {
    let mine = app.state::<AppState>().db_path.read().as_ref().and_then(|p| p.parent()).is_some_and(|p| p == lib);
    if mine {
        if let Some(w) = app.get_webview_window("main") {
            raise(&w);
        }
    } else if let Some(h) = instance::holder(lib) {
        let _ = instance::tell(&h, "focus", &serde_json::json!({}));
    }
}

/// Bring back every window, hidden in the tray or not.
pub(crate) fn show_all(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        raise(&w);
    }
    for h in other_holders(true) {
        let _ = instance::tell(&h, "focus", &serde_json::json!({}));
    }
}

/// End every window of the app, then this one.
pub(crate) fn quit_all(app: &tauri::AppHandle) {
    for h in other_holders(false) {
        let _ = instance::tell(&h, "quit", &serde_json::json!({}));
    }
    app.exit(0);
}

#[tauri::command]
pub(super) async fn quit_app(app: tauri::AppHandle) {
    quit_all(&app);
}

/// Start another window of the app, on a library or on the welcome page.
#[tauri::command]
pub(super) async fn new_window(path: Option<String>) -> R<()> {
    spawn_window(path)
}

pub(crate) fn spawn_window(path: Option<String>) -> R<()> {
    if let Some(p) = &path {
        if !instance::is_library(Path::new(p)) {
            return Err(format!("This library is no longer at {p}"));
        }
        if let Some(h) = instance::holder(Path::new(p)) {
            match instance::tell(&h, "focus", &serde_json::json!({})) {
                Ok(_) => return Ok(()),
                // it was ending in the background; the new window takes the library
                Err(_) if h.background => instance::wait_gone(Path::new(p), h.pid, RELAUNCH_WAIT),
                Err(e) => return Err(e),
            }
        }
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut cmd = std::process::Command::new(exe);
    match path {
        Some(p) => cmd.arg("--library").arg(p),
        None => cmd.arg("--welcome"),
    };
    cmd.spawn().map(|_| ()).map_err(|e| format!("could not open another window: {e}"))
}

/// Open items in their own viewer window ("Open Item(s) in New Window").
#[tauri::command]
pub(super) async fn open_item_window(app: tauri::AppHandle, ids: Vec<i64>) -> R<()> {
    item_window(&app, &ids)
}

pub(super) fn item_window(app: &tauri::AppHandle, ids: &[i64]) -> R<()> {
    let list: Vec<String> = ids.iter().map(i64::to_string).collect();
    let url = format!("index.html?items={}", list.join(","));
    let w = crate::desktop::system::material::sized(crate::desktop::system::material::frame(own_data(tauri::WebviewWindowBuilder::new(app, format!("item{}", fastrand::u32(..)), tauri::WebviewUrl::App(url.into())))), app, ITEM_WINDOW_WIDTH, ITEM_WINDOW_HEIGHT)
        .focused(!background())
        .always_on_bottom(background())
        .visible(false)
        .title("Curio")
        .additional_browser_args(&browser_args())
        .min_inner_size(320.0, 240.0)
        .build()
        .map_err(|e| e.to_string())?;
    crate::desktop::system::material::apply(&w, saved_look());
    reveal_fallback(&w);
    Ok(())
}

/// Reveals a secondary window once its page has painted its first frame, avoiding white/transparent startup flashes.
#[tauri::command]
pub(super) fn reveal_window(window: tauri::WebviewWindow) {
    if window.is_visible().unwrap_or(false) {
        return;
    }
    if background() {
        keep_behind(&window.as_ref().window());
        return;
    }
    let _ = window.show();
    let _ = window.set_focus();
}

/// Safety net so a secondary window built hidden still appears if its page fails before calling `reveal_window`.
pub(super) fn reveal_fallback(w: &tauri::WebviewWindow) {
    let w = w.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(2500));
        if !w.is_visible().unwrap_or(true) {
            reveal_window(w);
        }
    });
}

const ITEM_WINDOW_WIDTH: f64 = 1100.0;

const ITEM_WINDOW_HEIGHT: f64 = 760.0;

/// The library's window, brought forward even in a background run since the user asked for it, or made when this process is a background holder.
pub(crate) fn show_library_window(app: &tauri::AppHandle) {
    match app.get_webview_window("main") {
        Some(w) => {
            let _ = w.show();
            let _ = w.unminimize();
            let _ = w.set_focus();
        }
        None => show_main(app),
    }
}

/// The main window, made the first time a background holder's library is opened: from then on it is an ordinary window.
pub(super) fn show_main(app: &tauri::AppHandle) {
    let _one = HOLDING.lock();
    if let Some(w) = app.get_webview_window("main") {
        raise(&w);
        return;
    }
    let st = app.state::<AppState>();
    if st.ending.load(Ordering::Relaxed) {
        return;
    }
    if let Err(e) = build_main_window(app, false) {
        logging::error(&format!("could not show the window: {e}"));
        return;
    }
    let me = {
        let mut me = st.me.lock();
        me.background = false;
        me.focused = now_ms();
        me.clone()
    };
    if let Some(lock) = st.instance.lock().as_ref() {
        lock.update(&me);
    }
    if let Some(p) = st.db_path.read().as_ref().and_then(|p| p.parent()) {
        let mut cfg = load_config();
        cfg.current_library = Some(p.to_path_buf());
        save_config(&cfg);
    }
    crate::desktop::system::tray::start(app);
}

pub(super) fn build_main_window(app: &tauri::AppHandle, hidden: bool) -> tauri::Result<tauri::WebviewWindow> {
    let st = app.state::<AppState>();
    st.window_ready.store(false, Ordering::Relaxed);
    let title = st.db_path.read().as_ref().and_then(|p| p.parent()).map(library_title).unwrap_or_else(|| "Curio".into());
    let mut b = crate::desktop::system::material::sized(crate::desktop::system::material::frame(own_data(tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into())))).title(title), app, 1440.0, 900.0).min_inner_size(720.0, 480.0);
    if background() {
        // stays behind whatever the user is working in
        b = b.focused(false).always_on_bottom(true).visible(false);
    }
    b = b.additional_browser_args(&browser_args());
    if hidden {
        b = b.visible(false);
    }
    let w = b.build()?;
    if !hidden {
        keep_behind(&w.as_ref().window());
    }
    crate::desktop::system::material::apply(&w, saved_look());
    Ok(w)
}

/// Shows a background run's window under the others, where it stays while it opens; built hidden (`.visible(!background())`), since Windows puts a window Tauri shows on top, unfocused but over what the person is working in, and `always_on_bottom` doesn't undo that.
pub(super) fn keep_behind(w: &tauri::Window) {
    #[cfg(windows)]
    if background() {
        use windows::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW;
        push_down(w, SWP_SHOWWINDOW);
        // the web view's first frames can lift it again
        let w = w.clone();
        std::thread::spawn(move || {
            for _ in 0..8 {
                std::thread::sleep(std::time::Duration::from_millis(250));
                push_down(&w, Default::default());
            }
        });
    }
    #[cfg(not(windows))]
    if background() {
        let _ = w.show();
    }
}

#[cfg(windows)]
fn push_down(w: &tauri::Window, extra: windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS) {
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_BOTTOM, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE};
    if let Ok(h) = w.hwnd() {
        let _ = unsafe { SetWindowPos(h, Some(HWND_BOTTOM), 0, 0, 0, 0, SWP_ASYNCWINDOWPOS | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | extra) };
    }
}

/// Bring a window to the front; a background run only shows a hidden one, as showing a shown window or restoring one also activates it on Windows.
pub(super) fn raise(w: &tauri::WebviewWindow) {
    if background() {
        if !w.is_visible().unwrap_or(true) {
            keep_behind(&w.as_ref().window());
        }
        return;
    }
    let _ = w.show();
    let _ = w.unminimize();
    let _ = w.set_focus();
}
