//! Opening files and addresses, reading and saving text, drags out of the window and drops into it.

use crate::desktop::AppState;
use crate::desktop::R;
use std::path::{Path, PathBuf};
use tauri::Manager;
use crate::core::library::instance;
use crate::core::library::changes::Changed;
use crate::core::library::changes::Parts;
use crate::core::system::logging;
use crate::core::system::app_dirs::data_dir;

/// The viewer showed an item, which puts it in Recently used.
#[tauri::command]
pub(super) async fn mark_used(state: tauri::State<'_, AppState>, id: i64) -> R<()> {
    state.with_lib(|l| {
        l.write(move |db, _| db.mark_used(id).map(|_| Changed::parts(Parts::SIDEBAR)));
        Ok(())
    })
}

#[tauri::command]
pub(super) async fn open_item(state: tauri::State<'_, AppState>, id: i64, how: String) -> R<()> {
    let p = state.read(|db| db.abs_path(id).ok_or("not found".into()))?;
    match how.as_str() {
        "reveal" => crate::core::system::filesystem::reveal(&p)?,
        _ => crate::core::system::filesystem::open_default(&p)?,
    }
    state.with_lib(|l| {
        l.write(move |db, _| db.mark_used(id).map(|_| Changed::parts(Parts::SIDEBAR)));
        Ok(())
    })
}

/// The system's "Open with" chooser.
#[tauri::command]
pub(super) async fn open_with(state: tauri::State<'_, AppState>, id: i64) -> R<()> {
    let p = state.read(|db| db.abs_path(id).ok_or("not found".into()))?;
    #[cfg(windows)]
    {
        std::process::Command::new("rundll32.exe").arg("shell32.dll,OpenAs_RunDLL").arg(&p).spawn().map_err(|e| e.to_string())?;
    }
    // a Mac has no chooser of its own to call: Finder's Open With > Other picks an app from Applications, as this does
    #[cfg(target_os = "macos")]
    {
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let Some(app) = rfd::FileDialog::new().set_title(crate::desktop::locale::text("dialog.openWith", &[("name", &name)])).set_directory("/Applications").add_filter(crate::desktop::locale::text("dialog.apps", &[]), &["app"]).pick_file() else { return Ok(()) };
        std::process::Command::new("open").arg("-a").arg(&app).arg(&p).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        crate::core::system::filesystem::open_default(&p)?;
    }
    Ok(())
}

#[tauri::command]
pub(super) async fn open_url(url: String) -> R<()> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("only web links can be opened".into());
    }
    // ShellExecuteW("open", url, SW_SHOWNORMAL); failure → "Failed to open: …"
    opener::open(&url).map_err(|e| format!("Failed to open: {e}"))
}

/// Save text through a native save dialog (theme export and similar).
#[tauri::command]
pub(super) async fn save_text(name: String, content: String) -> R<bool> {
    let Some(p) = rfd::FileDialog::new().set_file_name(&name).save_file() else { return Ok(false) };
    std::fs::write(&p, content).map_err(|e| e.to_string())?;
    Ok(true)
}

fn read_capped_text(path: &Path) -> R<String> {
    let md = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if md.len() > 2 * 1024 * 1024 {
        return Err("file is too large".into());
    }
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

/// Pick and read a text file (theme import) inside Rust so the window has no arbitrary path reader.
#[tauri::command]
pub(super) async fn pick_text_file() -> R<Option<serde_json::Value>> {
    tauri::async_runtime::spawn_blocking(|| {
        let Some(p) = rfd::FileDialog::new().pick_file() else { return Ok(None) };
        let text = read_capped_text(&p)?;
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        Ok(Some(serde_json::json!({ "name": name, "text": text })))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Pick a theme CSS file and store its path in app.json (never settable directly by the window).
#[tauri::command]
pub(super) async fn pick_theme_file() -> R<Option<String>> {
    tauri::async_runtime::spawn_blocking(|| {
        let Some(p) = rfd::FileDialog::new().pick_file() else { return Ok(None) };
        let shown = p.to_string_lossy().into_owned();
        let mut cfg = crate::desktop::config::load_config();
        cfg.theme_file = Some(p);
        crate::desktop::config::save_config(&cfg);
        Ok(Some(shown))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Clear the stored theme file path.
#[tauri::command]
pub(super) async fn clear_theme_file() -> R<()> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut cfg = crate::desktop::config::load_config();
        cfg.theme_file = None;
        crate::desktop::config::save_config(&cfg);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Read the user-picked theme file stored in app.json. Capped at 2 MB.
#[tauri::command]
pub(super) async fn read_theme_file() -> R<String> {
    tauri::async_runtime::spawn_blocking(|| {
        let Some(p) = crate::desktop::config::load_config().theme_file else { return Ok(String::new()) };
        read_capped_text(&p)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Native drag of the selected files out of the window (to Explorer, other apps).
#[tauri::command]
pub(super) async fn start_drag(app: tauri::AppHandle, state: tauri::State<'_, AppState>, ids: Vec<i64>) -> R<()> {
    let (paths, icon) = state.read(|db| {
        let paths: Vec<PathBuf> = ids.iter().filter_map(|i| db.abs_path(*i)).filter(|p| p.exists()).collect();
        let icon = ids.first().and_then(|i| db.asset(*i).ok().flatten()).map(|a| (a.uid, a.thumb_ver));
        Ok((paths, icon))
    })?;
    if paths.is_empty() {
        return Err("nothing to drag (files missing or offline)".into());
    }
    leave_transfer(&state, &ids);
    // drag image: the first item's thumbnail, else a plain tile
    let mut png = Vec::new();
    let img = icon
        .and_then(|(uid, ver)| state.thumbs_dir.read().clone().map(|d| crate::core::thumbs::thumb_path(&d, &uid, ver)))
        .and_then(|p| image::ImageReader::open(p).ok()?.with_guessed_format().ok()?.decode().ok())
        .map(|i| i.thumbnail(DRAG_ICON_SIZE, DRAG_ICON_SIZE))
        .unwrap_or_else(|| image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(64, 64, image::Rgba([50, 151, 255, 200]))));
    let _ = img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png);
    let window = app.get_webview_window("main").ok_or("no window")?;
    let w2 = window.clone();
    window
        .run_on_main_thread(move || {
            #[cfg(target_os = "linux")]
            let target = match w2.gtk_window() {
                Ok(g) => g,
                Err(_) => return,
            };
            #[cfg(target_os = "linux")]
            let target = &target;
            #[cfg(not(target_os = "linux"))]
            let target = &w2;
            if let Err(e) = drag::start_drag(target, drag::DragItem::Files(paths), drag::Image::Raw(png), |_res, _pos| {}, drag::Options::default()) {
                logging::warn(&format!("drag out failed: {e}"));
            }
        })
        .map_err(|e| e.to_string())
}

/// This window takes what other programs drop on it beyond files with paths (`drop-in` events), where Curio has a drop target of its own.
#[tauri::command]
pub(super) fn take_drops(window: tauri::WebviewWindow) {
    crate::desktop::system::dropin::take(&window);
}

/// Whether another window stands over this one under the pointer: a drag held inside this window still has to go to the operating system to land there.
#[tauri::command]
pub(super) fn pointer_over_other_window(window: tauri::WebviewWindow) -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::{GA_ROOT, GetAncestor, GetCursorPos, WindowFromPoint};
        let Ok(own) = window.hwnd() else { return false };
        let mut pt = POINT::default();
        if unsafe { GetCursorPos(&mut pt) }.is_err() {
            return false;
        }
        let under = unsafe { GetAncestor(WindowFromPoint(pt), GA_ROOT) };
        !under.0.is_null() && under.0 as isize != own.0 as isize
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        false
    }
}

const DRAG_ICON_SIZE: u32 = 96;

/// Tags, ratings and notes of these items, for the library that receives them.
pub(super) fn leave_transfer(state: &AppState, ids: &[i64]) {
    let items = state.read(|db| {
        let fields: std::collections::HashMap<i64, crate::core::model::FieldDef> = db.fields()?.into_iter().map(|f| (f.id, f)).collect();
        let list = db.assets(ids)?;
        Ok(list.iter().map(|a| instance::transfer_key(&a.abs_path)).zip(crate::core::export::assets_json(db, &list, &fields)).collect::<Vec<_>>())
    });
    if let Ok(items) = items {
        instance::write_transfer(&data_dir(), items);
    }
}
