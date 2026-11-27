//! Curio's notices of what was added to a library, and of what an add from outside couldn't, as the system's notifications: a click shows the items in Curio; one that couldn't add has a button that keeps a copy or tries again.
//! The process that made the add shows them, a background holder too; Windows reports a click only while the notification is on screen, not from its list.

use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde_json::{json, Value};
use tauri::Manager;
use tauri_plugin_notification::{Action, ActionPerformed, ActionType, NotificationExt};

use crate::core::api::{self, Caller};
use crate::core::import::notices::{Kind, Notice};
use crate::core::{library::instance, system::logging};

const KEPT: usize = 50;
/// About as long as the system shows a notification: a background holder stays that long for a click on it.
const ON_SCREEN: Duration = Duration::from_secs(10);
static SOUND: &[u8] = include_bytes!("notify.wav");

/// The notices shown, by notification id, for the clicks on them.
static SHOWN: Mutex<Vec<(i32, Notice)>> = Mutex::new(Vec::new());
static LAST: Mutex<Option<Instant>> = Mutex::new(None);

/// The language the buttons were last labelled in.
static BUTTONS: Mutex<String> = Mutex::new(String::new());

/// The buttons a notice can carry, labelled in the language shown now; registered again, under the same ids, when it has changed since.
fn label_buttons(app: &tauri::AppHandle) {
    let now = crate::desktop::locale::current();
    let mut done = BUTTONS.lock();
    if *done == now {
        return;
    }
    let one = |kind: &str, id: &str, label: String| ActionType::builder(kind).actions(vec![Action::builder(id, label).build()]).build();
    let types = vec![one("already", "keep", crate::desktop::locale::text("notice.keepBoth", &[])), one("failed", "again", crate::desktop::locale::text("notice.retry", &[]))];
    if app.notification().register_action_types(types).is_ok() {
        *done = now;
    }
}

/// Notices go to the system's notifications from now on.
pub fn install(app: &tauri::AppHandle) {
    let n = app.notification();
    label_buttons(app);
    let a = app.clone();
    let _ = n.on_action(move |p: &ActionPerformed| {
        let Some(id) = p.notification().map(|n| n.id()) else { return };
        let Some(notice) = SHOWN.lock().iter().find(|s| s.0 == id).map(|s| s.1.clone()) else { return };
        let action = if p.action_id() == "tap" { "show".to_string() } else { p.action_id().to_string() };
        let a = a.clone();
        std::thread::spawn(move || {
            if let Err(e) = act(&a, &notice, &action) {
                logging::warn(&format!("notice {action}: {e}"));
            }
        });
    });
    let app = app.clone();
    crate::core::import::notices::set_sink(Box::new(move |n| show(&app, n)));
}

/// Whether a notice shown here may still be clicked.
pub fn showing() -> bool {
    GATHERED.lock().is_some() || LAST.lock().is_some_and(|t| t.elapsed() < ON_SCREEN)
}

fn read_ui() -> Value {
    crate::desktop::settings::read_ui_settings().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn sound_enabled(kind: Kind) -> bool {
    let ui = read_ui();
    if !ui["notifySound"].as_bool().unwrap_or(true) || !ui["notifySoundOs"].as_bool().unwrap_or(true) {
        return false;
    }
    let key = if kind == Kind::Failed { "error" } else { "import" };
    ui["notifySoundEvents"][key].as_bool().unwrap_or(true)
}

fn play_sound() {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
        let _ = unsafe { PlaySoundW(PCWSTR(SOUND.as_ptr() as *const u16), None, SND_MEMORY | SND_ASYNC | SND_NODEFAULT) };
    }
    #[cfg(unix)]
    {
        std::thread::spawn(|| {
            let path = std::env::temp_dir().join("curio-notify.wav");
            if !path.is_file() && std::fs::write(&path, SOUND).is_err() {
                return;
            }
            let players: &[&str] = if cfg!(target_os = "macos") { &["afplay"] } else { &["paplay", "pw-play", "aplay"] };
            for bin in players {
                if std::process::Command::new(bin).arg(&path).status().is_ok_and(|s| s.success()) {
                    break;
                }
            }
        });
    }
}

/// Settings › Notifications › Import notifications: every add, adds made while no window of Curio's has the keyboard (unless set), failures only, or none.
fn wanted(app: &tauri::AppHandle, kind: Kind) -> bool {
    // a test run's windows never have the keyboard, so every add would reach the desktop
    if crate::desktop::background() {
        return false;
    }
    let ui = read_ui();
    match ui["addNotices"].as_str().unwrap_or("background") {
        "off" => false,
        "failures" => kind != Kind::Added,
        "all" => true,
        _ => kind != Kind::Added || !app.webview_windows().values().any(|w| w.is_focused().unwrap_or(false)),
    }
}

fn where_to(n: &Notice) -> String {
    let lib = instance::library_name(&n.library);
    match &n.folder {
        Some(f) => format!("{lib} › {f}"),
        None => lib,
    }
}

/// The title, the text, and the action type that gives the notification its button.
fn words(n: &Notice) -> (String, String, Option<&'static str>) {
    words_in(n, &crate::desktop::locale::current())
}

/// Counts follow a colon ("Added: 12"), so the words need no plural forms in any language.
fn words_in(n: &Notice, code: &str) -> (String, String, Option<&'static str>) {
    let text = |key: &str, args: &[(&str, &str)]| crate::desktop::locale::text_in(code, key, args);
    match n.kind {
        Kind::Added => {
            let title = match (n.items.len(), n.names.first()) {
                (1, Some(name)) => name.clone(),
                (k, _) => text("notice.added", &[("count", &k.to_string())]),
            };
            let mut body = text("notice.addedTo", &[("place", &where_to(n))]);
            if !n.errors.is_empty() {
                body += &format!(" · {}", text("notice.failed", &[("count", &n.errors.len().to_string())]));
            }
            (title, body, None)
        }
        Kind::Already => (text("notice.already", &[]), instance::library_name(&n.library), (!n.again.is_empty()).then_some("already")),
        Kind::Failed => {
            let title = if n.errors.len() > 1 { text("notice.notAddedCount", &[("count", &n.errors.len().to_string())]) } else { text("notice.notAdded", &[]) };
            (title, n.errors.first().cloned().unwrap_or_default(), (!n.again.is_empty()).then_some("failed"))
        }
    }
}

/// Adds made one after another are told in one notice, once none has come for this long: a long import is told at its end.
const GATHER: Duration = Duration::from_millis(1200);

/// The adds gathered so far, and the turn of the last.
static GATHERED: Mutex<Option<(Notice, u64)>> = Mutex::new(None);
static TURN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Tells of what a library just added.
pub fn added(app: &tauri::AppHandle, n: Notice) {
    show(app, n);
}

fn show(app: &tauri::AppHandle, n: Notice) {
    if !wanted(app, n.kind) {
        return;
    }
    if n.kind != Kind::Added {
        return present(app, n);
    }
    let turn = TURN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let other = {
        let mut g = GATHERED.lock();
        match g.as_mut() {
            Some((held, last)) if held.library == n.library => {
                held.absorb(n);
                *last = turn;
                None
            }
            _ => g.replace((n, turn)).map(|h| h.0),
        }
    };
    // another library's adds, told now that this one's began
    if let Some(o) = other {
        present(app, o);
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(GATHER);
        let due = {
            let mut g = GATHERED.lock();
            if g.as_ref().is_some_and(|(_, last)| *last == turn) { g.take().map(|h| h.0) } else { None }
        };
        if let Some(n) = due {
            present(&app, n);
        }
    });
}

fn present(app: &tauri::AppHandle, n: Notice) {
    if n.kind != Kind::Added {
        label_buttons(app);
    }
    let (title, text, kind) = words(&n);
    let id = fastrand::i32(1..i32::MAX);
    let notice_kind = n.kind;
    {
        let mut shown = SHOWN.lock();
        shown.push((id, n));
        let surplus = shown.len().saturating_sub(KEPT);
        shown.drain(..surplus);
    }
    *LAST.lock() = Some(Instant::now());
    let mut b = app.notification().builder().id(id).title(title).body(text);
    if let Some(k) = kind {
        b = b.action_type_id(k);
    }
    if let Err(e) = b.show() {
        logging::warn(&format!("no notification: {e}"));
    } else if sound_enabled(notice_kind) {
        play_sound();
    }
}

/// A click on a notice (show) or its button: keep both or retry.
fn act(app: &tauri::AppHandle, n: &Notice, action: &str) -> Result<(), String> {
    let host = crate::desktop::api_host::AppHost(app.clone());
    let mine = api::Host::library(&host).is_some_and(|l| api::same_path(&l, &n.library));
    if !mine {
        return Err(format!("{} is no longer open here", instance::library_name(&n.library)));
    }
    let window = Caller::window();
    match action {
        "again" | "keep" => {
            let (data, hub) = (api::Host::data_dir(&host), api::Host::events(&host));
            crate::core::api::with_library(&host, |lib| Ok(lib.downloads(&data, hub.clone()).again(&n.again, action == "keep"))).map(|_| ())
        }
        "show" => {
            let items: Vec<String> = if n.kind == Kind::Already { n.of.iter().cloned().collect() } else { n.items.clone() };
            crate::desktop::app_windows::show_library_window(app);
            if items.is_empty() {
                return Ok(());
            }
            // a window just made answers once it has loaded its library
            let mut last = String::new();
            for _ in 0..60 {
                let shown = api::handle(&host, &window, "view.show", json!({"what": "recent"})).and_then(|_| api::handle(&host, &window, "selection.set", json!({"ids": items})));
                match shown {
                    Ok(_) => return Ok(()),
                    Err(e) => last = e.message,
                }
                std::thread::sleep(Duration::from_millis(250));
            }
            Err(last)
        }
        other => Err(format!("No action {other}")),
    }
}
