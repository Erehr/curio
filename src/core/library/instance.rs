//! One library, one writer. Every window of the app is its own process, so different libraries
//! can be open side by side. A library is held with an operating-system file lock: the second
//! window that asks for it is sent to the window that already has it, because two processes
//! would each run a scanner, a thumbnail queue and in-memory caches over the same data.
//! The lock disappears by itself when a process ends, so a crash never leaves a library stuck.
//!
//! Each process also listens on a private loopback port (with a random token kept in the
//! library folder, readable only by this user) so another window can bring it to the front or
//! hand it files ("Add to Other Library").
//! A process may hold a library in the background for the API, with no window (`Holder::background`); it gives the library up when asked.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const LOCK_FILE: &str = "instance.lock";
const INFO_FILE: &str = "instance.json";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Holder {
    pub pid: u32,
    pub port: u16,
    pub token: String,
    /// When this window took the library and when it last had the focus, for the API's default library.
    #[serde(default)]
    pub opened: i64,
    #[serde(default)]
    pub focused: i64,
    /// Held for the API with no window (`--headless`); it ends when idle, and gives the library up to a window that asks.
    #[serde(default)]
    pub background: bool,
    /// The window's id for the API's window operations; it stays when the window switches libraries.
    #[serde(default)]
    pub window: String,
}

/// Held for as long as this process has the library open.
pub struct InstanceLock {
    _file: File,
    info: PathBuf,
}

impl InstanceLock {
    /// Rewrites what other processes read about this holder.
    pub fn update(&self, me: &Holder) {
        write_private(&self.info, &serde_json::to_vec(me).unwrap_or_default());
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.info);
    }
}

pub enum Acquired {
    Mine(InstanceLock),
    /// Another window has it (details are missing only in the instant it is starting up).
    Elsewhere(Option<Holder>),
}

fn read_info(lib: &Path) -> Option<Holder> {
    serde_json::from_str(&std::fs::read_to_string(lib.join(INFO_FILE)).ok()?).ok()
}

/// The suffix library folders get.
pub const LIBRARY_SUFFIX: &str = ".curio";

/// A library folder's name without its suffix.
pub fn library_name(path: &Path) -> String {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    name.strip_suffix(LIBRARY_SUFFIX).map(String::from).unwrap_or(name)
}

/// Only a folder holding a library database counts; opening must never turn other folders into libraries.
pub fn is_library(path: &Path) -> bool {
    path.join("library.db").is_file()
}

pub const OPEN_LIBRARY_RENAME: &str = "That library is open. Close its window first, or rename it from another window.";

/// Rename a library folder to `<name>.curio` beside it. Refused while any window holds it.
pub fn rename_library(from: &Path, name: &str) -> Result<PathBuf, String> {
    if holder(from).is_some() {
        return Err(OPEN_LIBRARY_RENAME.into());
    }
    if name.trim().is_empty() {
        return Err("The name is empty".into());
    }
    let clean = crate::core::import::sanitize_name(name.trim());
    let to = from.with_file_name(format!("{clean}{LIBRARY_SUFFIX}"));
    if to == from {
        return Ok(to);
    }
    if to.exists() {
        return Err(format!("{} already exists there", to.display()));
    }
    // a background holder that just let go may still be closing its files
    let started = std::time::Instant::now();
    loop {
        match std::fs::rename(from, &to) {
            Ok(()) => return Ok(to),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied && started.elapsed() < RELEASE_WAIT => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(e) => return Err(format!("Couldn't rename the library folder: {e}")),
        }
    }
}

/// Wait until the process `pid` no longer holds a library, as one ending by itself does.
pub fn wait_gone(lib: &Path, pid: u32, limit: std::time::Duration) {
    let started = std::time::Instant::now();
    while holder(lib).is_some_and(|h| h.pid == pid) && started.elapsed() < limit {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Ask a library's background holder to give it up. False when a window holds it, or the holder is busy; with `show` a busy holder then shows its window.
pub fn release(lib: &Path, show: bool) -> bool {
    let Some(h) = holder(lib).filter(|h| h.background) else { return false };
    let released = tell(&h, "release", &serde_json::json!({ "show": show })).is_ok_and(|r| r["released"] == true);
    if released {
        wait_gone(lib, h.pid, RELEASE_WAIT);
    }
    released && holder(lib).is_none()
}

const RELEASE_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// Wait for the process holding a library to finish, for a relaunch that replaces it.
pub fn wait_until_free(lib: &Path, limit: std::time::Duration) {
    let started = std::time::Instant::now();
    while holder(lib).is_some() && started.elapsed() < limit {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

pub fn acquire(lib: &Path, me: &Holder) -> Result<Acquired, String> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lib.join(LOCK_FILE))
        .map_err(|e| format!("cannot open the library lock: {e}"))?;
    let mut tries = 0;
    let got = loop {
        match file.try_lock() {
            // another window may only be looking (see `holder`)
            Err(std::fs::TryLockError::WouldBlock) if read_info(lib).is_none() && tries < LOOK_TRIES => {
                tries += 1;
                std::thread::sleep(LOOK_PAUSE);
            }
            other => break other,
        }
    };
    match got {
        Ok(()) => {
            let info = lib.join(INFO_FILE);
            write_private(&info, &serde_json::to_vec(me).unwrap_or_default());
            Ok(Acquired::Mine(InstanceLock { _file: file, info }))
        }
        Err(std::fs::TryLockError::WouldBlock) => Ok(Acquired::Elsewhere(read_info(lib))),
        Err(std::fs::TryLockError::Error(e)) => Err(format!("cannot lock the library: {e}")),
    }
}

/// Looking takes the lock for an instant, so two looks at once would each see the other as a holder.
static LOOKING: std::sync::Mutex<()> = std::sync::Mutex::new(());
const LOOK_TRIES: u32 = 10;
const LOOK_PAUSE: std::time::Duration = std::time::Duration::from_millis(20);

/// Who has this library open right now, without keeping any lock.
pub fn holder(lib: &Path) -> Option<Holder> {
    let _one_at_a_time = LOOKING.lock().unwrap_or_else(|e| e.into_inner());
    let file = OpenOptions::new().read(true).write(true).open(lib.join(LOCK_FILE)).ok()?;
    for _ in 0..LOOK_TRIES {
        match file.try_lock() {
            Ok(()) => return None,
            // a window holding the library writes its details straight away; without them it is most likely another window looking
            Err(std::fs::TryLockError::WouldBlock) => {
                if let Some(h) = read_info(lib) {
                    return Some(h);
                }
                std::thread::sleep(LOOK_PAUSE);
            }
            Err(_) => return None,
        }
    }
    Some(Holder::default())
}

/// Ask the window holding a library to do something. Returns false when it can't be reached.
pub fn tell(holder: &Holder, route: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
    if holder.port == 0 {
        return Err("that window is still starting".into());
    }
    let url = format!("http://127.0.0.1:{}/{route}", holder.port);
    let resp = ureq::post(&url)
        .set("X-Curio-Token", &holder.token)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("could not reach the other window: {e}"))?;
    // `into_string` stops at 10 MB; a long item list can be larger
    let mut text = String::new();
    std::io::Read::read_to_string(&mut resp.into_reader(), &mut text).map_err(|e| e.to_string())?;
    Ok(serde_json::from_str(&text).unwrap_or(serde_json::Value::Null))
}

// ---------------------------------------------------------------- moving items between libraries

const TRANSFER_MAX_AGE_MS: i64 = 60 * 60 * 1000;

/// What the source window knows about the files it is handing over (tags, rating, notes …),
/// left where the receiving window can read it because a file drop only carries paths.
pub fn write_transfer(data_dir: &Path, items: Vec<(String, serde_json::Value)>) {
    let map: serde_json::Map<String, serde_json::Value> = items.into_iter().collect();
    let body = serde_json::json!({ "at": crate::core::model::now_ms(), "items": map });
    let _ = std::fs::create_dir_all(data_dir);
    let _ = std::fs::write(data_dir.join("transfer.json"), serde_json::to_vec(&body).unwrap_or_default());
}

pub fn read_transfer(data_dir: &Path) -> serde_json::Map<String, serde_json::Value> {
    let Ok(text) = std::fs::read_to_string(data_dir.join("transfer.json")) else { return Default::default() };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return Default::default() };
    if crate::core::model::now_ms() - v["at"].as_i64().unwrap_or(0) > TRANSFER_MAX_AGE_MS {
        return Default::default();
    }
    v["items"].as_object().cloned().unwrap_or_default()
}

/// Dropped paths and stored paths can differ in case and slashes on Windows.
pub fn transfer_key(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

/// A file inside another library's own store belongs to that library: it has to be copied.
pub fn in_library_store(path: &Path) -> bool {
    path.parent().is_some_and(|dir| {
        dir.components().any(|c| {
            let n = c.as_os_str().to_string_lossy().to_lowercase();
            n.ends_with(".library") || n.ends_with(LIBRARY_SUFFIX)
        })
    })
}

/// The file holds the control token: on Unix only this user may read it.
fn write_private(path: &Path, data: &[u8]) {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let _ = std::fs::remove_file(path);
        if let Ok(mut f) = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path) {
            let _ = f.write_all(data);
        }
    }
    #[cfg(not(unix))]
    let _ = std::fs::write(path, data);
}
