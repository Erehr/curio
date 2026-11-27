//! Tiny application log: an append-only file in the config folder (`curio.log`).

use std::io::Write;
use std::sync::OnceLock;

use parking_lot::Mutex;

struct Log {
    file: Option<std::fs::File>,
}

static LOG: OnceLock<Mutex<Log>> = OnceLock::new();

fn get() -> &'static Mutex<Log> {
    LOG.get_or_init(|| Mutex::new(Log { file: None }))
}

pub fn init_file(path: &std::path::Path) {
    if let Some(p) = path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    // keep the log small: start over past 4 MB
    if std::fs::metadata(path).map(|m| m.len() > 4 << 20).unwrap_or(false) {
        let _ = std::fs::remove_file(path);
    }
    let f = std::fs::OpenOptions::new().create(true).append(true).open(path).ok();
    get().lock().file = f;
}

pub fn log(level: &str, msg: &str) {
    let ts = chrono::Local::now().format("%H:%M:%S");
    let line = format!("{ts} [{level}] {msg}");
    let mut l = get().lock();
    if let Some(f) = l.file.as_mut() {
        let _ = writeln!(f, "{line}");
    }
}

pub fn info(msg: &str) {
    log("info", msg);
}

pub fn warn(msg: &str) {
    log("warn", msg);
}

pub fn error(msg: &str) {
    log("error", msg);
}

/// Panics in worker threads otherwise reach only a console nobody has.
pub fn log_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        error(&format!("panic: {info}"));
        previous(info);
    }));
}
