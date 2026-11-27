//! Opening, creating and listing libraries, and the window that shows each: its events forwarded to it.

use crate::desktop::AppConfig;
use crate::desktop::R;
use crate::desktop::AppState;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};
use crate::core::api;
use crate::core::library::instance;
use crate::core::library::{changes::Changed, LibEvent, Library};
use crate::core::model::*;
use crate::core::said::Said;
use crate::core::system::app_dirs::{config_dir, data_dir};
use crate::desktop::protocol::MEDIA_BASE;
use crate::desktop::config::save_config;
use crate::desktop::config::plugin_dirs;
use crate::desktop::config::prune_plugin_dirs;
use crate::desktop::config::load_config;
use crate::desktop::app_windows::other_holders;

/// Reported to the window when a library was already open elsewhere and that window was raised.
const OPEN_ELSEWHERE: &str = "This library is already open in another window.";

pub(super) fn library_title(path: &Path) -> String {
    let name = instance::library_name(path);
    if name.is_empty() {
        "Curio".into()
    } else {
        format!("{name} - Curio")
    }
}

pub(super) fn close_library_inner(app: &tauri::AppHandle, state: &AppState) {
    *state.lib_gen.lock() += 1;
    if let Some(old) = state.lib.lock().take() {
        old.shutdown();
    }
    state.selection.lock().clear();
    *state.viewer_shown.lock() = None;
    *state.instance.lock() = None;
    *state.db_path.write() = None;
    state.readers.lock().clear();
    *state.thumbs_dir.write() = None;
    *state.tmp_dir.write() = None;
    *state.previews_dir.write() = None;
    state.handlers.write().clear();
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_title("Curio");
    }
}

pub(super) fn open_library_inner(app: &tauri::AppHandle, state: &AppState, path: PathBuf, create: bool) -> R<()> {
    let already = state.db_path.read().as_ref().and_then(|p| p.parent().map(Path::to_path_buf));
    if already.as_deref() == Some(path.as_path()) && state.lib.lock().is_some() {
        return Ok(());
    }
    if create {
        std::fs::create_dir_all(&path).map_err(|e| format!("cannot create the library folder: {e}"))?;
    } else if !path.is_dir() {
        return Err(format!("This library is no longer at {}", path.display()));
    } else if !instance::is_library(&path) {
        return Err(format!("{} isn't a Curio library. To bring in a .library folder, use Import.", path.display()));
    }
    // one window per library
    let new_lock = {
        let me = {
            let mut me = state.me.lock();
            me.opened = now_ms();
            me.clone()
        };
        match instance::acquire(&path, &me)? {
            instance::Acquired::Mine(l) => Some(l),
            instance::Acquired::Elsewhere(_) if me.background => return Err("Another process holds this library".into()),
            // a background holder that isn't busy hands the library over; a busy one shows its window instead
            instance::Acquired::Elsewhere(Some(h)) if h.background => match instance::release(&path, true).then(|| instance::acquire(&path, &me)).transpose()? {
                Some(instance::Acquired::Mine(l)) => Some(l),
                _ => return Err(OPEN_ELSEWHERE.into()),
            },
            instance::Acquired::Elsewhere(h) => {
                if let Some(h) = h {
                    let _ = instance::tell(&h, "focus", &serde_json::json!({}));
                }
                return Err(OPEN_ELSEWHERE.into());
            }
        }
    };
    // the new library opens before the old one closes, so a failure leaves the old one in use
    let mut cfg = load_config();
    prune_plugin_dirs(&mut cfg);
    let known_here = cfg.libraries.contains(&path) || cfg.library_ids.contains_key(&path);
    let lib = Library::open(&path, cfg.cache_dir.as_deref(), &plugin_dirs(&cfg), known_here)?;
    let mut from = None;
    if let Some(old) = state.lib.lock().take() {
        from = Some(old.ctx.library_id.clone());
        old.shutdown();
    }
    state.selection.lock().clear();
    *state.viewer_shown.lock() = None;
    *state.instance.lock() = new_lock;
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_title(&library_title(&path));
    }
    *state.db_path.write() = Some(lib.paths().db.clone());
    state.readers.lock().clear();
    *state.thumbs_dir.write() = Some(lib.paths().thumbs.clone());
    *state.tmp_dir.write() = Some(lib.paths().tmp.clone());
    *state.previews_dir.write() = Some(lib.paths().previews.clone());
    *state.handlers.write() = lib.contributions().thumb_handlers.clone();
    let rx = lib.subscribe();
    let gen = {
        let mut g = state.lib_gen.lock();
        *g += 1;
        *g
    };
    let id = lib.ctx.library_id.clone();
    // downloads left unfinished go on
    lib.downloads(&crate::core::system::app_dirs::data_dir(), Some(state.hub.clone()));
    *state.lib.lock() = Some(lib);
    // a background holder leaves the list and the last library as they are
    let background = state.me.lock().background;
    {
        let (path, id) = (path.clone(), id.clone());
        change_list(app, move |cfg| {
            if !background && !cfg.libraries.contains(&path) {
                cfg.libraries.insert(0, path.clone());
            }
            if !id.is_empty() && cfg.library_ids.get(&path) != Some(&id) {
                cfg.library_ids.insert(path.clone(), id);
            }
            if !background {
                cfg.current_library = Some(path);
            }
            Ok(())
        })?;
    }
    if !background {
        let _ = app.emit("library-switched", path.to_string_lossy().to_string());
    }
    if let Some(from) = from.filter(|_| !background) {
        let window = state.me.lock().window.clone();
        state.hub.publish("library.switched", Some(id.as_str()).filter(|i| !i.is_empty()), serde_json::json!({"window": window, "from": from, "to": id}));
    }
    let (app2, app3) = (app.clone(), app.clone());
    let root = path.clone();
    let hub = state.hub.clone();
    let db_path = state.db_path.read().clone();
    // read only while this library is the one open, so a switch can't name items from the next one
    let names = move |ids: &[AssetId]| app3.state::<AppState>().read(|db| if Some(&db.path) == db_path.as_ref() { db.uids(ids) } else { Ok(vec![]) }).unwrap_or_default();
    std::thread::Builder::new()
        .name("event-forward".into())
        .spawn(move || {
            let mut batch = ToWindow::default();
            api::events::relay(rx, hub, id, names, |ev| {
                if *app2.state::<AppState>().lib_gen.lock() != gen {
                    return false;
                }
                let Some(ev) = ev else {
                    batch.send_due(&app2);
                    return true;
                };
                match ev {
                    LibEvent::Changed(c) => {
                        tell_added(&app2, &root, c);
                        batch.changed(c);
                        batch.send_due(&app2);
                        return true;
                    }
                    LibEvent::ThumbReady(id) => {
                        batch.thumbs.push(*id);
                        batch.send_due(&app2);
                        return true;
                    }
                    _ => {}
                }
                let payload = match ev {
                    LibEvent::Changed(_) | LibEvent::ThumbReady(_) => return true,
                    LibEvent::Progress(m) => serde_json::json!({"kind": "info", "text": m.text(), "said": m}),
                    LibEvent::Done(m) => serde_json::json!({"kind": "done", "text": m.text(), "said": m}),
                    LibEvent::Error(m) => serde_json::json!({"kind": "error", "text": m.text(), "said": m}),
                    // a location's scan starting or ending: its sidebar row shows it under way
                    LibEvent::Scan(n) if n.files == 0 || n.done.is_some() => serde_json::json!({"kind": "scan", "location": n.location, "done": n.done.is_some()}),
                    LibEvent::Scan(_) => return true,
                };
                // what waits goes first, so the window sees events in order
                batch.send(&app2);
                let _ = app2.emit("lib", payload);
                true
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Thumbnails made and changes, for the window, sent together at most every `WINDOW_EVERY`: a large add makes thousands of them in a few seconds, and one event each kept the page busy handling and redrawing.
struct ToWindow {
    thumbs: Vec<AssetId>,
    ids: Vec<AssetId>,
    changed: bool,
    structure: bool,
    requery: bool,
    sent: Instant,
}

const WINDOW_EVERY: Duration = Duration::from_millis(100);

impl Default for ToWindow {
    fn default() -> Self {
        ToWindow { thumbs: vec![], ids: vec![], changed: false, structure: false, requery: false, sent: Instant::now().checked_sub(WINDOW_EVERY).unwrap_or_else(Instant::now) }
    }
}

impl ToWindow {
    fn changed(&mut self, c: &Changed) {
        self.changed = true;
        self.ids.extend(c.all_ids());
        self.structure |= c.sidebar();
        self.requery |= c.requery;
    }

    /// Sends what waits once `WINDOW_EVERY` has passed since the last send: the first after a quiet spell goes at once.
    fn send_due(&mut self, app: &tauri::AppHandle) {
        if self.sent.elapsed() >= WINDOW_EVERY {
            self.send(app);
        }
    }

    fn send(&mut self, app: &tauri::AppHandle) {
        if self.thumbs.is_empty() && !self.changed {
            return;
        }
        if !self.thumbs.is_empty() {
            let _ = app.emit("lib", serde_json::json!({"kind": "thumbs", "ids": std::mem::take(&mut self.thumbs)}));
        }
        if self.changed {
            self.ids.sort_unstable();
            self.ids.dedup();
            let _ = app.emit("lib", serde_json::json!({"kind": "changed", "ids": std::mem::take(&mut self.ids), "structure": self.structure, "requery": self.requery}));
            (self.changed, self.structure, self.requery) = (false, false, false);
        }
        self.sent = Instant::now();
    }
}

/// A notice of what was just added by a drop on the window, over the API or by Auto-Import; a scan, a copy and a restore add nothing new to tell of.
fn tell_added(app: &tauri::AppHandle, root: &Path, c: &Changed) {
    if c.added.is_empty() || !matches!(c.how, "window" | "api" | "autoImport") {
        return;
    }
    let first = c.added[0];
    let read = app.state::<AppState>().read(|db| Ok((db.uids(&c.added)?, db.asset(first)?.map(|a| a.name), db.asset_folders(first)?.into_iter().next().map(|f| f.name))));
    let Ok((items, name, folder)) = read else { return };
    if items.is_empty() {
        return;
    }
    crate::desktop::system::notifications::added(app, crate::core::import::notices::Notice { kind: crate::core::import::notices::Kind::Added, library: root.to_path_buf(), items: items.into_iter().map(|i| i.1).collect(), names: name.into_iter().collect(), folder, errors: vec![], of: None, again: vec![] });
}

/// The libraries on the list, in order, with their ids and looks: what `library.listChanged` is about.
fn list_of(c: &AppConfig) -> String {
    let rows: Vec<serde_json::Value> = c.libraries.iter().map(|p| serde_json::json!([p, c.library_ids.get(p), c.library_looks.get(p).map(|l| (&l.icon, &l.color))])).collect();
    serde_json::to_string(&rows).unwrap_or_default()
}

/// The one way the library list changes: saved, and every window told when it did.
pub(super) fn change_list<T>(app: &tauri::AppHandle, f: impl FnOnce(&mut AppConfig) -> R<T>) -> R<T> {
    let mut cfg = load_config();
    let before = list_of(&cfg);
    let current = cfg.current_library.clone();
    let out = f(&mut cfg)?;
    let changed = list_of(&cfg) != before;
    if changed || cfg.current_library != current {
        save_config(&cfg);
    }
    if changed {
        announce(app, "library.listChanged");
    }
    Ok(out)
}

/// Tells this process's windows, and the other windows of the app, that something they show changed.
fn announce(app: &tauri::AppHandle, name: &'static str) {
    let _ = app.emit("event", serde_json::json!({ "name": name }));
    std::thread::spawn(move || {
        for h in other_holders(true) {
            let _ = instance::tell(&h, "event", &serde_json::json!({ "name": name }));
        }
    });
}

#[tauri::command]
pub(super) async fn startup(state: tauri::State<'_, AppState>) -> R<serde_json::Value> {
    let cfg = load_config();
    Ok(serde_json::json!({
        "frameSeconds": crate::core::consts::PLAYER_FRAME_SECONDS,
        "ratioTolerance": crate::core::consts::RATIO_TOLERANCE,
        "safeMode": state.opts.safe_mode,
        "libraries": cfg.libraries,
        "current": state.db_path.read().as_ref().and_then(|p| p.parent()).map(|p| p.to_path_buf()),
        "problems": state.lib.lock().as_ref().map(|l| l.problems.clone()).unwrap_or_default(),
        "warnings": state.lib.lock().as_ref().map(|l| l.warnings.clone()).unwrap_or_default().into_iter().chain(crate::core::system::app_dirs::problem().map(str::to_string)).collect::<Vec<_>>(),
        "platform": std::env::consts::OS,
        "mediaBase": MEDIA_BASE.get(),
        // a build between releases shows its commit after the version
        "version": if env!("CURIO_BUILD").is_empty() { env!("CURIO_VERSION").to_string() } else { format!("{}+{}", env!("CURIO_VERSION"), env!("CURIO_BUILD")) },
        "configDir": config_dir(),
        "portable": crate::core::system::app_dirs::portable().is_some(),
        "materials": crate::desktop::system::material::materials(),
    }))
}

/// The language a window shows, and the system's, which an empty setting follows.
#[tauri::command]
pub(super) async fn language() -> serde_json::Value {
    serde_json::json!({ "current": crate::desktop::locale::current(), "system": crate::desktop::locale::system() })
}

#[tauri::command]
pub(super) async fn open_library(app: tauri::AppHandle, state: tauri::State<'_, AppState>, path: String) -> Result<(), api::ApiError> {
    let p = PathBuf::from(&path);
    if let Some(said) = crate::core::db::Db::floor_refusal(&p.join("library.db")) {
        return Err(api::ApiError::said("failed", said));
    }
    open_library_inner(&app, &state, p, false).map_err(api::ApiError::from)
}

#[tauri::command]
pub(super) async fn new_library(app: tauri::AppHandle, state: tauri::State<'_, AppState>, name: String, choose: bool) -> Result<Option<String>, api::ApiError> {
    let folder = format!("{}{}", crate::core::import::sanitize_name(name.trim()), instance::LIBRARY_SUFFIX);
    let base = if choose {
        match rfd::FileDialog::new().set_title(crate::desktop::locale::text("dialog.libraryFolder", &[("folder", &folder)])).pick_folder() {
            Some(d) => d,
            None => return Ok(None),
        }
    } else {
        data_dir().join("Libraries")
    };
    let p = base.join(&folder);
    if choose && p.exists() {
        let said = Said::new("errors.library.existsAt", serde_json::json!({"path": p.display().to_string()}), format!("{} already exists. Choose another name, or use Open library folder.", p.display()));
        return Err(api::ApiError::said("failed", said));
    }
    open_library_inner(&app, &state, p.clone(), true)?;
    Ok(Some(p.to_string_lossy().to_string()))
}

#[tauri::command]
pub(super) async fn open_library_dialog(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<bool, api::ApiError> {
    match rfd::FileDialog::new().set_title(crate::desktop::locale::text("dialog.openLibrary", &[])).pick_folder() {
        Some(d) => {
            if let Some(said) = crate::core::db::Db::floor_refusal(&d.join("library.db")) {
                return Err(api::ApiError::said("failed", said));
            }
            open_library_inner(&app, &state, d, false).map_err(api::ApiError::from)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// The libraries this computer knows, and which of them another window has open.
#[tauri::command]
pub(super) async fn library_list(state: tauri::State<'_, AppState>) -> R<serde_json::Value> {
    let cfg = load_config();
    let current = state.db_path.read().as_ref().and_then(|p| p.parent().map(Path::to_path_buf));
    let list: Vec<serde_json::Value> = cfg
        .libraries
        .iter()
        .map(|p| {
            let me = current.as_deref() == Some(p.as_path());
            let look = cfg.library_looks.get(p).cloned().unwrap_or_default();
            serde_json::json!({
                "path": p,
                "id": cfg.library_ids.get(p),
                "name": instance::library_name(p),
                "icon": look.icon,
                "color": look.color,
                "current": me,
                "openElsewhere": !me && instance::holder(p).is_some_and(|h| !h.background),
                "exists": instance::is_library(p),
            })
        })
        .collect();
    Ok(serde_json::json!(list))
}
