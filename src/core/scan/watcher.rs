//! Native filesystem notifications (ReadDirectoryChangesW / inotify / FSEvents via `notify`),
//! batched and debounced. Notifications are treated as hints: overflow, errors or very large
//! batches fall back to a full reconcile of the affected root.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use notify::{EventKind, RecursiveMode, Watcher};
use parking_lot::Mutex;

use crate::core::consts::{WATCH_BATCH_MS, WATCH_OVERFLOW};

#[derive(Debug, Clone)]
pub struct WatchBatch {
    pub root_id: i64,
    pub paths: Vec<PathBuf>,
    /// true when notifications were lost or too many arrived: reconcile the whole root
    pub rescan: bool,
}

type RootMap = Arc<Mutex<HashMap<i64, PathBuf>>>;

pub struct FsWatcher {
    watcher: Option<notify::RecommendedWatcher>,
    roots: RootMap,
}

impl FsWatcher {
    /// Start the watcher; batches are delivered on `out`.
    pub fn start(out: Sender<WatchBatch>) -> FsWatcher {
        let (tx, rx) = crossbeam_channel::unbounded::<notify::Result<notify::Event>>();
        let watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .map_err(|e| crate::core::system::logging::warn(&format!("filesystem notifications unavailable: {e}")))
        .ok();
        let roots: RootMap = Default::default();
        let shared = roots.clone();
        std::thread::Builder::new().name("fs-watch-debounce".into()).spawn(move || debounce_loop(rx, out, shared)).ok();
        FsWatcher { watcher, roots }
    }

    pub fn watch(&mut self, root_id: i64, path: PathBuf) {
        if self.roots.lock().get(&root_id) == Some(&path) {
            return;
        }
        self.unwatch(root_id);
        if let Some(w) = self.watcher.as_mut() {
            match w.watch(&path, RecursiveMode::Recursive) {
                Ok(()) => {
                    self.roots.lock().insert(root_id, path);
                }
                Err(e) => crate::core::system::logging::warn(&format!("cannot watch {}: {e}", path.display())),
            }
        }
    }

    pub fn unwatch(&mut self, root_id: i64) {
        let removed = self.roots.lock().remove(&root_id);
        if let (Some(p), Some(w)) = (removed, self.watcher.as_mut()) {
            let _ = w.unwatch(&p);
        }
    }

    pub fn is_watching(&self, root_id: i64) -> bool {
        self.roots.lock().contains_key(&root_id)
    }
}

fn root_for(roots: &HashMap<i64, PathBuf>, p: &std::path::Path) -> Option<i64> {
    roots.iter().filter(|(_, r)| p.starts_with(r)).max_by_key(|(_, r)| r.as_os_str().len()).map(|(id, _)| *id)
}

fn debounce_loop(rx: Receiver<notify::Result<notify::Event>>, out: Sender<WatchBatch>, roots: RootMap) {
    let window = Duration::from_millis(WATCH_BATCH_MS * 2);
    let mut pending: HashMap<i64, HashSet<PathBuf>> = HashMap::new();
    let mut rescan: HashSet<i64> = HashSet::new();
    let mut first_at: Option<Instant> = None;
    loop {
        let timeout = match first_at {
            Some(t) => window.saturating_sub(t.elapsed()),
            None => Duration::from_secs(3600),
        };
        match rx.recv_timeout(timeout) {
            Ok(Ok(ev)) => {
                let r = roots.lock();
                let need_rescan = ev.need_rescan();
                if !matches!(ev.kind, EventKind::Access(_)) {
                    for p in ev.paths {
                        if let Some(id) = root_for(&r, &p) {
                            if need_rescan {
                                rescan.insert(id);
                            } else {
                                pending.entry(id).or_default().insert(p);
                            }
                        }
                    }
                    if need_rescan {
                        rescan.extend(r.keys());
                    }
                }
                if first_at.is_none() && (!pending.is_empty() || !rescan.is_empty()) {
                    first_at = Some(Instant::now());
                }
            }
            Ok(Err(_err)) => {
                // errors usually mean overflow or a vanished root: rescan everything watched
                rescan.extend(roots.lock().keys());
                first_at.get_or_insert_with(Instant::now);
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                for (root_id, paths) in pending.drain() {
                    let full = rescan.remove(&root_id) || paths.len() > WATCH_OVERFLOW;
                    let _ = out.send(WatchBatch { root_id, paths: if full { vec![] } else { paths.into_iter().collect() }, rescan: full });
                }
                for root_id in rescan.drain() {
                    let _ = out.send(WatchBatch { root_id, paths: vec![], rescan: true });
                }
                first_at = None;
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }
    }
}
