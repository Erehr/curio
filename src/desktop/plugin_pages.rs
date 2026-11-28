//! Plugin pages: their `plugin://` addresses, what they may call and the files they read and write.

use crate::desktop::settings::saved_look;
use crate::desktop::app_windows::{raise, reveal_fallback};
use crate::desktop::background;
use crate::desktop::PluginWin;
use crate::desktop::FileToken;
use crate::desktop::api_host::AppHost;
use crate::desktop::R;
use crate::desktop::AppState;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager};
use crate::core::api;
use crate::core::library::instance;
use crate::core::db::Db;
use crate::core::library::Library;
use crate::core::model::*;
use crate::core::plugins;
use crate::desktop::protocol::APP_ORIGIN;
use crate::desktop::protocol::preview_key;
use crate::desktop::protocol::asset_info;
use crate::desktop::protocol::serve_file_for;
use crate::desktop::protocol::browser_args;
use crate::desktop::config::own_data;
use crate::desktop::process::new_token;
use crate::desktop::api_host::window_call;

/// The origin a plugin's pages are served from, its own so that no two plugins share storage or can script each other's frames: `http://plugin.<label>.localhost` on Windows (WebView2 takes `http://plugin.*` to the `plugin` scheme), `plugin://<label>` elsewhere.
fn plugin_origin(id: &str) -> String {
    let label = plugins::pages::page_label(id);
    if cfg!(windows) {
        format!("http://plugin.{label}.localhost")
    } else {
        format!("plugin://{label}")
    }
}

/// Windows showing a Window plugin's page are labelled like this.
const PLUGIN_WINDOW_PREFIX: &str = "plugin";

/// The client every page gets, the same file scripts and extensions use.
const CURIO_JS: &str = include_str!("../core/api/client/curio.js");

/// A file plugin pages may fetch through `plugin_file`; past this they get an error rather than a copy in memory.
const FRAME_FILE_LIMIT: u64 = 512 << 20;

/// A plugin that may show pages now: on, approved, and loaded in this window's library.
fn page_plugin(state: &AppState, id: &str) -> R<plugins::manifest::Plugin> {
    let found = state.with_lib(|l| Ok(l.plugins().into_iter().find(|p| p.manifest.id == id && p.source == plugins::manifest::Source::Native)))?;
    match found {
        Some(p) if p.active() => Ok(p),
        Some(p) => Err(format!("The plugin {} is off{}", p.manifest.name, p.problem.map(|w| format!(": {w}")).unwrap_or_default())),
        None => Err(format!("There is no plugin {id} in this library")),
    }
}

fn plugin_caller(p: &plugins::manifest::Plugin) -> api::Caller {
    api::Caller::plugin(p)
}

/// A plugin page's way into the API: questions and dialogs open over the window showing the page; the rest is the main window's.
struct FrameHost {
    app: AppHost,
    label: String,
    plugin: String,
}

/// Tokens kept for `item.fileUrl` addresses; past this the oldest go.
const FRAME_TOKENS: usize = 4096;

/// A file name as one segment of an address.
fn url_segment(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

impl api::Host for FrameHost {
    fn library(&self) -> Option<PathBuf> {
        self.app.library()
    }
    fn me(&self) -> instance::Holder {
        self.app.me()
    }
    fn read(&self, f: &mut dyn FnMut(&Db)) -> R<()> {
        self.app.read(f)
    }
    fn with_library(&self, f: &mut dyn FnMut(&Library)) -> R<()> {
        self.app.with_library(f)
    }
    fn selection(&self) -> Vec<i64> {
        self.app.selection()
    }
    fn recent_folders(&self) -> Vec<i64> {
        self.app.recent_folders()
    }
    fn config(&self) -> api::ApiConfig {
        self.app.config()
    }
    fn change_library(&self, change: api::LibraryChange) -> R<PathBuf> {
        self.app.change_library(change)
    }
    fn activity(&self) -> Option<&api::Activity> {
        self.app.activity()
    }
    fn start_holder(&self, library: &Path) -> R<()> {
        self.app.start_holder(library)
    }
    fn events(&self) -> Option<Arc<api::events::Hub>> {
        self.app.events()
    }
    fn theme(&self) -> Option<serde_json::Value> {
        self.app.theme()
    }
    fn language(&self) -> String {
        self.app.language()
    }
    fn window(&self, op: &str, args: serde_json::Value, ask: bool) -> Option<api::ApiResult> {
        let own = self.label != "main" && (op.starts_with("ui.") || op.starts_with("dialog."));
        window_call(&self.app.0, if own { &self.label } else { "main" }, op, args, ask)
    }
    fn page_file(&self, item: AssetId, ext: &str) -> Option<String> {
        let st = self.app.0.state::<AppState>();
        let p = page_plugin(&st, &self.plugin).ok()?;
        let siblings = p.manifest.formats.iter().any(|f| f.exts.iter().any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(ext)));
        let path = st.read(|db| Ok(db.abs_path(item))).ok().flatten()?;
        let name = path.file_name()?.to_string_lossy().to_string();
        let seq = st.frame_token_seq.fetch_add(1, Ordering::Relaxed);
        let mut map = st.frame_tokens.lock();
        // the same item keeps its token, so a page's addresses for it stay the same and what the web view kept of them serves again
        if let Some((token, t)) = map.iter_mut().find(|(_, t)| t.plugin == self.plugin && t.item == item && t.siblings == siblings) {
            t.seq = seq;
            return Some(format!("{}/{}/~item/{token}/{}", plugin_origin(&self.plugin), self.plugin, url_segment(&name)));
        }
        let token = new_token();
        if map.len() >= FRAME_TOKENS {
            let floor = seq.saturating_sub(FRAME_TOKENS as u64 / 2);
            map.retain(|_, t| t.seq >= floor);
        }
        map.insert(token.clone(), FileToken { plugin: self.plugin.clone(), item, siblings, seq });
        Some(format!("{}/{}/~item/{token}/{}", plugin_origin(&self.plugin), self.plugin, url_segment(&name)))
    }
    fn page_write(&self, w: api::PageWrite) -> Option<String> {
        let st = self.app.0.state::<AppState>();
        let token = new_token();
        let mut map = st.write_tokens.lock();
        // addresses never used are dropped once there are many
        if map.len() >= FRAME_TOKENS {
            map.clear();
        }
        map.insert(token.clone(), (self.plugin.clone(), w));
        Some(format!("{}/{}/~write/{token}", plugin_origin(&self.plugin), self.plugin))
    }
}

/// A plugin page's call, handed over by the page showing it (web/src/plugins/bridge.js), which knows which frame asked.
/// Answers `{ok}` or `{error}`, so the page gets the error's code; a file comes back as a key for `plugin_file`.
#[tauri::command]
pub(super) async fn plugin_api(window: tauri::WebviewWindow, app: tauri::AppHandle, plugin: String, op: String, args: Option<serde_json::Value>) -> R<serde_json::Value> {
    let label = window.label().to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let st = app.state::<AppState>();
        let p = match page_plugin(&st, &plugin) {
            Ok(p) => p,
            Err(e) => return serde_json::json!({"error": api::ApiError::new("not_allowed", e)}),
        };
        let host = FrameHost { app: AppHost(app.clone()), label, plugin: plugin.clone() };
        let bytes = api::find(&op).is_some_and(|o| o.bytes);
        match api::handle(&host, &plugin_caller(&p), &op, args.unwrap_or_default()) {
            // only an operation that answers with a file can: another's answer may hold what the plugin stored, `$file` included
            Ok(v) => match api::as_file(&v).filter(|_| bytes) {
                Some((path, mime)) => {
                    let key = new_token();
                    st.frame_files.lock().insert(key.clone(), (path, mime.clone()));
                    serde_json::json!({"file": {"key": key, "type": mime}})
                }
                None => serde_json::json!({"ok": v}),
            },
            Err(e) => e.json(),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// The bytes of a file a plugin page's call answered with; each key works once.
#[tauri::command]
pub(super) async fn plugin_file(state: tauri::State<'_, AppState>, key: String) -> R<tauri::ipc::Response> {
    let (path, _) = state.frame_files.lock().remove(&key).ok_or("That file was already fetched")?;
    let len = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    if len > FRAME_FILE_LIMIT {
        return Err(format!("The file is {} MB; a plugin page can fetch up to {} MB", len >> 20, FRAME_FILE_LIMIT >> 20));
    }
    Ok(tauri::ipc::Response::new(std::fs::read(&path).map_err(|e| e.to_string())?))
}

/// Follows events for a plugin page: what its plugin may receive of this window's library, sent to the window showing the page as `plugin-event`.
#[tauri::command]
pub(super) async fn plugin_subscribe(window: tauri::WebviewWindow, app: tauri::AppHandle, plugin: String, sub: String, filter: serde_json::Value) -> R<serde_json::Value> {
    let label = window.label().to_string();
    let st = app.state::<AppState>();
    let fail = |code: &str, m: String| Ok(serde_json::json!({"error": api::ApiError::new(code, m)}));
    let p = match page_plugin(&st, &plugin) {
        Ok(p) => p,
        Err(e) => return fail("not_allowed", e),
    };
    let mut f: api::events::Filter = match serde_json::from_value(filter) {
        Ok(f) => f,
        Err(e) => return fail("bad_request", e.to_string()),
    };
    if f.library.is_some() {
        return fail("not_allowed", "A plugin follows the library of the window it was started from".into());
    }
    // the library is this process's, which may not be on the list, so it and the items are told apart here
    let ids: std::collections::HashSet<String> = std::mem::take(&mut f.ids).into_iter().collect();
    let host = AppHost(app.clone());
    let caller = plugin_caller(&p);
    let s = match api::events::subscription(&host, &caller, &f) {
        Ok(s) => s,
        Err(e) => return Ok(e.json()),
    };
    let rx = st.hub.subscribe(&api::events::Since::Now);
    let stop = Arc::new(AtomicBool::new(false));
    st.frame_subs.lock().insert(sub.clone(), (label.clone(), stop.clone()));
    std::thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) && app.get_webview_window(&label).is_some() {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(e) => {
                    let Some(mut v) = s.deliver(&host, &e) else { continue };
                    // re-check the open library and the plugin's state there, so switching libraries delivers `library.switched` and new events without leaking across disabled or unapproved plugins
                    let st = app.state::<AppState>();
                    let allowed = st.with_lib(|l| {
                        let own = (!l.ctx.library_id.is_empty()).then_some(l.ctx.library_id.as_str());
                        if e.library.is_some() && e.library.as_deref() != own {
                            return Ok(false);
                        }
                        let Some(p) = l.plugins().into_iter().find(|p| p.manifest.id == plugin && p.source == plugins::manifest::Source::Native && p.active()) else { return Ok(false) };
                        let caller = plugin_caller(&p);
                        Ok(api::events::EVENTS.iter().find(|x| x.0 == e.name).is_none_or(|x| caller.may(x.1)))
                    }).unwrap_or(false);
                    if !allowed {
                        continue;
                    }
                    if !ids.is_empty() && e.name.starts_with("items.") {
                        let mine: Vec<serde_json::Value> = v["data"]["ids"].as_array().into_iter().flatten().filter(|i| i.as_str().is_some_and(|i| ids.contains(i))).cloned().collect();
                        if mine.is_empty() {
                            continue;
                        }
                        v["data"]["ids"] = serde_json::json!(mine);
                    }
                    let _ = app.emit_to(label.as_str(), "plugin-event", serde_json::json!({"sub": sub, "to": label, "event": v}));
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(_) => break,
            }
        }
        app.state::<AppState>().frame_subs.lock().remove(&sub);
    });
    Ok(serde_json::json!({"ok": {}}))
}

/// Ends one subscription, or every subscription of the asking window when its page starts again.
#[tauri::command]
pub(super) fn plugin_unsubscribe(window: tauri::WebviewWindow, state: tauri::State<AppState>, sub: Option<String>) {
    let mut subs = state.frame_subs.lock();
    subs.retain(|id, (label, stop)| {
        let ends = label == window.label() && sub.as_ref().is_none_or(|s| s == id);
        if ends {
            stop.store(true, Ordering::Relaxed);
        }
        !ends
    });
}

/// Opens a Window plugin's page in a Curio window of its own, for `items` (their ids) when given; `run` is the rest of what its page hears in `plugin.run`, such as the action or the addresses it was opened for.
/// A plugin window that isn't `multiple` and is open already comes forward and hears `plugin.run` again.
#[tauri::command]
pub(super) async fn plugin_window(app: tauri::AppHandle, state: tauri::State<'_, AppState>, plugin: String, window: String, items: Option<Vec<String>>, run: Option<serde_json::Value>) -> R<()> {
    let p = page_plugin(&state, &plugin)?;
    let w = p.manifest.windows.iter().find(|w| w.id == window).ok_or_else(|| format!("{} has no window {window}", p.manifest.name))?;
    if !plugins::pages::page_id_ok(&w.id) {
        return Err(format!("{}'s window id {} can't be part of an address", p.manifest.name, w.id));
    }
    let mut run = match run {
        Some(serde_json::Value::Object(o)) => serde_json::Value::Object(o),
        _ => serde_json::json!({}),
    };
    run["window"] = serde_json::json!(w.id);
    if let Some(items) = items.filter(|i| !i.is_empty()) {
        run["items"] = serde_json::json!(items);
    }
    if !w.multiple {
        let open: Vec<String> = state.plugin_windows.lock().iter().filter(|(_, x)| x.plugin == plugin && x.window == window).map(|(l, _)| l.clone()).collect();
        for label in open {
            match app.get_webview_window(&label) {
                Some(existing) => {
                    raise(&existing);
                    if let Some(x) = state.plugin_windows.lock().get_mut(&label) {
                        x.run = run.clone();
                    }
                    let _ = app.emit_to(label.as_str(), "plugin-rerun", serde_json::json!({ "to": label, "run": run }));
                    return Ok(());
                }
                None => {
                    state.plugin_windows.lock().remove(&label);
                }
            }
        }
    }
    let label = format!("{PLUGIN_WINDOW_PREFIX}{}", fastrand::u32(..));
    state.plugin_windows.lock().insert(label.clone(), PluginWin { plugin: plugin.clone(), window: w.id.clone(), page: w.page.clone(), title: w.title.clone(), run });
    let url = format!("index.html?plugin={}&page={}", p.manifest.id, w.id);
    let built = crate::desktop::system::material::sized(crate::desktop::system::material::frame(own_data(tauri::WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::App(url.into())))), &app, w.width.unwrap_or(900).clamp(320, 4000) as f64, w.height.unwrap_or(640).clamp(240, 3000) as f64)
        .focused(!background())
        .always_on_bottom(background())
        .visible(false)
        .title(&w.title)
        .additional_browser_args(&browser_args())
        .min_inner_size(320.0, 240.0)
        .build()
        .map_err(|e| e.to_string())?;
    crate::desktop::system::material::apply(&built, saved_look());
    reveal_fallback(&built);
    Ok(())
}

/// What a Window plugin's window shows: the plugin, its page and title, and what it was opened for.
#[tauri::command]
pub(super) fn plugin_window_info(window: tauri::WebviewWindow, state: tauri::State<AppState>) -> Option<serde_json::Value> {
    let w = state.plugin_windows.lock().get(window.label()).cloned()?;
    Some(serde_json::json!({ "plugin": w.plugin, "window": w.window, "page": w.page, "title": w.title, "run": w.run }))
}

/// The theme the main window applied; a change after the first is told as `app.themeChanged`.
#[tauri::command]
pub(super) fn theme_shown(window: tauri::WebviewWindow, state: tauri::State<AppState>, theme: serde_json::Value) {
    if window.label() != "main" {
        return;
    }
    let before = state.theme.lock().replace(theme.clone());
    if before.is_some_and(|b| b != theme) {
        let mut data = theme;
        data["window"] = serde_json::json!(state.me.lock().window.clone());
        state.hub.publish("app.themeChanged", None, data);
    }
}

fn page_response(status: u16, mime: &str, body: Vec<u8>, csp: Option<String>) -> tauri::http::Response<Vec<u8>> {
    let mut b = tauri::http::Response::builder()
        .status(status)
        .header("Content-Type", mime)
        // connect-src keeps other plugins' pages from fetching these
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "no-cache")
        .header("X-Content-Type-Options", "nosniff");
    if let Some(c) = csp {
        b = b.header("Content-Security-Policy", c);
    }
    b.body(body).unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
}

/// Item files reached through `item.fileUrl` never run as pages of the plugin's origin.
const ITEM_FILE_CSP: &str = "sandbox; default-src 'none'; img-src * data: blob:; media-src * data: blob:; style-src * 'unsafe-inline'; font-src * data:";

/// `<origin>/<id>/~item/<token>/<file>`: an item's file, or one beside it, for the plugin the token was made for.
fn serve_item_file(app: &tauri::AppHandle, id: &str, rest: &str, range: Option<&str>) -> tauri::http::Response<Vec<u8>> {
    let missing = || page_response(404, "text/plain", b"not found".to_vec(), None);
    let (token, rel) = rest.split_once('/').unwrap_or((rest, ""));
    let Some(t) = app.state::<AppState>().frame_tokens.lock().get(token).cloned() else { return missing() };
    if t.plugin != id {
        return missing();
    }
    if let Some(size) = rel.strip_prefix("~preview/") {
        return kept(serve_item_preview(app, t.item, size.parse().unwrap_or(0)));
    }
    // `archive.entryUrl`: an entry of the item's archive, a picture scaled when a size is given
    if let Some(entry) = rel.strip_prefix("~entry/") {
        let (size, name) = entry.split_once('/').unwrap_or(("0", entry));
        let Some(name) = plugins::pages::percent_decode(name) else { return missing() };
        return kept(serve_archive_entry(app, t.item, &name, size.parse().unwrap_or(0)));
    }
    // a file grouped into the item, by its name, wherever it is and whatever the page's formats; a model names its textures with the folders they were in when it was made
    let wanted = plugins::pages::percent_decode(rel).unwrap_or_default().to_lowercase();
    let wanted = wanted.rsplit('/').next().unwrap_or_default().to_string();
    let member = app.state::<AppState>().read(|db| Ok(db.members_of(t.item)?.into_iter().filter_map(|p| db.abs_path(p.id)).find(|f| f.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase() == wanted)))).ok().flatten();
    let Some(file) = app.state::<AppState>().read(|db| Ok(db.abs_path(t.item))).ok().flatten() else { return missing() };
    let (Some(dir), Ok(own)) = (file.parent(), file.canonicalize()) else { return missing() };
    let Some(file) = member.filter(|f| f.is_file()).or_else(|| plugins::pages::page_file(dir, rel).filter(|f| *f == own || (t.siblings && companion(app, &t.plugin, rel, f)))) else { return missing() };
    let mut r = serve_file_for(&file, plugins::pages::page_mime(&file), range, "*");
    if let Ok(v) = tauri::http::HeaderValue::from_str(ITEM_FILE_CSP) {
        r.headers_mut().insert("Content-Security-Policy", v);
    }
    r
}

/// A file a format's page may load from beside its item, as a model's textures and materials: never in a hidden folder or hidden itself, at most three folders down, and of a kind such a page needs.
fn companion(app: &tauri::AppHandle, plugin: &str, rel: &str, file: &Path) -> bool {
    let rel = plugins::pages::percent_decode(rel).unwrap_or_default();
    let segs: Vec<&str> = rel.split('/').collect();
    if segs.len() > 4 || segs.iter().any(|s| s.starts_with('.')) {
        return false;
    }
    let ext = file.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if matches!(ext.as_str(), "mtl" | "bin" | "tga" | "dds" | "ktx" | "ktx2" | "hdr" | "exr") || kind_for_ext(&ext) == AssetKind::Image {
        return true;
    }
    page_plugin(&app.state::<AppState>(), plugin).is_ok_and(|p| p.manifest.formats.iter().any(|f| f.exts.iter().any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(&ext))))
}

/// A successful answer the web view may keep: its address names the item's version (`?v=`), so a changed file gets another address.
fn kept(mut r: tauri::http::Response<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    if r.status() == 200 {
        r.headers_mut().insert("Cache-Control", tauri::http::HeaderValue::from_static("private, max-age=31536000, immutable"));
    }
    r
}

/// `archive.entryUrl`: one entry; a picture asked for at `size` comes scaled, made when asked for, unless it is smaller already or moves, which come as they are.
fn serve_archive_entry(app: &tauri::AppHandle, item: AssetId, name: &str, size: u32) -> tauri::http::Response<Vec<u8>> {
    let st = app.state::<AppState>();
    let Some((abs, _, _, uid, ver, _)) = asset_info(&st, item) else { return page_response(404, "text/plain", b"not found".to_vec(), None) };
    let file = Path::new(&abs);
    let as_is = |bytes: Vec<u8>| page_response(200, plugins::pages::page_mime(Path::new(name)), bytes, Some(ITEM_FILE_CSP.into()));
    if size == 0 || !crate::core::archive::is_picture(name) {
        return match crate::core::archive::read(file, name) {
            Ok(b) => as_is(b),
            Err(_) => page_response(404, "text/plain", b"not found".to_vec(), None),
        };
    }
    let size = size.clamp(64, 8192);
    let mut original = None;
    let scaled = crate::core::thumbs::previews::bytes_preview(
        &format!("{}|{name}", preview_key(&st, &uid, ver)),
        || {
            let bytes = crate::core::archive::read(file, name)?;
            // pictures Curio can't scale (AVIF, JPEG XL) go as they are, without reading them a second time
            let unscaled = name.rsplit('.').next().is_some_and(|e| e.eq_ignore_ascii_case("avif") || e.eq_ignore_ascii_case("jxl"));
            match crate::core::archive::head_look(&bytes) {
                _ if unscaled => {
                    original = Some(bytes);
                    Err(String::new())
                }
                Some(l) if l.animated || l.width.max(l.height) <= size => {
                    original = Some(bytes);
                    Err(String::new())
                }
                _ => Ok(bytes),
            }
        },
        size,
    );
    match (scaled, original) {
        (_, Some(bytes)) => as_is(bytes),
        (Ok(b), None) => page_response(200, if b.starts_with(b"\x89PNG") { "image/png" } else { "image/jpeg" }, b.to_vec(), Some(ITEM_FILE_CSP.into())),
        // a picture Curio can't scale (AVIF, JPEG XL) still shows: the web view draws it at full size
        (Err(_), None) => match crate::core::archive::read(file, name) {
            Ok(b) => as_is(b),
            Err(_) => page_response(404, "text/plain", b"not found".to_vec(), None),
        },
    }
}

/// `item.previewUrl`: the item's picture at `size` on its long edge, made by the viewer's own scaler when asked for.
fn serve_item_preview(app: &tauri::AppHandle, item: AssetId, size: u32) -> tauri::http::Response<Vec<u8>> {
    let st = app.state::<AppState>();
    let Some((abs, ext, kind, uid, ver, _)) = asset_info(&st, item) else { return page_response(404, "text/plain", b"not found".to_vec(), None) };
    let size = size.clamp(256, 8192);
    let handlers = st.handlers.read().clone();
    let tmp = st.tmp_dir.read().clone().unwrap_or_else(std::env::temp_dir);
    match crate::core::thumbs::previews::preview(&preview_key(&st, &uid, ver), Path::new(&abs), &ext, kind, size, &handlers, &tmp) {
        Ok(b) => {
            let b = b.to_vec();
            let mime = if b.starts_with(b"\x89PNG") { "image/png" } else { "image/jpeg" };
            page_response(200, mime, b, Some(ITEM_FILE_CSP.into()))
        }
        Err(e) => page_response(415, "text/plain", e.into_bytes(), None),
    }
}

/// `<origin>/<id>/~write/<token>`: a file a page sends with PUT, put where `item.writeUrl` said; the answer is the call's.
fn receive_page_write(app: &tauri::AppHandle, id: &str, token: &str, label: &str, method: &str, body: Vec<u8>) -> tauri::http::Response<Vec<u8>> {
    let json = |status: u16, v: serde_json::Value| {
        let mut r = page_response(status, "application/json", v.to_string().into_bytes(), None);
        r.headers_mut().insert("Access-Control-Allow-Methods", tauri::http::HeaderValue::from_static("PUT"));
        r.headers_mut().insert("Access-Control-Allow-Headers", tauri::http::HeaderValue::from_static("content-type"));
        r
    };
    // a page's PUT is cross-origin, as its frame's origin is opaque, so the web view asks first
    if method == "OPTIONS" {
        let mut r = json(204, serde_json::json!({}));
        r.body_mut().clear();
        return r;
    }
    let fail = |e: api::ApiError| json(e.status(), e.json());
    if method != "PUT" {
        return fail(api::ApiError::new("bad_request", "Send the file with PUT"));
    }
    let st = app.state::<AppState>();
    let found = st.write_tokens.lock().remove(token).filter(|(p, _)| p == id);
    let Some((plugin, w)) = found else { return fail(api::ApiError::new("not_found", "That address was used already, or never made")) };
    let p = match page_plugin(&st, &plugin) {
        Ok(p) => p,
        Err(e) => return fail(api::ApiError::new("not_allowed", e)),
    };
    let ext = match &w.name {
        Some(n) if w.beside => Path::new(n).extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default(),
        _ => st.read(|db| Ok(db.asset(w.item)?.map(|a| a.ext))).ok().flatten().unwrap_or_default(),
    };
    let Some(tmp) = st.tmp_dir.read().clone() else { return fail(api::ApiError::new("library_closed", "No library is open")) };
    let file = tmp.join(format!("page-write-{}.{ext}", fastrand::u64(..)));
    if let Err(e) = std::fs::create_dir_all(&tmp).and_then(|_| std::fs::write(&file, body)) {
        return fail(api::ApiError::new("failed", e.to_string()));
    }
    let host = FrameHost { app: AppHost(app.clone()), label: label.to_string(), plugin: plugin.clone() };
    let r = api::finish_page_write(&host, &plugin_caller(&p), &w, &file);
    let _ = std::fs::remove_file(&file);
    match r {
        Ok(v) => json(200, serde_json::json!({"ok": v})),
        Err(e) => fail(e),
    }
}

/// `<origin>/curio/ui/<file>`: the client, opt-in stylesheet and scrollbars, under every plugin's origin; `<origin>/<id>/<file>`: a plugin's own files, its pages under its policy, only under its own origin (`plugin_origin`).
pub(super) fn serve_plugin(app: &tauri::AppHandle, host: &str, path: &str, range: Option<&str>, label: &str, method: &str, body: Vec<u8>) -> tauri::http::Response<Vec<u8>> {
    let missing = || page_response(404, "text/plain", b"not found".to_vec(), None);
    let path = path.trim_start_matches('/');
    if let Some(file) = path.strip_prefix("curio/ui/") {
        if file == "curio.js" {
            return page_response(200, "text/javascript; charset=utf-8", CURIO_JS.as_bytes().to_vec(), None);
        }
        if !file.split('/').all(plugins::pages::page_id_ok) {
            return missing();
        }
        return match app.asset_resolver().get(format!("plugin/{file}")) {
            Some(a) => page_response(200, plugins::pages::page_mime(Path::new(file)), a.bytes().to_vec(), None),
            None => missing(),
        };
    }
    let (id, rest) = path.split_once('/').unwrap_or((path, ""));
    if host.strip_suffix(".localhost").unwrap_or(host) != plugins::pages::page_label(id) {
        return missing();
    }
    let Ok(p) = page_plugin(&app.state::<AppState>(), id) else { return missing() };
    if let Some(item) = rest.strip_prefix("~item/") {
        return serve_item_file(app, id, item, range);
    }
    if let Some(token) = rest.strip_prefix("~write/") {
        return receive_page_write(app, id, token, label, method, body);
    }
    let Some(file) = plugins::pages::page_file(&p.dir, rest) else { return missing() };
    let Ok(body) = std::fs::read(&file) else { return missing() };
    let mime = plugins::pages::page_mime(&file);
    let network = p.has("network").then_some(p.manifest.network.as_slice());
    let origin = plugin_origin(id);
    let csp = if mime.contains("svg") || mime.contains("xml") {
        Some(ITEM_FILE_CSP.to_string())
    } else {
        Some(plugins::pages::page_csp(&format!("{origin}/{id}/"), &format!("{origin}/curio/ui/"), network, APP_ORIGIN))
    };
    page_response(200, mime, body, csp)
}

/// The largest answer a page extractor may give for one file, as JSON.
const MAX_PAGE_ANSWER: usize = 4 << 20;

/// Files waiting for a plugin page that reads them, while a window is open (web/src/plugins/pageJobs.js).
#[tauri::command]
pub(super) async fn page_reads(state: tauri::State<'_, AppState>, limit: usize, skip: Option<Vec<(i64, String)>>) -> R<Vec<serde_json::Value>> {
    let readers: Vec<crate::core::metadata::readers::Reader> = state.with_lib(|l| Ok(l.contributions().readers))?.into_iter().filter(|r| r.page().is_some()).collect();
    let ids: Vec<String> = readers.iter().map(|r| r.id.clone()).collect();
    let skip = skip.unwrap_or_default();
    state.read(|db| {
        let mut out = Vec::new();
        for (id, reader, mode) in db.pending_queue(&ids, limit, &skip)? {
            let (Some(a), Some(r)) = (db.asset(id)?, readers.iter().find(|r| r.id == reader)) else { continue };
            let parts: Vec<serde_json::Value> = if a.parts > 0 { db.members_of(id)?.into_iter().filter(|p| p.status == Availability::Ok).map(|p| serde_json::json!({ "role": p.role, "name": p.name, "uid": p.uid, "size": p.size })).collect() } else { vec![] };
            out.push(serde_json::json!({ "id": id, "uid": a.uid, "reader": reader, "mode": mode as i64, "plugin": r.owner, "page": r.page(), "ext": a.ext, "name": a.name, "size": a.size, "parts": parts }));
        }
        Ok(out)
    })
}

/// What a plugin page read from a file, or why it couldn't.
#[tauri::command]
pub(super) fn page_read_done(state: tauri::State<AppState>, id: i64, reader: String, mode: i64, values: Option<serde_json::Map<String, serde_json::Value>>, error: Option<String>, timed_out: bool) -> R<()> {
    use crate::core::metadata::readers::{Mode, Outcome};
    let outcome = match (values, error) {
        _ if timed_out => Outcome::Failed("The page didn't answer within a minute".into()),
        (_, Some(e)) => Outcome::Failed(e.chars().take(1000).collect()),
        (Some(v), None) if serde_json::to_vec(&v).map_or(0, |b| b.len()) > MAX_PAGE_ANSWER => Outcome::Failed("The page answered more than 4 MB".into()),
        (v, None) => Outcome::Read(v.unwrap_or_default().iter().map(|(k, j)| (k.clone(), crate::core::plugins::run::json_to_raw(j))).collect()),
    };
    state.with_lib(|l| {
        l.page_answered(crate::core::library::metadata::PageAnswer { id, reader, mode: Mode::from_db(mode), outcome });
        Ok(())
    })
}
