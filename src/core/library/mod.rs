//! The library service: owns the background workers and is the only API the UI talks to.
//!
//! Threads (each with its own SQLite connection; WAL lets readers run during writes):
//!   writer    — user edits and worker results, batched into short transactions
//!   scanner   — folder reconciliation, filesystem notifications, maintenance, hashing
//!   query     — searches and sidebar statistics (stale requests are dropped)
//!   thumbs×N  — thumbnails, palettes, fingerprints; visible items first
//!   metadata  — extraction (built-in + plugin extractors, ffprobe durations)
//! The UI thread keeps one read connection for small primary-key lookups.

pub mod backup;
pub mod changes;
pub mod edits;
pub mod instance;
pub mod metadata;
pub mod paths;
mod plugin_host;
mod scanning;
pub mod settings;
pub mod thumbnailing;
pub mod upkeep;
pub mod writer;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::{Condvar, Mutex, RwLock};

use crate::core::db::{Db, DbResult};
use crate::core::library::paths::LibraryPaths;
use crate::core::system::logging;
use crate::core::model::*;
use crate::core::plugins::{self, contributions::Contributions, manifest::Plugin};
use crate::core::metadata::readers::{self, Mode};
use crate::core::said::Said;
use crate::core::scan::{self, ScanOptions};
use crate::core::scan::watcher::WatchBatch;
use crate::core::library::plugin_host::record_failure;
use crate::core::library::plugin_host::apply_reply;
use crate::core::library::plugin_host::plugin_succeeded;
use crate::core::library::plugin_host::PROGRAM_TIMEOUT;
use crate::core::library::plugin_host::plugin_payload;
use crate::core::library::thumbnailing::Poster;
use crate::core::library::metadata::meta_loop;
use crate::core::library::thumbnailing::thumb_loop;
use crate::core::library::scanning::scanner_loop;
use crate::core::library::writer::writer_loop;
use crate::core::library::thumbnailing::TQ;
use crate::core::library::plugin_host::load_plugins;
use crate::core::library::thumbnailing::ThumbQueue;
use crate::core::library::changes::Parts;
use crate::core::library::settings::LibrarySettings;
use crate::core::library::changes::Changed;

/// A location's scan under way (`done` empty) or finished.
#[derive(Clone, Debug)]
pub struct ScanNote {
    pub location: i64,
    pub files: usize,
    pub done: Option<scan::ScanStats>,
}

pub enum LibEvent {
    Changed(Changed),
    ThumbReady(AssetId),
    Scan(ScanNote),
    Progress(Said),
    Done(Said),
    Error(Said),
}

type WriteFn = Box<dyn FnOnce(&mut Db, &Ctx) -> DbResult<Changed> + Send>;
type DoneFn = Box<dyn FnOnce(DbResult<()>) + Send>;

enum WriterJob {
    Run(WriteFn),
    /// An edit whose caller waits; told the outcome once the transaction is committed or rolled back.
    RunThen(WriteFn, DoneFn),
    ThumbDone { id: AssetId, ok: bool, dims: Option<(i64, i64)>, aspect: Option<f64>, palette: Vec<PaletteColor>, look: Option<crate::core::duplicates::fingerprints::Look>, duration: Option<f64>, title: Option<String> },
    /// Waits for FFmpeg to be found.
    ThumbWaiting { id: AssetId },
    /// Left to a plugin's thumbnail page.
    ThumbPage { id: AssetId },
    /// What reading a file found. `whole`: every reader ran, for a new or changed file; otherwise the queued readers `done` ran. `ran` names every reader run, whose errors `failed` replaces. `later`: the page readers the file is queued for afterwards.
    Meta { id: AssetId, plan: readers::Plan, whole: bool, done: Vec<String>, ran: Vec<String>, failed: Vec<(String, String)>, duration: Option<f64>, dims: Option<(i64, i64)>, later: Vec<String> },
}

enum ScanJob {
    Root(i64),
    All,
    Watch(WatchBatch),
    Maintenance,
    Backup,
    /// purged copies waiting for the Recycle Bin
    FileOps,
    /// managed items whose files must follow their folders or the trash
    SyncStore,
    /// queue every managed item whose files don't match the index, then follow the queue
    CheckStore,
    /// queued entries whose store pass failed (`0` the folder tree), queued again after a pause
    RetryStore(Vec<AssetId>),
    Run(WriteFn),
}

/// Shared state visible to every worker.
pub struct Ctx {
    pub paths: LibraryPaths,
    pub settings: RwLock<LibrarySettings>,
    /// The stored settings JSON object, patched in place so keys or values from newer builds survive a save.
    settings_raw: RwLock<serde_json::Map<String, serde_json::Value>>,
    pub plugins: RwLock<Vec<Plugin>>,
    pub contrib: RwLock<Contributions>,
    pub events: Sender<LibEvent>,
    pub cancel: AtomicBool,
    pub thumbs_done: AtomicUsize,
    pub busy: Mutex<Option<String>>,
    /// the import under way, for the progress bar
    pub import: Mutex<Option<ImportState>>,
    /// new items arrived while a long job runs: look for thumbnails again now
    pub thumbs_wanted: AtomicBool,
    /// locations whose scan is under way: their new items get thumbnails and metadata once it ends, rather than taking the writer from it batch by batch
    scanning: Mutex<HashSet<i64>>,
    /// `meta.library_id`, stable for the life of the library folder
    pub library_id: String,
    /// changed since the last backup (persisted as `meta.backup_dirty` by the scanner)
    pub backup_dirty: AtomicBool,
    /// Whether the managed store's filesystem supports hard links; without them an item in several folders has a full copy in each.
    pub hard_links: bool,
    /// The duplicate search under way, stopped when another starts.
    pub dup_search: Mutex<Option<Arc<AtomicBool>>>,
    /// What plugin pages read in the window, waiting for the metadata thread to apply it.
    pub page_answers: Mutex<Vec<crate::core::library::metadata::PageAnswer>>,
    pub meta_wake: Condvar,
    writer: Sender<WriterJob>,
    scanner: Sender<ScanJob>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ImportState {
    pub done: usize,
    pub total: usize,
    pub name: String,
    pub started: i64,
}

/// One writer job per `PURGE_CHUNK` items, each queued behind the edits already waiting, so a large purge never holds the writer for long.
fn purge_in_jobs(db: &mut Db, ctx: &Ctx, mut ids: Vec<AssetId>) -> DbResult<Changed> {
    let rest = ids.split_off(ids.len().min(crate::core::consts::PURGE_CHUNK));
    let uids = db.uids(&ids)?;
    db.purge_items(&ids)?;
    if !rest.is_empty() {
        ctx.write(move |db, ctx| purge_in_jobs(db, ctx, rest));
    }
    // purged folders' and tags' counts change, and empty tags may go
    Ok(Changed::removed(&ids, true).with(Parts::TAGS | Parts::FOLDERS).naming(uids))
}

impl Ctx {
    /// The locations whose scan is under way.
    pub fn scanning(&self) -> Vec<i64> {
        self.scanning.lock().iter().copied().collect()
    }

    /// A location's scan begins: the window shows it under way.
    pub fn scan_started(&self, root: i64) {
        self.scanning.lock().insert(root);
        self.send(LibEvent::Scan(ScanNote { location: root, files: 0, done: None }));
    }

    /// A location's scan ends: its items' thumbnails and metadata follow.
    pub fn scan_ended(&self, root: i64, stats: scan::ScanStats) {
        self.scanning.lock().remove(&root);
        self.thumbs_wanted.store(true, Ordering::Relaxed);
        self.send(LibEvent::Scan(ScanNote { location: root, files: stats.seen, done: Some(stats) }));
    }

    /// Queue a small write from a thread that only reads; the writer owns every write but the scanner's.
    pub fn write(&self, f: impl FnOnce(&mut Db, &Ctx) -> DbResult<Changed> + Send + 'static) {
        let _ = self.writer.send(WriterJob::Run(Box::new(f)));
    }

    /// Have the scanner thread move managed files after their folders or trash state changed.
    pub fn kick_store(&self) {
        let _ = self.scanner.send(ScanJob::SyncStore);
    }

    /// Have the scanner thread carry out the queued file work, such as purged copies for the Recycle Bin.
    fn kick_file_ops(&self) {
        let _ = self.scanner.send(ScanJob::FileOps);
    }

    pub fn send(&self, e: LibEvent) {
        if matches!(&e, LibEvent::Changed(c) if !c.is_empty()) {
            self.backup_dirty.store(true, Ordering::Relaxed);
        }
        let _ = self.events.send(e);
    }

    /// Where this library's backups go.
    pub fn backup_dest(&self) -> PathBuf {
        let custom = self.settings.read().backup_dir.trim().to_string();
        if custom.is_empty() {
            return self.paths.backups.clone();
        }
        // several libraries may share one destination
        let name = self.paths.root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "library".into());
        PathBuf::from(custom).join(name)
    }
    pub fn scan_options(&self) -> ScanOptions {
        let s = self.settings.read();
        let c = self.contrib.read();
        let rules = crate::core::grouping::companions::raw_rule(&s.raw_pairs).into_iter().chain(c.companions.iter().cloned()).collect();
        ScanOptions { index_all_files: s.index_all_files, extra_exts: c.extra_exts.clone(), ..Default::default() }.with_companions(rules)
    }
}

pub struct Library {
    pub ctx: Arc<Ctx>,
    pub db: Db,
    writer: Sender<WriterJob>,
    scanner: Sender<ScanJob>,
    events: Receiver<LibEvent>,
    thumbq: Arc<ThumbQueue>,
    shutdown: Arc<AtomicBool>,
    downloads: std::sync::OnceLock<Arc<crate::core::import::downloads::Queue>>,
    /// found while opening: a damaged database, a rejected restore
    pub problems: Vec<String>,
    /// worth knowing but needing no action, such as a library on a network drive
    pub warnings: Vec<String>,
}

/// Why file work waiting in a library was set aside rather than finished.
pub const SET_ASIDE_NEW: &str = "Recorded on another computer, so it was skipped without touching files.";
pub const SET_ASIDE_RESTORED: &str = "Recorded before the restore, so it was skipped without touching files.";

impl Library {
    /// Open (or create) a library folder. `known_here`: this computer has opened it before; a library from elsewhere brings a database anyone could have written, so the file work it recorded and the folders it watches wait for the person.
    pub fn open(root: &Path, cache_override: Option<&Path>, plugin_dirs: &[PathBuf], known_here: bool) -> Result<Library, String> {
        let paths = LibraryPaths::new(root, cache_override);
        paths.ensure().map_err(|e| format!("cannot create library folders: {e}"))?;
        // shown to the user once the window is up
        let mut problems = vec![];
        let mut restored = false;
        match crate::core::library::backup::apply_pending_restore(&paths)? {
            crate::core::library::backup::Restore::Applied => {
                restored = true;
                logging::info("restored library database from backup");
            }
            crate::core::library::backup::Restore::Rejected(why) => {
                logging::error(&format!("staged restore not applied: {why}"));
                problems.push(format!("The backup chosen for restoring wasn't used, and the library is unchanged. {why}"));
            }
            crate::core::library::backup::Restore::Nothing => {}
        }
        let mut db = Db::open(&paths.db)?;
        db.catch_up_if_needed()?;
        // journalled paths are resolved against the managed root, so it has to point here before recovery
        db.repoint_managed_root(&paths.managed.to_string_lossy())?;
        db.refold_placement_keys()?;
        // a restored backup's file work was recorded against the files as they were then
        if !known_here || restored {
            let (why, whence) = if restored { (SET_ASIDE_RESTORED, "before the backup was restored") } else { (SET_ASIDE_NEW, "where this library came from") };
            let n = db.set_aside_file_ops(why)?;
            if n > 0 {
                logging::warn(&format!("set aside {n} file operations: {why}"));
                problems.push(format!("{n} unfinished file {} recorded {whence} skipped. See Settings › Storage & backup.", if n == 1 { "operation" } else { "operations" }));
            }
        }
        let mut replaced = crate::core::store::journal::recover_replaces(&mut db);
        // a replace cut short before its item's other folders were renewed renews them now
        for id in &replaced {
            if let Err(e) = crate::core::store::relink(&db, &paths, *id) {
                logging::warn(&e);
            }
        }
        replaced.extend(crate::core::store::recovery::recover(&mut db, &paths));
        if !crate::core::store::recovery::unknown_pending(&db).is_empty() {
            problems.push("Store file sync is paused: this library has pending file operations from a newer version of Curio.".into());
        }
        let ok = db.integrity_check(true)?;
        if ok != vec!["ok".to_string()] {
            logging::error(&format!("library database check reported: {}", ok.join("; ")));
            problems.push(format!("The library database failed its check, so some of it may be damaged: {}", ok.join("; ")));
        }
        let (mut settings_raw, mut settings) = LibrarySettings::parse_stored(db.meta_get("settings").as_deref());
        if !known_here && settings.auto_import.enabled {
            settings.auto_import.enabled = false;
            let ai = settings_raw.entry("auto_import".to_string()).or_insert_with(|| serde_json::json!({}));
            if let Some(obj) = ai.as_object_mut() {
                obj.insert("enabled".to_string(), serde_json::Value::Bool(false));
            }
            db.meta_set("settings", &serde_json::to_string(&settings_raw).map_err(|e| e.to_string())?)?;
            logging::warn("auto-import turned off: the library is new to this computer");
            problems.push("Auto-import was turned off: this library is new to this computer, and its watched folders were chosen elsewhere. Check them in Settings › Auto-Import before turning it on.".into());
        }
        let backup_dirty = db.meta_get("backup_dirty").as_deref() != Some("0");
        // one transaction: each separate commit would add its pages to the WAL again
        let library_id = db.in_tx(|db| {
            db.meta_set("app_version", env!("CURIO_VERSION"))?;
            db.meta_set("written_by", &crate::core::db::schema::SCHEMA_VERSION.to_string())?;
            let library_id = match db.meta_get("library_id") {
                Some(id) => id,
                None => {
                    let id = new_uid();
                    db.meta_set("library_id", &id)?;
                    id
                }
            };
            crate::core::import::managed_root(db, &paths)?;
            // built-in extractor schemas
            for f in crate::core::metadata::extract::builtin_schemas() {
                if let Err(e) = db.register_field(&f) {
                    logging::warn(&format!("field {}: {e}", f.key));
                }
            }
            Ok(library_id)
        })?;
        // no other connection is open yet, so the WAL left by migrations and the writes above can be emptied
        let _ = db.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

        let hard_links = crate::core::store::disk::probe_hard_links(&paths.managed);
        if !hard_links {
            logging::warn("the managed store's filesystem doesn't support hard links; items in several folders get full copies");
        }
        let (ev_tx, ev_rx) = unbounded();
        let (w_tx, w_rx) = unbounded();
        let (s_tx, s_rx) = unbounded();
        let ctx = Arc::new(Ctx {
            paths: paths.clone(),
            settings: RwLock::new(settings.clone()),
            settings_raw: RwLock::new(settings_raw),
            plugins: RwLock::new(vec![]),
            contrib: RwLock::new(Contributions::default()),
            events: ev_tx,
            cancel: AtomicBool::new(false),
            thumbs_done: AtomicUsize::new(0),
            busy: Mutex::new(None),
            import: Mutex::new(None),
            thumbs_wanted: AtomicBool::new(false),
            scanning: Mutex::new(HashSet::new()),
            library_id,
            backup_dirty: AtomicBool::new(backup_dirty),
            hard_links,
            dup_search: Mutex::new(None),
            page_answers: Mutex::new(Vec::new()),
            meta_wake: Condvar::new(),
            writer: w_tx.clone(),
            scanner: s_tx.clone(),
        });
        let mut dirs = plugin_dirs.to_vec();
        dirs.push(paths.plugins.clone());
        load_plugins(&db, &ctx, &dirs);
        let loaded = ctx.contrib.read().readers.clone();
        match db.requeue_changed_readers(&loaded) {
            Ok(0) => {}
            Ok(n) => logging::info(&format!("{n} file(s) queued to be read again by readers that changed")),
            Err(e) => logging::warn(&format!("readers that changed: {e}")),
        }

        let shutdown = Arc::new(AtomicBool::new(false));
        let thumbq = Arc::new(ThumbQueue { inner: Mutex::new(TQ::default()), cv: Condvar::new() });

        {
            let ctx = ctx.clone();
            let dbw = Db::open(&paths.db)?;
            std::thread::Builder::new().name("writer".into()).spawn(move || writer_loop(dbw, w_rx, ctx)).map_err(|e| e.to_string())?;
        }
        {
            let ctx = ctx.clone();
            let dbs = Db::open(&paths.db)?;
            let s_tx2 = s_tx.clone();
            let sd = shutdown.clone();
            std::thread::Builder::new()
                .name("scanner".into())
                .spawn(move || scanner_loop(dbs, s_rx, s_tx2, ctx, sd))
                .map_err(|e| e.to_string())?;
        }
        for i in 0..settings.thumb_workers.max(1) {
            let ctx = ctx.clone();
            let dbt = Db::open(&paths.db)?;
            let q = thumbq.clone();
            let w = w_tx.clone();
            let sd = shutdown.clone();
            std::thread::Builder::new()
                .name(format!("thumb-{i}"))
                .spawn(move || thumb_loop(dbt, q, w, ctx, sd))
                .map_err(|e| e.to_string())?;
        }
        {
            let ctx = ctx.clone();
            let dbm = Db::open(&paths.db)?;
            let w = w_tx.clone();
            let sd = shutdown.clone();
            std::thread::Builder::new().name("metadata".into()).spawn(move || meta_loop(dbm, w, ctx, sd)).map_err(|e| e.to_string())?;
        }
        let mut warnings = vec![];
        if !hard_links {
            warnings.push("This drive can't link files, so items in several folders are copied into each.".to_string());
        }
        if crate::core::system::filesystem::volume_hint(root) == Some("network") {
            warnings.push("This library is on a network drive and can be damaged if the connection drops. A local drive is safer.".to_string());
        }
        let lib = Library { ctx, db, writer: w_tx, scanner: s_tx, events: ev_rx, thumbq, shutdown, downloads: std::sync::OnceLock::new(), problems, warnings };
        if !replaced.is_empty() {
            lib.kick_thumbs();
        }
        lib.kick_file_ops();
        let _ = lib.scanner.send(ScanJob::CheckStore);
        // reconcile everything at startup; watchers are attached after each root's scan
        lib.scan_all();
        Ok(lib)
    }

    pub fn paths(&self) -> &LibraryPaths {
        &self.ctx.paths
    }

    pub fn ctx_scan_options(&self) -> ScanOptions {
        self.ctx.scan_options()
    }

    pub fn hard_links(&self) -> bool {
        self.ctx.hard_links
    }

    pub fn settings(&self) -> LibrarySettings {
        self.ctx.settings.read().clone()
    }

    /// Merges changed keys into the stored settings JSON so unknown keys and unparsed enum values from newer builds survive.
    pub fn patch_settings(&self, patch: serde_json::Map<String, serde_json::Value>) {
        let mut raw = self.ctx.settings_raw.write();
        for (k, v) in patch {
            raw.insert(k, v);
        }
        *self.ctx.settings.write() = LibrarySettings::from_map(&raw);
        let json = serde_json::to_string(&*raw).unwrap_or_default();
        drop(raw);
        self.write(move |db, _| {
            db.meta_set("settings", &json)?;
            Ok(Changed::default())
        });
    }

    /// A receiver for background events.
    pub fn subscribe(&self) -> Receiver<LibEvent> {
        self.events.clone()
    }

    /// Run an edit on the writer thread. The UI never blocks on writes.
    pub fn write(&self, f: impl FnOnce(&mut Db, &Ctx) -> DbResult<Changed> + Send + 'static) {
        let _ = self.writer.send(WriterJob::Run(Box::new(f)));
    }

    /// Run an edit on the writer thread; the receiver gets its result once the transaction is committed or rolled back.
    pub fn write_reply<T: Send + 'static>(&self, f: impl FnOnce(&mut Db, &Ctx) -> DbResult<(T, Changed)> + Send + 'static) -> Receiver<DbResult<T>> {
        let slot = Arc::new(Mutex::new(None));
        let put = slot.clone();
        let (tx, rx) = crossbeam_channel::bounded(1);
        let run: WriteFn = Box::new(move |db, ctx| {
            let (t, ch) = f(db, ctx)?;
            *put.lock() = Some(t);
            Ok(ch)
        });
        let done: DoneFn = Box::new(move |r| {
            let _ = tx.send(r.and_then(|_| slot.lock().take().ok_or_else(|| "The edit gave no result".to_string())));
        });
        let _ = self.writer.send(WriterJob::RunThen(run, done));
        rx
    }

    /// Forget trashed items (`None`: the whole trash). Their managed copies then go to the Recycle Bin on the scanner thread.
    pub fn purge(&self, ids: Option<Vec<AssetId>>) {
        self.write(move |db, ctx| {
            let ids = match ids {
                Some(ids) => ids,
                None => db.conn.prepare("SELECT id FROM assets WHERE deleted = 1").and_then(|mut st| st.query_map([], |r| r.get(0))?.collect()).map_err(|e| e.to_string())?,
            };
            purge_in_jobs(db, ctx, ids)
        });
    }

    pub fn kick_file_ops(&self) {
        let _ = self.scanner.send(ScanJob::FileOps);
    }

    /// The library's downloads, read from the app's data in `data_dir` the first time, when unfinished ones go on.
    pub fn downloads(&self, data_dir: &Path, hub: Option<Arc<crate::core::api::events::Hub>>) -> Arc<crate::core::import::downloads::Queue> {
        self.downloads.get_or_init(|| crate::core::import::downloads::Queue::open(data_dir, self.paths(), self.poster(), hub)).clone()
    }

    /// Downloads waiting or running.
    pub fn downloading(&self) -> bool {
        self.downloads.get().is_some_and(|q| q.busy())
    }

    /// Hands work to this library's job thread from elsewhere.
    pub fn poster(&self) -> Poster {
        Poster { scanner: self.scanner.clone(), thumbq: self.thumbq.clone() }
    }

    /// Run a long job (import, export, relink) on the scanner thread.
    pub fn job(&self, f: impl FnOnce(&mut Db, &Ctx) -> DbResult<Changed> + Send + 'static) {
        let _ = self.scanner.send(ScanJob::Run(Box::new(f)));
    }

    pub fn scan_root(&self, id: i64) {
        let _ = self.scanner.send(ScanJob::Root(id));
    }

    pub fn scan_all(&self) {
        let _ = self.scanner.send(ScanJob::All);
    }

    pub fn maintenance(&self) {
        let _ = self.scanner.send(ScanJob::Maintenance);
    }

    /// Back up now on the scanner thread; the result arrives as a `done` or `error` event.
    pub fn backup_now(&self) {
        let _ = self.scanner.send(ScanJob::Backup);
    }

    pub fn cancel_jobs(&self) {
        self.ctx.cancel.store(true, Ordering::Relaxed);
    }

    pub fn import_state(&self) -> Option<ImportState> {
        self.ctx.import.lock().clone()
    }

    pub fn busy(&self) -> Option<String> {
        self.ctx.busy.lock().clone()
    }

    pub fn thumbs_pending(&self) -> usize {
        let q = self.thumbq.inner.lock();
        q.urgent.len() + q.background.len() + q.in_flight.len()
    }

    /// Visible / selected items jump the thumbnail queue.
    pub fn prioritize_thumbs(&self, ids: &[AssetId]) {
        if ids.is_empty() {
            return;
        }
        let mut q = self.thumbq.inner.lock();
        for id in ids.iter().rev() {
            if q.in_flight.contains(id) {
                continue;
            }
            if let Some(pos) = q.urgent.iter().position(|x| x == id) {
                q.urgent.remove(pos);
            }
            q.urgent.push_front(*id);
            q.queued.insert(*id);
        }
        while q.urgent.len() > 400 {
            if let Some(old) = q.urgent.pop_back() {
                q.queued.remove(&old);
            }
        }
        q.exhausted_at = None;
        drop(q);
        self.thumbq.cv.notify_all();
    }

    /// Queues a thumbnail whose cache file is missing on disk so it is made again under a fresh version.
    pub fn remake_missing_thumb(&self, uid: String, ver: i64) {
        let thumbq = self.thumbq.clone();
        let slot = Arc::new(Mutex::new(None));
        let put = slot.clone();
        let run: WriteFn = Box::new(move |db, _| {
            let id: Option<AssetId> = db.conn.query_row("SELECT id FROM assets WHERE uid = uid_bytes(?1) AND thumb_state = 1 AND thumb_ver = ?2 AND status = 0", rusqlite::params![uid, ver], |r| r.get(0)).ok();
            if let Some(id) = id {
                let n = db.conn.execute("UPDATE assets SET thumb_state = 0, thumb_ver = thumb_ver + 1 WHERE id = ?1 AND thumb_state = 1 AND thumb_ver = ?2", rusqlite::params![id, ver]).map_err(|e| e.to_string())?;
                if n > 0 {
                    *put.lock() = Some(id);
                }
            }
            Ok(Changed::default())
        });
        let done: DoneFn = Box::new(move |res| {
            if res.is_ok() {
                if let Some(id) = *slot.lock() {
                    let mut q = thumbq.inner.lock();
                    if !q.in_flight.contains(&id) {
                        q.urgent.push_front(id);
                        q.queued.insert(id);
                        q.exhausted_at = None;
                    }
                    drop(q);
                    thumbq.cv.notify_all();
                }
            }
        });
        let _ = self.writer.send(WriterJob::RunThen(run, done));
    }

    /// Wake thumbnail workers to look for new work (after scans).
    pub fn kick_thumbs(&self) {
        self.thumbq.inner.lock().exhausted_at = None;
        self.thumbq.cv.notify_all();
    }

    /// A page extractor's answer from the window, applied by the metadata thread while the file is still queued for that reader.
    pub fn page_answered(&self, a: crate::core::library::metadata::PageAnswer) {
        self.ctx.page_answers.lock().push(a);
        self.ctx.meta_wake.notify_one();
    }

    /// The readers a plugin page runs, whose files the window works through.
    pub fn page_readers(&self) -> Vec<String> {
        self.ctx.contrib.read().readers.iter().filter(|r| r.page().is_some()).map(|r| r.id.clone()).collect()
    }

    pub fn contributions(&self) -> Contributions {
        self.ctx.contrib.read().clone()
    }

    pub fn plugins(&self) -> Vec<Plugin> {
        self.ctx.plugins.read().clone()
    }

    /// Rebuilds the full-text rows of the items holding these fields' values, a chunk per edit so other edits get their turn.
    pub fn reindex_text(&self, field_ids: Vec<i64>) {
        let ids = match self.db.holders(&field_ids) {
            Ok(ids) => ids,
            Err(e) => return logging::warn(&format!("search index: {e}")),
        };
        for chunk in ids.chunks(2000) {
            let chunk = chunk.to_vec();
            self.write(move |db, _| db.refresh_fts(&chunk).map(|_| Changed::default()));
        }
    }

    /// Queues readers for their files again (`scope`: only these items) and follows the queue as a job, "Reading metadata", which stops the read when cancelled. The receiver gets the job, none when nothing was queued, and how many files, once they are queued.
    pub fn rescan(&self, readers: Vec<readers::ReaderInfo>, scope: Option<Vec<AssetId>>, mode: Mode, hub: Option<Arc<crate::core::api::events::Hub>>, plugin: Option<String>) -> Receiver<DbResult<(Option<String>, usize)>> {
        let ids: Vec<String> = readers.iter().map(|r| r.id.clone()).collect();
        let within = scope.clone();
        let keys: Vec<String> = readers.iter().flat_map(|r| r.keys.clone()).collect();
        let queued = self.write_reply(move |db, _| {
            let n = db.queue_readers(&readers, within.as_deref(), mode)?;
            // the fields switched on to be read later are read now
            for mut f in db.fields()?.into_iter().filter(|f| f.needs_scan && keys.contains(&f.key)) {
                f.needs_scan = false;
                db.define_field(&f)?;
            }
            Ok(((n, db.queue_left(&ids, within.as_deref(), true), ids), Changed::parts(Parts::FIELDS)))
        });
        let (tx, rx) = crossbeam_channel::bounded(1);
        let (writer, shutdown, path, ctx) = (self.writer.clone(), self.shutdown.clone(), self.ctx.paths.db.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let (n, total, ids) = match queued.recv().unwrap_or_else(|_| Err("The library closed".into())) {
                Ok(v) => v,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            if total == 0 {
                let _ = tx.send(Ok((None, n)));
                return;
            }
            let (job, cancel) = crate::core::api::jobs::start(hub, plugin.as_deref(), "Reading metadata", total as f64);
            let _ = tx.send(Ok((Some(job.clone()), n)));
            let Ok(db) = Db::open(&path) else { return };
            loop {
                std::thread::sleep(Duration::from_millis(500));
                if shutdown.load(Ordering::Relaxed) {
                    let _ = crate::core::api::jobs::finish(&job, None, None, Some("Paused until the library reopens".into()));
                    return;
                }
                if cancel.load(Ordering::Relaxed) {
                    let _ = writer.send(WriterJob::Run(Box::new(move |db, _| db.unqueue(&ids, scope.as_deref()).map(|_| Changed::default()))));
                    return;
                }
                // its readers' plugin was switched off: the files stay queued for when it is back
                if !ctx.contrib.read().readers.iter().any(|r| ids.contains(&r.id)) {
                    let _ = crate::core::api::jobs::finish(&job, None, None, Some("The readers were switched off".into()));
                    return;
                }
                let now = db.queue_left(&ids, scope.as_deref(), true);
                let _ = crate::core::api::jobs::progress(&job, None, Some((total - now).max(0) as f64), None, None);
                if now == 0 {
                    let _ = crate::core::api::jobs::finish(&job, None, Some(serde_json::json!({ "files": total })), None);
                    return;
                }
            }
        });
        rx
    }

    /// On the writer, since loading defines plugin fields. The receiver gets a message once the plugins are loaded.
    pub fn reload_plugins(&self, extra_dirs: &[PathBuf]) -> Receiver<()> {
        let mut dirs = extra_dirs.to_vec();
        dirs.push(self.ctx.paths.plugins.clone());
        self.write(move |db, ctx| {
            load_plugins(db, ctx, &dirs);
            Ok(Changed::parts(Parts::FIELDS | Parts::SIDEBAR))
        });
        self.after_writes()
    }

    pub fn set_plugin(&self, id: &str, enabled: bool, granted: Vec<String>, extra_dirs: &[PathBuf]) -> Receiver<()> {
        let found = self.ctx.plugins.read().iter().find(|p| p.manifest.id == id).cloned();
        let version = found.as_ref().map(|p| p.manifest.version.clone()).unwrap_or_default();
        // switching it on here is the approval a plugin inside the library folder waits for
        if let Some(mut p) = found.filter(|p| enabled && p.dir.starts_with(&self.ctx.paths.plugins)) {
            p.granted = granted.clone();
            crate::core::import::auto_import::allow(&[plugins::loading::approval_print(&p)]);
        }
        let id = id.to_string();
        self.write(move |db, _| {
            if let Err(e) = db.set_plugin_state(&id, enabled, &version, &granted) {
                logging::error(&e);
            }
            db.plugin_failure(&id, true);
            Ok(Changed::default())
        });
        self.reload_plugins(extra_dirs)
    }

    pub fn remove_plugin(&self, id: &str, extra_dirs: &[PathBuf]) -> Receiver<()> {
        let id = id.to_string();
        self.write(move |db, _| {
            if let Err(e) = db.remove_plugin_state(&id) {
                logging::error(&e);
            }
            Ok(Changed::default())
        });
        self.reload_plugins(extra_dirs)
    }

    /// Runs a plugin action's program on items. The answer is the message the program gave.
    pub fn run_action(&self, plugin_id: String, action: plugins::manifest::ActionDecl, ids: Vec<AssetId>) -> Receiver<Result<Option<String>, String>> {
        let scope = ids.clone();
        self.run_program(plugin_id, action.title, action.run, action.dir, "action", ids, serde_json::json!({ "action": action.id }), scope)
    }

    /// Hands addresses to the plugin program that takes them over (`urls` in its manifest), with where new items should go.
    pub fn run_url_program(&self, plugin_id: String, handler: plugins::manifest::UrlDecl, urls: Vec<String>, place: serde_json::Value) -> Receiver<Result<Option<String>, String>> {
        self.run_program(plugin_id, handler.title, handler.run, handler.dir, "url", vec![], serde_json::json!({ "handler": handler.id, "urls": urls, "place": place }), vec![])
    }

    /// On a thread of its own, as the program may call the API, imports included, while it runs; its answer's changes go through the writer.
    #[allow(clippy::too_many_arguments)]
    fn run_program(&self, plugin_id: String, title: String, argv: Vec<String>, dir: Option<PathBuf>, event: &'static str, ids: Vec<AssetId>, extra: serde_json::Value, scope: Vec<AssetId>) -> Receiver<Result<Option<String>, String>> {
        let ctx = self.ctx.clone();
        let (tx, rx) = crossbeam_channel::bounded(1);
        let spawned = std::thread::Builder::new().name("plugin-run".into()).spawn(move || {
            let out = (|| {
                let plugin = ctx.plugins.read().iter().find(|p| p.manifest.id == plugin_id).cloned().ok_or("That plugin isn't loaded")?;
                if !plugin.active() || !plugin.has("exec") {
                    return Err(format!("{} may not run programs", plugin.manifest.name));
                }
                let mut payload = plugin_payload(&Db::open(&ctx.paths.db)?, &plugin, event, &ids);
                if let (Some(p), Some(x)) = (payload.as_object_mut(), extra.as_object()) {
                    p.extend(x.clone());
                }
                match plugins::run::run_command(&plugin_id, &argv, dir.as_deref(), &payload, PROGRAM_TIMEOUT) {
                    Ok(reply) => {
                        plugin_succeeded(&ctx, &plugin_id);
                        let message = reply.message.clone();
                        if !reply.set.is_empty() || message.is_some() {
                            // the answer comes once its changes are in, so the next step of an action sees them
                            let (done, applied) = crossbeam_channel::bounded(1);
                            ctx.write(move |db, ctx| {
                                let r = apply_reply(db, &plugin, reply, ctx, &scope);
                                let _ = done.send(());
                                r
                            });
                            let _ = applied.recv_timeout(Duration::from_secs(30));
                        }
                        Ok(message)
                    }
                    Err(e) => {
                        record_failure(&ctx, &plugin_id, &e);
                        Err(format!("{title}: {e}"))
                    }
                }
            })();
            if let Err(e) = &out {
                ctx.send(LibEvent::Error(e.clone().into()));
            }
            let _ = tx.send(out);
        });
        if let Err(e) = spawned {
            let (tx, rx) = crossbeam_channel::bounded(1);
            let _ = tx.send(Err(e.to_string()));
            return rx;
        }
        rx
    }

    /// Gets a message once every edit queued so far is committed.
    pub fn after_writes(&self) -> Receiver<()> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        self.write(move |_, _| {
            let _ = tx.send(());
            Ok(Changed::default())
        });
        rx
    }

    /// Wait until the writer has saved everything queued so far, at most `limit`.
    pub fn flush(&self, limit: Duration) -> bool {
        self.after_writes().recv_timeout(limit).is_ok()
    }

    pub fn shutdown(&self) {
        // edits made just before closing are saved, not dropped with the process
        if !self.shutdown.load(Ordering::Relaxed) && !self.flush(Duration::from_secs(5)) {
            logging::warn("closing before every edit was saved");
        }
        if let Some(q) = self.downloads.get() {
            q.close();
        }
        self.shutdown.store(true, Ordering::Relaxed);
        self.ctx.cancel.store(true, Ordering::Relaxed);
        self.thumbq.cv.notify_all();
        self.ctx.meta_wake.notify_all();
        let _ = self.db.conn.execute_batch("PRAGMA optimize;");
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        self.shutdown();
    }
}

