//! The public API's host in the shell: the `api` command, calls answered by a window, API tokens and the API port.

use crate::desktop::AppConfig;
use crate::desktop::app_windows::item_window;
use crate::desktop::libraries::change_list;
use crate::desktop::app_windows::spawn_window;
use crate::desktop::app_windows::raise;
use crate::desktop::settings::read_ui_settings;
use crate::desktop::AppState;
use crate::desktop::R;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager};
use crate::core::api;
use crate::core::library::instance;
use crate::core::db::Db;
use crate::core::library::Library;
use crate::core::system::logging;
use crate::desktop::config::save_config;
use crate::desktop::config::load_config;

pub(super) fn token_json(t: &api::tokens::ApiToken) -> serde_json::Value {
    serde_json::json!({"id": t.id, "name": t.name, "perms": t.perms, "origins": t.origins, "libraries": t.libraries, "created": t.created})
}

/// Web tokens: `create` (returns the secret, shown once), `update` and `delete`.
#[tauri::command]
pub(super) async fn api_token_op(op: String, id: Option<String>, name: Option<String>, perms: Option<Vec<String>>, origins: Option<Vec<String>>, libraries: Option<Vec<String>>) -> R<serde_json::Value> {
    let mut cfg = load_config();
    let parse = |list: Option<Vec<String>>| list.map(|l| l.iter().filter_map(|p| api::Perm::parse(p)).filter(|p| api::Perm::FOR_TOKENS.contains(p)).collect::<Vec<_>>());
    let out = match op.as_str() {
        "create" => {
            let (t, secret) = api::tokens::make(name.as_deref().unwrap_or(""), parse(perms).unwrap_or_default(), origins.unwrap_or_default(), libraries.unwrap_or_default());
            let v = serde_json::json!({"token": token_json(&t), "secret": secret});
            cfg.api_tokens.push(t);
            v
        }
        "update" => {
            let t = cfg.api_tokens.iter_mut().find(|t| Some(&t.id) == id.as_ref()).ok_or("That token was deleted")?;
            if let Some(n) = name.filter(|n| !n.trim().is_empty()) {
                t.name = n.trim().to_string();
            }
            if let Some(p) = parse(perms) {
                let mut next: Vec<String> = p.into_iter().map(|x| x.name().to_string()).collect();
                for existing in &t.perms {
                    if api::Perm::parse(existing).is_none() && !next.contains(existing) {
                        next.push(existing.clone());
                    }
                }
                t.perms = next;
            }
            if let Some(o) = origins {
                t.origins = o.iter().map(|x| api::tokens::clean_origin(x)).filter(|x| !x.is_empty()).collect();
            }
            if let Some(l) = libraries {
                t.libraries = l;
            }
            token_json(t)
        }
        "delete" => {
            cfg.api_tokens.retain(|t| Some(&t.id) != id.as_ref());
            serde_json::Value::Null
        }
        other => return Err(format!("Unknown token op {other}")),
    };
    save_config(&cfg);
    Ok(out)
}

/// The window's way into the API: every permission, and its own library unless `library` names another.
#[tauri::command]
pub(super) async fn api(app: tauri::AppHandle, op: String, args: Option<serde_json::Value>) -> Result<serde_json::Value, api::ApiError> {
    // a call may wait on another process, or on the person answering a question
    tauri::async_runtime::spawn_blocking(move || api::handle(&AppHost(app), &api::Caller::window(), &op, args.unwrap_or_default())).await.map_err(|e| api::ApiError::from(e.to_string()))?
}

/// How long a call waits for a window's page to load.
const WINDOW_START: Duration = Duration::from_secs(30);

/// What this process offers the API.
pub(crate) struct AppHost(pub(crate) tauri::AppHandle);

impl api::Host for AppHost {
    fn library(&self) -> Option<PathBuf> {
        self.0.state::<AppState>().db_path.read().as_ref().and_then(|p| p.parent()).map(Path::to_path_buf)
    }

    fn me(&self) -> instance::Holder {
        self.0.state::<AppState>().me.lock().clone()
    }

    fn read(&self, f: &mut dyn FnMut(&Db)) -> R<()> {
        self.0.state::<AppState>().read(|db| {
            f(db);
            Ok(())
        })
    }

    fn with_library(&self, f: &mut dyn FnMut(&Library)) -> R<()> {
        self.0.state::<AppState>().with_lib(|lib| {
            f(lib);
            Ok(())
        })
    }

    fn selection(&self) -> Vec<i64> {
        self.0.state::<AppState>().selection.lock().clone()
    }

    fn recent_folders(&self) -> Vec<i64> {
        let (Some(lib), Some(ui)) = (self.library(), read_ui_settings()) else { return vec![] };
        let v: serde_json::Value = serde_json::from_str(&ui).unwrap_or_default();
        v["recents"][lib.to_string_lossy().as_ref()]["folders"].as_array().map(|a| a.iter().filter_map(|x| x.as_i64()).collect()).unwrap_or_default()
    }

    fn change_library(&self, change: api::LibraryChange) -> R<PathBuf> {
        use api::LibraryChange as C;
        let mine = self.library();
        let open_here = |p: &Path| mine.as_deref() == Some(p);
        if let C::Open(p) = change {
            if open_here(&p) {
                if let Some(w) = self.0.get_webview_window("main") {
                    raise(&w);
                }
            } else {
                spawn_window(Some(p.to_string_lossy().to_string()))?;
            }
            return Ok(p);
        }
        change_list(&self.0, |cfg| {
            Ok(match change {
                C::Open(p) => p,
                C::Add { path, id } => {
                    if !cfg.libraries.contains(&path) {
                        cfg.libraries.push(path.clone());
                    }
                    cfg.library_ids.insert(path.clone(), id);
                    path
                }
                C::Rename(from, name) => {
                    if open_here(&from) {
                        return Err(instance::OPEN_LIBRARY_RENAME.into());
                    }
                    instance::release(&from, false);
                    let to = instance::rename_library(&from, &name)?;
                    if to == from {
                        return Ok(to);
                    }
                    for p in cfg.libraries.iter_mut().filter(|p| **p == from) {
                        *p = to.clone();
                    }
                    if cfg.current_library.as_deref() == Some(from.as_path()) {
                        cfg.current_library = Some(to.clone());
                    }
                    if let Some(look) = cfg.library_looks.remove(&from) {
                        cfg.library_looks.insert(to.clone(), look);
                    }
                    if let Some(id) = cfg.library_ids.remove(&from) {
                        cfg.library_ids.insert(to.clone(), id);
                    }
                    to
                }
                C::Forget(target) => {
                    if open_here(&target) {
                        crate::desktop::libraries::close_library_inner(&self.0, &self.0.state::<AppState>());
                    } else {
                        instance::release(&target, false);
                        if instance::holder(&target).is_some() {
                            return Err("That library is open. Close its window first.".into());
                        }
                    }
                    cfg.libraries.retain(|p| p != &target);
                    cfg.library_looks.remove(&target);
                    cfg.library_ids.remove(&target);
                    if cfg.current_library.as_deref() == Some(target.as_path()) {
                        cfg.current_library = None;
                    }
                    target
                }
                C::Place(p, before) => {
                    let Some(from) = cfg.libraries.iter().position(|x| x == &p) else { return Err("That library is not on the list".into()) };
                    cfg.libraries.remove(from);
                    let at = before.and_then(|b| cfg.libraries.iter().position(|x| x == &b)).unwrap_or(cfg.libraries.len());
                    cfg.libraries.insert(at, p.clone());
                    p
                }
                C::Look { path, icon, color } => {
                    let look = cfg.library_looks.entry(path.clone()).or_default();
                    if let Some(i) = icon {
                        look.icon = Some(i).filter(|v| !v.is_empty());
                    }
                    if let Some(c) = color {
                        look.color = Some(c).filter(|v| !v.is_empty());
                    }
                    if look.icon.is_none() && look.color.is_none() {
                        cfg.library_looks.remove(&path);
                    }
                    path
                }
            })
        })
    }

    fn activity(&self) -> Option<&api::Activity> {
        Some(&self.0.state::<AppState>().inner().activity)
    }

    fn events(&self) -> Option<Arc<api::events::Hub>> {
        Some(self.0.state::<AppState>().hub.clone())
    }

    fn window(&self, op: &str, args: serde_json::Value, ask: bool) -> Option<api::ApiResult> {
        window_call(&self.0, "main", op, args, ask)
    }

    fn theme(&self) -> Option<serde_json::Value> {
        self.0.state::<AppState>().theme.lock().clone()
    }

    fn language(&self) -> String {
        crate::desktop::locale::current()
    }

    fn start_holder(&self, library: &Path) -> R<()> {
        let name = instance::library_name(library);
        logging::info(&format!("opening {name} in the background for the API"));
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut child = std::process::Command::new(exe).arg("--headless").arg("--library").arg(library).spawn().map_err(|e| format!("Couldn't open {name} in the background: {e}"))?;
        std::thread::spawn(move || child.wait());
        Ok(())
    }

    fn config(&self) -> api::ApiConfig {
        let cfg = load_config();
        let libraries = cfg
            .libraries
            .iter()
            .map(|p| {
                let look = cfg.library_looks.get(p).cloned().unwrap_or_default();
                api::KnownLibrary { path: p.clone(), id: cfg.library_ids.get(p).cloned(), icon: look.icon, color: look.color }
            })
            .collect();
        api::ApiConfig { libraries, tokens: cfg.api_tokens, mcp_enabled: cfg.mcp_enabled, mcp_disabled: cfg.mcp_disabled, current: cfg.current_library }
    }

    fn add_token(&self, token: api::tokens::ApiToken) -> R<()> {
        let mut cfg = load_config();
        cfg.api_tokens.push(token);
        save_config(&cfg);
        Ok(())
    }

    fn drop_token(&self, id: &str) {
        let mut cfg = load_config();
        cfg.api_tokens.retain(|t| t.id != id);
        save_config(&cfg);
    }
}

/// Has the window `label` do a window operation and waits for its answer; `ask` brings it forward and waits for the person.
pub(super) fn window_call(app: &tauri::AppHandle, label: &str, op: &str, args: serde_json::Value, ask: bool) -> Option<api::ApiResult> {
    let st = app.state::<AppState>();
    if st.me.lock().background {
        return None;
    }
    let w = app.get_webview_window(label)?;
    let _one = match ask.then(|| st.asking.try_lock()) {
        Some(None) => return Some(Err(api::ApiError::new("busy", "The window is already asking something"))),
        Some(g) => g,
        None => None,
    };
    if ask {
        raise(&w);
    }
    match op {
        "item.window" => {
            let ids: Vec<i64> = serde_json::from_value(args["ids"].clone()).unwrap_or_default();
            return Some(item_window(app, &ids).map(|_| serde_json::json!({})).map_err(api::ApiError::from));
        }
        "dialog.openFile" | "dialog.openFolder" | "dialog.saveFile" => return Some(api_dialog(&w, op, &args)),
        _ => {}
    }
    let wait = api::window_wait(op, ask);
    let started = std::time::Instant::now();
    // a window just opened may still be loading its page; any other window asking is already loaded
    while label == "main" && !st.window_ready.load(Ordering::Relaxed) {
        if started.elapsed() > WINDOW_START {
            return Some(Err(api::ApiError::new("busy", "The window is still starting")));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let id = st.window_seq.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = crossbeam_channel::bounded(1);
    st.window_calls.lock().insert(id, tx);
    // every window's page hears every event, so the call names the one meant
    let _ = app.emit_to(label, "api-window", serde_json::json!({"id": id, "op": op, "args": args, "to": label}));
    let got = rx.recv_timeout(wait);
    st.window_calls.lock().remove(&id);
    Some(got.unwrap_or_else(|_| {
        let _ = app.emit_to(label, "api-window-cancel", id);
        Err(api::ApiError::new("busy", if ask { "Nobody answered in the window" } else { "The window didn't answer in time" }))
    }))
}

/// Answers on the API's port when it is free; stops when the API is turned off or moved to another port.
pub(super) fn claim_api_port(app: &tauri::AppHandle, cfg: &AppConfig) {
    let st = app.state::<AppState>();
    let mut cur = st.api.lock();
    let port = cfg.api_port();
    if cur.as_ref().is_some_and(|s| !cfg.api_enabled || s.port != port) {
        if let Some(s) = cur.take() {
            s.stop();
        }
    }
    if cfg.api_enabled && cur.is_none() {
        if let Ok(s) = api::http::start(Arc::new(AppHost(app.clone())), port) {
            logging::info(&format!("API listening on http://127.0.0.1:{port}{}", api::http::PREFIX));
            *cur = Some(s);
        }
    }
}

/// The process answering on the port may quit at any time; another one then takes the port over.
pub(super) fn keep_claiming_api_port(app: &tauri::AppHandle) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("api-claim".into()).spawn(move || loop {
        claim_api_port(&app, &load_config());
        std::thread::sleep(Duration::from_secs(3));
    });
}

#[tauri::command]
pub(super) fn api_window_ready(window: tauri::WebviewWindow, state: tauri::State<AppState>) {
    if window.label() == "main" {
        state.window_ready.store(true, Ordering::Relaxed);
    }
}

/// The window's answer to an API call it was handed (see `AppHost::window`).
#[tauri::command]
pub(super) fn api_window_reply(state: tauri::State<AppState>, id: u64, ok: Option<serde_json::Value>, error: Option<api::ApiError>) {
    if let Some(tx) = state.window_calls.lock().remove(&id) {
        let _ = tx.send(error.map_or_else(|| Ok(ok.unwrap_or_default()), Err));
    }
}

/// The system's file dialogs for the API, over the window.
fn api_dialog(w: &tauri::WebviewWindow, op: &str, args: &serde_json::Value) -> api::ApiResult {
    let mut d = rfd::FileDialog::new().set_parent(w);
    if let Some(t) = args["title"].as_str() {
        d = d.set_title(t);
    }
    for f in args["filters"].as_array().into_iter().flatten() {
        let exts: Vec<&str> = f["extensions"].as_array().into_iter().flatten().filter_map(|e| e.as_str()).collect();
        d = d.add_filter(f["name"].as_str().unwrap_or_default(), &exts);
    }
    let shown = |p: PathBuf| p.to_string_lossy().to_string();
    Ok(match op {
        "dialog.openFile" if args["multiple"] == true => serde_json::json!({"paths": d.pick_files().map(|v| v.into_iter().map(shown).collect::<Vec<_>>())}),
        "dialog.openFile" => serde_json::json!({"paths": d.pick_file().map(|p| vec![shown(p)])}),
        "dialog.openFolder" => serde_json::json!({"path": d.pick_folder().map(shown)}),
        _ => {
            if let Some(n) = args["fileName"].as_str() {
                d = d.set_file_name(n);
            }
            serde_json::json!({"path": d.save_file().map(shown)})
        }
    })
}
