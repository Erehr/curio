//! Desktop shell (Tauri). The window is a Vue + PrimeVue front end in `web/`; this module
//! exposes the core engine to it as commands, streams background events, and serves
//! thumbnails/previews through the `thumb://` protocol (never through the file system API,
//! so the web view gets no general file access).

mod app_windows;
mod edits;
mod files;
mod fonts;
mod import;
mod libraries;
mod maintenance;
mod media;
mod plugin_manager;
mod reads;
mod settings;
mod tools;
mod viewer_page;
mod api_host;
mod config;
mod plugin_pages;
mod process;
mod protocol;
pub mod locale;
pub mod system;

use crate::desktop::api_host::AppHost;
use crate::desktop::protocol::WEB_WINDOW_PREFIX;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::core::api;
use crate::core::library::instance;
use crate::core::db::Db;
use crate::core::library::{changes::Changed, settings::HashPolicy, Library};
use crate::core::system::logging;
use crate::core::model::*;

type R<T> = Result<T, String>;

#[derive(Clone, Debug, Default, Serialize)]
pub struct StartOptions {
    pub library: Option<PathBuf>,
    pub safe_mode: bool,
    /// Start on the welcome page instead of reopening the last library.
    pub no_library: bool,
    /// Started by the process it replaces, which may not have finished exiting.
    pub relaunch: bool,
    /// Files named on the command line, added to the library that opens.
    pub files: Vec<PathBuf>,
    /// Start with the window kept in the tray (starting with the system).
    pub tray: bool,
    /// Hold `library` for the API without a window, until idle or until someone opens it (see `show_main`).
    pub headless: bool,
    /// A `curio:` link this start came from; it was the only argument kept.
    #[serde(skip)]
    pub link: Option<crate::desktop::system::scheme::Link>,
}

pub struct AppState {
    lib: Mutex<Option<Library>>,
    /// Keeps this library to this window (see core/library/instance.rs).
    instance: Mutex<Option<instance::InstanceLock>>,
    me: Mutex<instance::Holder>,
    readers: Mutex<Vec<Db>>,
    db_path: RwLock<Option<PathBuf>>,
    thumbs_dir: RwLock<Option<PathBuf>>,
    tmp_dir: RwLock<Option<PathBuf>>,
    previews_dir: RwLock<Option<PathBuf>>,
    handlers: RwLock<Vec<crate::core::thumbs::ExternalHandler>>,
    opts: StartOptions,
    lib_gen: Mutex<u64>,
    /// current selection in the window, for the API's `selection.get`
    pub selection: Mutex<Vec<i64>>,
    /// set while this process answers on the API's port
    api: Mutex<Option<api::http::Server>>,
    activity: api::Activity,
    /// A background holder giving its library up; it answers nothing more.
    ending: AtomicBool,
    /// This library's events for the API (see core/api/events.rs).
    hub: Arc<api::events::Hub>,
    /// API calls waiting for the window's answer (see `AppHost::window`).
    window_calls: Mutex<HashMap<u64, crossbeam_channel::Sender<api::ApiResult>>>,
    window_seq: AtomicU64,
    /// The API asks the person one thing at a time.
    asking: Mutex<()>,
    /// One library opens or closes at a time: two switches side by side would each set half of the state.
    switching: Mutex<()>,
    /// The item the viewer shows, and the file: its own or one grouped into it; the last one for `viewer.closed`.
    viewer_shown: Mutex<Option<(String, String)>>,
    /// The main window's page listens for the API's calls (see web/src/app/windowRequests.js).
    window_ready: AtomicBool,
    /// What the main window shows, for `app.theme` and plugin pages.
    theme: Mutex<Option<serde_json::Value>>,
    /// Files plugin pages asked for, until their bytes are fetched (see `plugin_api`).
    frame_files: Mutex<HashMap<String, (PathBuf, String)>>,
    /// Plugin pages' event subscriptions, by id, with the window each is for.
    frame_subs: Mutex<HashMap<String, (String, Arc<AtomicBool>)>>,
    /// Item files plugin pages read by address (`item.fileUrl`), by token.
    frame_tokens: Mutex<HashMap<String, FileToken>>,
    frame_token_seq: AtomicU64,
    /// Addresses plugin pages send one file to (`item.writeUrl`): the plugin and where the file goes, by token.
    write_tokens: Mutex<HashMap<String, (String, api::PageWrite)>>,
    /// Window plugins' windows by label: what each shows and what it was opened for.
    plugin_windows: Mutex<HashMap<String, PluginWin>>,
    /// Tool windows by label: which tool each shows and what it was opened for.
    tool_windows: Mutex<HashMap<String, ToolWin>>,
}

/// What an `item.fileUrl` address reaches: the item's file wherever it is now, and the files beside it when the plugin reads that format.
#[derive(Clone)]
struct FileToken {
    plugin: String,
    item: AssetId,
    siblings: bool,
    seq: u64,
}

/// A tool's window.
#[derive(Clone)]
struct ToolWin {
    tool: String,
    run: serde_json::Value,
}

/// A Window plugin's window.
#[derive(Clone)]
struct PluginWin {
    plugin: String,
    window: String,
    page: String,
    title: String,
    run: serde_json::Value,
}

use crate::core::system::app_dirs::config_dir;
use crate::desktop::process::finish_downloads;
use crate::desktop::process::end_when_idle;
use crate::desktop::api_host::keep_claiming_api_port;
use crate::desktop::process::start_control_server;
use crate::desktop::plugin_pages::serve_plugin;
use crate::desktop::protocol::serve;
use crate::desktop::protocol::http_response;
use crate::desktop::process::new_token;
use crate::desktop::config::ensure_installation_id;
use crate::desktop::process::RELAUNCH_WAIT;
use crate::desktop::config::load_config;
use crate::desktop::app_windows::build_main_window;
use crate::desktop::import::add_files;
use crate::desktop::libraries::open_library_inner;
use crate::desktop::app_windows::keeps_running;

/// Set by `--background` and inherited by the windows it starts: they open behind the others and never take the focus, so test runs don't cover what the user is working in.
pub const BACKGROUND_ENV: &str = "CURIO_BACKGROUND";

pub(crate) fn background() -> bool {
    std::env::var_os(BACKGROUND_ENV).is_some()
}

#[derive(Serialize, Deserialize, Clone)]
struct AppConfig {
    #[serde(default)]
    libraries: Vec<PathBuf>,
    #[serde(default)]
    current_library: Option<PathBuf>,
    #[serde(default)]
    plugin_dirs: Vec<PathBuf>,
    #[serde(default)]
    cache_dir: Option<PathBuf>,
    /// User-picked CSS file overlaid on the active theme (set only via `pick_theme_file`).
    #[serde(default)]
    theme_file: Option<PathBuf>,
    /// GPU acceleration of the web view (on by default; off adds --disable-gpu, needs restart)
    #[serde(default = "yes")]
    gpu: bool,
    /// The API for scripts and extensions (on by default; every call needs a token)
    #[serde(default = "yes")]
    api_enabled: bool,
    #[serde(default, alias = "mcp_port")]
    api_port: Option<u16>,
    #[serde(default)]
    api_tokens: Vec<api::tokens::ApiToken>,
    /// MCP at the API's address (off by default)
    #[serde(default)]
    mcp_enabled: bool,
    /// MCP tools turned off in Settings
    #[serde(default)]
    mcp_disabled: Vec<String>,
    /// fetch previews of web links (off by default: a scanned link file must not make requests by itself)
    #[serde(default)]
    link_previews: bool,
    /// identifies this computer's installation; made on first start
    #[serde(default)]
    installation_id: String,
    /// closing a library's window keeps it running in the tray (on by default)
    #[serde(default = "yes")]
    close_to_tray: bool,
    /// each library's own icon and colour in the library lists, by its folder
    #[serde(default)]
    library_looks: std::collections::BTreeMap<PathBuf, LibraryLook>,
    /// each library's id, learnt when it was last open, so API callers can name libraries that aren't open
    #[serde(default)]
    library_ids: std::collections::BTreeMap<PathBuf, String>,
    /// `curio://` links from the browser start Curio unless this is set (Settings › API)
    #[serde(default)]
    no_browser_start: bool,
    #[serde(flatten, default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    extra: std::collections::BTreeMap<String, serde_json::Value>,
    /// Known fields whose stored value didn't parse in this build, kept intact on save unless changed.
    #[serde(skip)]
    raw_fallbacks: std::collections::BTreeMap<String, serde_json::Value>,
    /// Token entries from a newer format that didn't parse, kept intact on save.
    #[serde(skip)]
    raw_tokens: Vec<serde_json::Value>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            libraries: Vec::new(),
            current_library: None,
            plugin_dirs: Vec::new(),
            cache_dir: None,
            theme_file: None,
            gpu: true,
            api_enabled: true,
            api_port: None,
            api_tokens: Vec::new(),
            mcp_enabled: false,
            mcp_disabled: Vec::new(),
            link_previews: false,
            installation_id: String::new(),
            close_to_tray: true,
            library_looks: std::collections::BTreeMap::new(),
            library_ids: std::collections::BTreeMap::new(),
            no_browser_start: false,
            extra: std::collections::BTreeMap::new(),
            raw_fallbacks: std::collections::BTreeMap::new(),
            raw_tokens: Vec::new(),
        }
    }
}

impl AppConfig {
    fn api_port(&self) -> u16 {
        self.api_port.unwrap_or(api::DEFAULT_PORT)
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct LibraryLook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    extra: std::collections::BTreeMap<String, serde_json::Value>,
}

fn yes() -> bool {
    true
}

impl AppState {
    /// The open library's id, once it has one.
    fn library_id(&self) -> Option<String> {
        self.lib.lock().as_ref().map(|l| l.ctx.library_id.clone()).filter(|i| !i.is_empty())
    }

    fn with_lib<T>(&self, f: impl FnOnce(&Library) -> R<T>) -> R<T> {
        let g = self.lib.lock();
        let lib = g.as_ref().ok_or("No library is open")?;
        f(lib)
    }

    /// Run a read on a pooled connection (keeps the UI's reads off the writer).
    fn read<T>(&self, f: impl FnOnce(&Db) -> R<T>) -> R<T> {
        let db = match self.readers.lock().pop() {
            Some(d) => d,
            None => {
                let p = self.db_path.read().clone().ok_or("No library is open")?;
                Db::open(&p)?
            }
        };
        let out = f(&db);
        let mut pool = self.readers.lock();
        if pool.len() < 6 && self.db_path.read().as_deref() == Some(db.path.as_path()) {
            pool.push(db);
        }
        out
    }

    /// Run an edit on the writer and wait for its result, without holding the library lock meanwhile.
    /// The result is handed over only once the edit is committed.
    fn write_wait<T: Send + 'static>(&self, f: impl FnOnce(&mut Db) -> R<T> + Send + 'static) -> R<T> {
        let slot = Arc::new(Mutex::new(None));
        let put = slot.clone();
        let done = self.with_lib(|lib| {
            lib.write(move |db, _| {
                *put.lock() = Some(f(db));
                Ok(Changed::default())
            });
            Ok(lib.after_writes())
        })?;
        let lost = || "The library closed before the edit was saved".to_string();
        done.recv().map_err(|_| lost())?;
        let r = slot.lock().take();
        r.ok_or_else(lost)?
    }

    fn write(&self, f: impl FnOnce(&mut Db) -> R<Changed> + Send + 'static) -> R<()> {
        self.with_lib(|lib| {
            lib.write(move |db, _| f(db));
            Ok(())
        })
    }

    /// An edit the window waits for, so one refused reaches it as an error and what it showed at once is put back; one still waiting its turn after a while is taken as made.
    async fn write_answered(&self, f: impl FnOnce(&mut Db) -> R<Changed> + Send + 'static) -> R<()> {
        let rx = self.with_lib(|lib| Ok(lib.write_reply(move |db, _| f(db).map(|c| ((), c)))))?;
        tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(std::time::Duration::from_secs(30)).unwrap_or(Ok(()))).await.map_err(|e| e.to_string())?
    }

    /// Previews in the viewer use the plugins' thumbnail handlers as they are now.
    fn refresh_handlers(&self) {
        if let Ok(h) = self.with_lib(|lib| Ok(lib.contributions().thumb_handlers)) {
            *self.handlers.write() = h;
        }
    }
}

// ------------------------------------------------------------------ run

/// The API sends calls that name no library to the most recently focused window.
fn note_focus(app: &tauri::AppHandle) {
    let st = app.state::<AppState>();
    let me = {
        let mut me = st.me.lock();
        me.focused = now_ms();
        me.clone()
    };
    if let Some(lock) = st.instance.lock().as_ref() {
        lock.update(&me);
    };
}

/// A library folder, or a file inside one, opens that library; other files are added to the one that opens.
fn sort_file_args(opts: &mut StartOptions) {
    for f in std::mem::take(&mut opts.files) {
        match f.ancestors().find(|a| instance::is_library(a)) {
            Some(lib) if opts.library.is_none() => opts.library = Some(lib.to_path_buf()),
            Some(_) => {}
            None => opts.files.push(f),
        }
    }
}

/// What a `curio:` link changes in this start; false when there is nothing to start.
fn follow_link(opts: &mut StartOptions) -> bool {
    let Some(link) = opts.link.clone() else { return true };
    let cfg = load_config();
    // a package's system-wide handler stays registered when the switch is off
    if cfg.no_browser_start {
        return false;
    }
    match link {
        crate::desktop::system::scheme::Link::Open { background: true } => {
            if api::http::answers_ping(cfg.api_port()) {
                return false;
            }
            opts.tray = true;
        }
        crate::desktop::system::scheme::Link::Library(id) => opts.library = cfg.library_ids.iter().find(|(_, v)| v.eq_ignore_ascii_case(&id)).map(|(p, _)| p.clone()),
        crate::desktop::system::scheme::Link::Open { background: false } => {}
    }
    true
}

pub fn run(mut opts: StartOptions) {
    if !follow_link(&mut opts) {
        return;
    }
    logging::init_file(&config_dir().join("curio.log"));
    logging::log_panics();
    crate::core::model::set_installation_id(ensure_installation_id());
    crate::core::thumbs::links::set_link_previews(load_config().link_previews);
    sort_file_args(&mut opts);
    if opts.headless {
        match &opts.library {
            Some(lib) if instance::is_library(lib) && instance::holder(lib).is_none() => {}
            _ => return,
        }
    }
    if !opts.headless && !load_config().no_browser_start {
        // also written again when the program has moved
        std::thread::spawn(|| {
            if let Err(e) = crate::desktop::system::scheme::set(true) {
                logging::warn(&format!("couldn't register curio:// links: {e}"));
            }
        });
    }
    let start_lib = opts.library.clone().or_else(|| if opts.no_library { None } else { load_config().current_library });
    if let (Some(lib), false) = (&start_lib, opts.headless) {
        if opts.relaunch {
            instance::wait_until_free(lib, RELAUNCH_WAIT);
        }
        // started again while its window is open, or kept in the tray: that window comes back
        if let Some(h) = instance::holder(lib) {
            if opts.tray {
                return;
            }
            let files: Vec<String> = opts.files.iter().map(|p| p.to_string_lossy().to_string()).collect();
            let added = files.is_empty() || instance::tell(&h, "add", &serde_json::json!({ "paths": files })).is_ok();
            if added && instance::tell(&h, "focus", &serde_json::json!({})).is_ok() {
                return;
            }
            // a background holder ending just now: this window takes the library once it has
            if h.background {
                instance::wait_gone(lib, h.pid, RELAUNCH_WAIT);
            }
        }
    }
    let state = AppState {
        lib: Mutex::new(None),
        instance: Mutex::new(None),
        me: Mutex::new(instance::Holder { pid: std::process::id(), port: 0, token: new_token(), background: opts.headless, window: format!("{:08x}", fastrand::u32(..)), ..Default::default() }),
        readers: Mutex::new(Vec::new()),
        db_path: RwLock::new(None),
        thumbs_dir: RwLock::new(None),
        tmp_dir: RwLock::new(None),
        previews_dir: RwLock::new(None),
        handlers: RwLock::new(Vec::new()),
        opts: opts.clone(),
        lib_gen: Mutex::new(0),
        selection: Mutex::new(Vec::new()),
        api: Mutex::new(None),
        activity: api::Activity::default(),
        hub: Default::default(),
        ending: AtomicBool::new(false),
        window_calls: Mutex::new(HashMap::new()),
        window_seq: AtomicU64::new(1),
        asking: Mutex::new(()),
        switching: Mutex::new(()),
        viewer_shown: Mutex::new(None),
        window_ready: AtomicBool::new(false),
        theme: Mutex::new(None),
        frame_files: Mutex::new(HashMap::new()),
        frame_subs: Mutex::new(HashMap::new()),
        frame_tokens: Mutex::new(HashMap::new()),
        frame_token_seq: AtomicU64::new(1),
        write_tokens: Mutex::new(HashMap::new()),
        plugin_windows: Mutex::new(HashMap::new()),
        tool_windows: Mutex::new(HashMap::new()),
    };
    let result = tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_notification::init())
        .register_asynchronous_uri_scheme_protocol("thumb", |ctx, req, responder| {
            if ctx.webview_label().starts_with(WEB_WINDOW_PREFIX) {
                responder.respond(http_response(403, "text/plain", vec![]));
                return;
            }
            let app = ctx.app_handle().clone();
            let path = req.uri().path().to_string();
            let range = req.headers().get("range").and_then(|v| v.to_str().ok()).map(String::from);
            std::thread::spawn(move || responder.respond(serve(&app, &path, range.as_deref())));
        })
        .register_asynchronous_uri_scheme_protocol("plugin", |ctx, req, responder| {
            // a service worker would outlive its page and answer for the plugin's files after they changed or it was switched off
            if ctx.webview_label().starts_with(WEB_WINDOW_PREFIX) || req.headers().contains_key("service-worker") {
                responder.respond(http_response(403, "text/plain", vec![]));
                return;
            }
            let app = ctx.app_handle().clone();
            let label = ctx.webview_label().to_string();
            let host = req.uri().host().unwrap_or_default().to_string();
            let path = req.uri().path().to_string();
            let range = req.headers().get("range").and_then(|v| v.to_str().ok()).map(String::from);
            let method = req.method().as_str().to_string();
            let body = req.into_body();
            std::thread::spawn(move || responder.respond(serve_plugin(&app, &host, &path, range.as_deref(), &label, &method, body)));
        })
        .on_window_event(|w, e| {
            if let tauri::WindowEvent::Focused(false) = e {
                crate::desktop::system::material::deactivated(w);
            }
            if let tauri::WindowEvent::Focused(true) = e {
                if w.label() == "main" {
                    note_focus(w.app_handle());
                }
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                if w.label() == "main" && keeps_running(w.app_handle()) {
                    api.prevent_close();
                    let (app, w) = (w.app_handle().clone(), w.clone());
                    // claiming the tray may wait for the main thread, which runs this handler
                    std::thread::spawn(move || {
                        let _ = if crate::desktop::system::tray::claim(&app) == crate::desktop::system::tray::Tray::None { w.destroy() } else { w.hide() };
                    });
                }
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            #[cfg(any(windows, target_os = "linux"))]
            {
                let h = handle.clone();
                crate::core::media::set_page_capturer(Box::new(move |url, out| crate::desktop::system::webshot::capture(&h, url, out)));
            }
            #[cfg(target_os = "macos")]
            crate::desktop::system::dock::follow_windows(&handle);
            crate::desktop::system::notifications::install(&handle);
            start_control_server(&handle);
            if !opts.headless {
                crate::desktop::system::tray::start(&handle);
            }
            #[cfg(target_os = "linux")]
            protocol::start_media_server(&handle);
            if let Some(p) = start_lib.clone() {
                let st = handle.state::<AppState>();
                match open_library_inner(&handle, &st, p.clone(), false) {
                    Ok(_) if !opts.files.is_empty() => {
                        if let Err(e) = add_files(&st, opts.files.clone(), "window") {
                            logging::error(&e);
                        }
                    }
                    Ok(_) => {}
                    Err(e) if opts.headless => {
                        logging::error(&format!("could not open {} in the background: {e}", instance::library_name(&p)));
                        std::process::exit(1);
                    }
                    Err(e) => logging::error(&format!("could not open library: {e}")),
                }
            }
            if let Err(e) = api::programs::start(Arc::new(AppHost(handle.clone()))) {
                logging::warn(&format!("plugin programs get no API address: {e}"));
            }
            keep_claiming_api_port(&handle);
            if opts.headless {
                end_when_idle(&handle);
                return Ok(());
            }
            finish_downloads(&handle);
            let to_tray = opts.tray && handle.state::<AppState>().db_path.read().is_some();
            let w = build_main_window(&handle, to_tray)?;
            if to_tray {
                // without a tray to come back from, the window shows after all
                let (app, w) = (handle.clone(), w.clone());
                std::thread::spawn(move || {
                    if crate::desktop::system::tray::claim(&app) == crate::desktop::system::tray::Tray::None {
                        let _ = w.show();
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            libraries::startup,
            plugin_manager::job_cancel,
            plugin_manager::tool_op,
            plugin_pages::plugin_api,
            plugin_pages::plugin_file,
            plugin_pages::plugin_subscribe,
            plugin_pages::plugin_unsubscribe,
            plugin_pages::plugin_window,
            plugin_pages::plugin_window_info,
            tools::tool_window,
            tools::tool_window_info,
            plugin_pages::theme_shown,
            media::page_jobs,
            media::page_job_done,
            fonts::font_describe,
            fonts::font_glyph_names,
            fonts::font_face_bytes,
            plugin_pages::page_reads,
            plugin_pages::page_read_done,
            settings::window_look,
            settings::system_fonts,
            reads::root_dirs,
            import::library_json,
            media::clipboard_thumbnail,
            media::video_frame_op,
            media::media_codecs_found,
            media::subtitle_tracks,
            media::subtitle_load,
            reads::tag_group_items,
            settings::ui_sync,
            app_windows::open_prefs_window,
            reads::saved_filters,
            reads::list_cells,
            reads::folder_covers,
            reads::root_dir_covers,
            import::cancel_import,
            libraries::open_library,
            libraries::new_library,
            libraries::open_library_dialog,
            settings::ui_settings_load,
            libraries::language,
            settings::ui_settings_save,
            settings::ui_settings_changed,
            reads::search,
            reads::grid_items,
            reads::grid_items_by_uid,
            reads::items_at_paths,
            reads::asset_detail,
            reads::assets_brief,
            reads::sidebar,
            reads::status,
            reads::prioritize_thumbs,
            reads::contributions,
            edits::file_ops,
            edits::managed_orphans,
            edits::folder_op,
            edits::tag_op,
            edits::smart_op,
            import::pick,
            import::import_clashes,
            import::transfer,
            import::import_clipboard,
            import::import_dot_library,
            import::root_op,
            import::relink,
            files::open_item,
            files::mark_used,
            reads::paths_of,
            maintenance::export,
            settings::library_settings,
            maintenance::maintenance,
            maintenance::backup_list,
            maintenance::backup_restore,
            plugin_manager::plugin_list,
            plugin_manager::plugin_op,
            files::save_text,
            files::pick_text_file,
            files::pick_theme_file,
            files::clear_theme_file,
            files::read_theme_file,
            files::open_url,
            reads::selection_summary,
            reads::filter_facets,
            reads::spectrum_colors,
            reads::field_facets,
            reads::smart_buckets,
            libraries::library_list,
            edits::favorite_op,
            reads::folder_items,
            settings::auto_import_get,
            settings::auto_import_save,
            settings::auto_import_allow,
            app_windows::new_window,
            app_windows::quit_app,
            import::copy_to_folder,
            edits::trash_in_folder,
            edits::restore_to_folder,
            import::copy_files,
            files::open_with,
            app_windows::open_item_window,
            app_windows::reveal_window,
            reads::set_selection,
            app_windows::window_shown,
            api_host::api_window_ready,
            api_host::api_window_reply,
            viewer_page::open_web,
            viewer_page::viewer_page,
            files::start_drag,
            files::pointer_over_other_window,
            files::take_drops,
            settings::app_config,
            api_host::api_token_op,
            api_host::api,
        ])
        .build(tauri::generate_context!());
    match result {
        Ok(app) => app.run(|app, event| match event {
            // a background holder has no window, so the hidden one a link's thumbnail is drawn in is its last when it closes; it ends when idle instead
            tauri::RunEvent::ExitRequested { code: None, api, .. } if app.state::<AppState>().me.lock().background => api.prevent_exit(),
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Opened { urls } => crate::desktop::system::scheme::opened(app, urls),
            // a click on its Dock icon brings back this process's window kept in the menu bar, as on a Mac it should
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { has_visible_windows: false, .. } => {
                if let Some(w) = app.get_webview_window("main") {
                    app_windows::raise(&w);
                }
            }
            tauri::RunEvent::Exit => {
                if let Some(lib) = app.state::<AppState>().lib.lock().take() {
                    lib.shutdown();
                }
            }
            _ => {}
        }),
        Err(e) => {
            eprintln!("window error: {e}");
            std::process::exit(1);
        }
    }
}

#[allow(dead_code)]
fn _assert_types(_: Arc<()>, _: HashMap<(), ()>, _: Duration, _: HashPolicy) {}
