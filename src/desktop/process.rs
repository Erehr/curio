//! The program among its processes: the control server and its token, holding a library no window shows, ending when idle.

use crate::desktop::import::add_files;
use crate::desktop::app_windows::show_main;
use crate::desktop::api_host::AppHost;
use crate::desktop::AppState;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;
use parking_lot::Mutex;
use tauri::{Emitter, Manager};
use crate::core::api;
use crate::core::library::instance;
use crate::core::system::logging;
use crate::desktop::api_host::window_call;

/// The control channel carries API calls with any permission, so its token must not be guessable.
pub(super) fn new_token() -> String {
    format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
}

/// Lets another window of the app raise this one or hand it files. Loopback only; requests must
/// carry the token this process wrote into the library folder.
pub(super) fn start_control_server(app: &tauri::AppHandle) {
    let Ok(server) = tiny_http::Server::http("127.0.0.1:0") else { return };
    let port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(0);
    let token = {
        let st = app.state::<AppState>();
        let mut me = st.me.lock();
        me.port = port;
        me.token.clone()
    };
    let app = app.clone();
    let _ = std::thread::Builder::new().name("control".into()).spawn(move || {
        for mut req in server.incoming_requests() {
            let authorised = req.headers().iter().any(|h| h.field.equiv("X-Curio-Token") && h.value.as_str() == token);
            if !authorised {
                let _ = req.respond(tiny_http::Response::empty(403));
                continue;
            }
            if app.state::<AppState>().ending.load(Ordering::Relaxed) {
                let _ = req.respond(tiny_http::Response::empty(503));
                continue;
            }
            let mut body = String::new();
            let _ = std::io::Read::read_to_string(req.as_reader(), &mut body);
            let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            if req.url() == "/api" {
                let app = app.clone();
                std::thread::spawn(move || {
                    let reply = api::answer_forwarded(&AppHost(app), &v);
                    let _ = req.respond(tiny_http::Response::from_string(reply.to_string()));
                });
                continue;
            }
            // a question for a person that the process on the API's port has no window to ask
            if req.url() == "/ask" {
                let app = app.clone();
                std::thread::spawn(move || {
                    let asked = (v["op"] == "pair.ask").then(|| window_call(&app, "main", "pair.ask", v["args"].clone(), true)).flatten();
                    let reply = match asked {
                        Some(Ok(ok)) => serde_json::json!({ "ok": ok }),
                        Some(Err(e)) => serde_json::json!({ "error": e }),
                        None => serde_json::json!({ "error": api::ApiError::new("no_window", "This process has no window") }),
                    };
                    let _ = req.respond(tiny_http::Response::from_string(reply.to_string()));
                });
                continue;
            }
            // the router gathers this library's events for the API's subscribers
            if req.url() == "/events" {
                let hub = app.state::<AppState>().hub.clone();
                std::thread::spawn(move || api::events::serve_lines(&hub, &v, req));
                continue;
            }
            match req.url() {
                "/focus" => show_main(&app),
                // another window changed something this one shows (see `announce`)
                "/event" => {
                    let _ = app.emit("event", &v);
                }
                "/release" => {
                    let released = end_holding(&app, Duration::ZERO);
                    if !released && v["show"] == true {
                        show_main(&app);
                    }
                    let _ = req.respond(tiny_http::Response::from_string(serde_json::json!({ "released": released }).to_string()));
                    if released {
                        app.exit(0);
                        return;
                    }
                    continue;
                }
                "/quit" => {
                    let _ = req.respond(tiny_http::Response::from_string("{}"));
                    app.exit(0);
                    return;
                }
                "/add" => {
                    let paths: Vec<String> = v["paths"].as_array().map(|a| a.iter().filter_map(|p| p.as_str().map(String::from)).collect()).unwrap_or_default();
                    let st = app.state::<AppState>();
                    let how = if v["how"] == "api" { "api" } else { "window" };
                    if let Err(e) = add_files(&st, paths.into_iter().map(PathBuf::from).collect(), how) {
                        logging::error(&e);
                    }
                }
                _ => {}
            }
            let _ = req.respond(tiny_http::Response::from_string("{}"));
        }
    });
}

pub(super) const RELAUNCH_WAIT: Duration = Duration::from_secs(15);

/// A background holder with nothing to do ends after this long.
const HOLDER_IDLE: Duration = Duration::from_secs(60);

const HOLDER_CHECK: Duration = Duration::from_secs(5);

/// Showing the window and ending a background holder exclude each other.
pub(super) static HOLDING: Mutex<()> = Mutex::new(());

/// A background holder lets its library go when no call came for `quiet` and its queues are empty; the caller then ends the process.
fn end_holding(app: &tauri::AppHandle, quiet: Duration) -> bool {
    let _one = HOLDING.lock();
    let st = app.state::<AppState>();
    if !st.me.lock().background || app.get_webview_window("main").is_some() || !st.activity.idle_for().is_some_and(|d| d >= quiet) {
        return false;
    }
    let busy = st.lib.lock().as_ref().is_some_and(|l| l.busy().is_some() || l.thumbs_pending() > 0 || l.import_state().is_some() || l.downloading()) || crate::desktop::system::notifications::showing();
    if busy {
        return false;
    }
    st.ending.store(true, Ordering::Relaxed);
    if let Some(lib) = st.lib.lock().take() {
        lib.shutdown();
    }
    st.instance.lock().take();
    true
}

/// Libraries no process holds whose downloads were left unfinished open in the background, which goes on with them.
pub(super) fn finish_downloads(app: &tauri::AppHandle) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("downloads".into()).spawn(move || {
        std::thread::sleep(Duration::from_secs(3));
        for lib in crate::core::import::downloads::unfinished(&crate::core::system::app_dirs::data_dir()) {
            if lib.join("library.db").is_file() && instance::holder(&lib).is_none() {
                if let Err(e) = api::Host::start_holder(&AppHost(app.clone()), &lib) {
                    logging::warn(&e);
                }
            }
        }
    });
}

pub(super) fn end_when_idle(app: &tauri::AppHandle) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("idle".into()).spawn(move || loop {
        std::thread::sleep(HOLDER_CHECK);
        if !app.state::<AppState>().me.lock().background {
            return;
        }
        if end_holding(&app, HOLDER_IDLE) {
            logging::info("background holder idle; ending");
            app.exit(0);
            return;
        }
    });
}
