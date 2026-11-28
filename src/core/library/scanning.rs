//! The scanner thread: locations read, the store kept in step with the index, auto-import.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use crossbeam_channel::{unbounded, Receiver, Sender};
use serde_json::{json, Value};
use crate::core::db::Db;
use crate::core::system::logging;
use crate::core::model::*;
use crate::core::said::Said;
use crate::core::scan::{self};
use crate::core::scan::watcher::{FsWatcher, WatchBatch};
use crate::core::library::ScanNote;
use crate::core::library::changes::Parts;
use crate::core::library::changes::Aspect;
use crate::core::library::upkeep::run_backup;
use crate::core::library::upkeep::backup_due;
use crate::core::library::upkeep::run_maintenance;
use crate::core::library::writer::guard;
use crate::core::library::changes::Changed;
use crate::core::library::LibEvent;
use crate::core::library::ScanJob;
use crate::core::library::Ctx;

/// Follow changes made in the store outside Curio (`only` = the notified paths, `None` = everything), then arrange what they queued.
fn reconcile_store(db: &mut Db, ctx: &Ctx, self_tx: &Sender<ScanJob>, only: Option<&[PathBuf]>) {
    *ctx.busy.lock() = Some("Checking the library's files".into());
    match crate::core::store::outside::reconcile(db, &ctx.paths, only, ctx.hard_links, crate::core::model::now_ms()) {
        Ok(mut r) => {
            match crate::core::grouping::companions::after_store_items(db, &ctx.paths, &r.adopted, &ctx.scan_options().companions) {
                Ok(pairs) => {
                    r.adopted.retain(|id| !pairs.iter().any(|p| p.1 == *id));
                    r.changed.extend(pairs.iter().map(|p| p.0));
                }
                Err(e) => logging::warn(&format!("grouping files added to the library's folder: {e}")),
            }
            if !r.is_empty() {
                ctx.send(LibEvent::Changed(Changed::files(&r.changed).and(Changed::added(&r.adopted, "scan"))));
            }
            if !r.adopted.is_empty() {
                ctx.thumbs_wanted.store(true, Ordering::Relaxed);
                logging::info(&format!("{} file(s) added to the library's folder by hand became items", r.adopted.len()));
            }
            for p in &r.problems {
                logging::warn(&format!("files changed outside Curio: {p}"));
            }
            if let Some(first) = r.problems.first() {
                ctx.send(LibEvent::Error(Said::new("errors.watch.notFollowed", json!({"error": first}), format!("A change made in the library's folder couldn't be followed: {first}"))));
            }
            if !r.unsettled.is_empty() {
                // still being written: look again once they have been still for a while
                if let Some(root) = db.roots().unwrap_or_default().into_iter().find(|r| r.kind == RootKind::Managed) {
                    let (tx, paths) = (self_tx.clone(), r.unsettled);
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(crate::core::store::outside::SETTLE_MS as u64 + 500));
                        let _ = tx.send(ScanJob::Watch(WatchBatch { root_id: root.id, paths, rescan: false }));
                    });
                }
            }
        }
        Err(e) => logging::error(&format!("following changes in the library's folder: {e}")),
    }
    *ctx.busy.lock() = None;
    if db.store_dirty_any() {
        let _ = self_tx.send(ScanJob::SyncStore);
    }
}

/// Items per store pass, so scans and imports queued meanwhile get their turn during a large rearrangement.
const STORE_BATCH: i64 = 500;

/// A queued entry whose store pass failed is tried again after this pause, doubled at each further failure; the user is told when it fails for the `STORE_TRIES`th time, about half a minute on.
const STORE_RETRY_PAUSE: Duration = Duration::from_millis(500);

const STORE_TRIES: u32 = 7;

fn store_retry_pause(failures: u32) -> Duration {
    STORE_RETRY_PAUSE * 2u32.pow(failures - 1)
}

/// One store pass. `tries` counts the failed passes of each queued entry (`0` the folder tree) since it last moved; returns the entries that failed in this one.
fn sync_store(db: &mut Db, ctx: &Ctx, self_tx: &Sender<ScanJob>, tries: &mut HashMap<AssetId, u32>) -> Vec<AssetId> {
    if !db.store_dirty_any() {
        return vec![];
    }
    *ctx.busy.lock() = Some("Arranging the library's files".into());
    let style = ctx.settings.read().name_suffix;
    let mut failed: Vec<AssetId> = vec![];
    match crate::core::store::sync::sync(db, &ctx.paths, ctx.hard_links, style, STORE_BATCH) {
        Ok(r) => {
            if !r.moved.is_empty() {
                ctx.send(LibEvent::Changed(Changed::files(&r.moved)));
            }
            for id in &r.moved {
                tries.remove(id);
            }
            failed = r.problems.iter().map(|(id, _)| *id).collect();
            failed.dedup();
            // a virus scanner or another program holding a file open usually lets go within seconds
            let mut later: HashMap<u32, Vec<AssetId>> = HashMap::new();
            for id in &failed {
                let n = tries.entry(*id).or_insert(0);
                *n += 1;
                if *n < STORE_TRIES {
                    later.entry(*n).or_default().push(*id);
                } else {
                    tries.remove(id);
                }
            }
            let mut given_up = vec![];
            for (id, why) in &r.problems {
                match tries.get(id) {
                    Some(n) => logging::warn(&format!("arranging files: {why}; trying again in {:?}", store_retry_pause(*n))),
                    None => {
                        logging::warn(&format!("arranging files: {why}"));
                        given_up.push(why);
                    }
                }
            }
            for (n, ids) in later {
                let tx = self_tx.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(store_retry_pause(n));
                    let _ = tx.send(ScanJob::RetryStore(ids));
                });
            }
            if let Some(first) = given_up.first() {
                let said = Said::new("errors.store.notMoved", json!({"error": first}), format!("Some files couldn't be moved to match their folders: {first}"));
                ctx.send(LibEvent::Error(said.and_more(given_up.len())));
            }
            if r.more || db.store_dirty_any() {
                let _ = self_tx.send(ScanJob::SyncStore);
            }
        }
        Err(e) => logging::error(&format!("arranging files: {e}")),
    }
    *ctx.busy.lock() = None;
    failed
}

pub(super) fn scanner_loop(mut db: Db, rx: Receiver<ScanJob>, self_tx: Sender<ScanJob>, ctx: Arc<Ctx>, shutdown: Arc<AtomicBool>) {
    let (wtx, wrx) = unbounded::<WatchBatch>();
    let mut watcher = FsWatcher::start(wtx);
    // the store is always watched: files changed there outside Curio must reach the index
    if let Some(r) = db.roots().unwrap_or_default().into_iter().find(|r| r.kind == RootKind::Managed) {
        watcher.watch(r.id, PathBuf::from(&r.path));
    }
    // forward watch batches into our own queue
    {
        let tx = self_tx.clone();
        std::thread::Builder::new()
            .name("watch-forward".into())
            .spawn(move || {
                for b in wrx {
                    if tx.send(ScanJob::Watch(b)).is_err() {
                        break;
                    }
                }
            })
            .ok();
    }
    let mut last_maint = Instant::now();
    let mut last_online_check = Instant::now();
    let mut last_auto_import = Instant::now();
    let mut auto_import_gap = Duration::from_secs(crate::core::consts::AUTO_IMPORT_POLL_SECS);
    // `meta.backup_dirty` already says "changed"
    let mut dirty_saved = false;
    // not while the library is still being reconciled after opening
    let mut next_backup_check = Instant::now() + Duration::from_secs(120);
    let mut store_tries: HashMap<AssetId, u32> = HashMap::new();
    if db.meta_get("animated_webp_recheck").as_deref() != Some("1") {
        if db.reset_webp_animated().is_ok() {
            let _ = db.meta_set("animated_webp_recheck", "1");
        }
    }
    let mut animated_pending = true;
    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        let wait = if animated_pending { Duration::from_millis(50) } else { Duration::from_secs(4) };
        let job = match rx.recv_timeout(wait) {
            Ok(ScanJob::Shutdown) => break,
            Ok(j) => Some(j),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => None,
            Err(_) => break,
        };
        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        let maint_every = Duration::from_secs(ctx.settings.read().maintenance_minutes.max(5) * 60);
        if last_maint.elapsed() > maint_every {
            last_maint = Instant::now();
            guard("maintenance", || run_maintenance(&mut db, &ctx));
        }
        if ctx.backup_dirty.load(Ordering::Relaxed) && !dirty_saved {
            dirty_saved = db.meta_set("backup_dirty", "1").is_ok();
        }
        if Instant::now() >= next_backup_check {
            next_backup_check = Instant::now() + Duration::from_secs(60);
            if backup_due(&db, &ctx) {
                if guard("backup", || run_backup(&db, &ctx, "scheduled")).is_some_and(|r| r.is_ok()) {
                    dirty_saved = false;
                } else {
                    // retried after an hour, not on every check
                    next_backup_check = Instant::now() + Duration::from_secs(3600);
                }
            }
        }
        if last_auto_import.elapsed() > auto_import_gap {
            last_auto_import = Instant::now();
            if guard("auto-import", || auto_import(&mut db, &ctx)).is_none() {
                db.rollback_open();
            }
            // a big watched tree is walked less often: a pass may not take over 5 % of the time
            auto_import_gap = Duration::from_secs(crate::core::consts::AUTO_IMPORT_POLL_SECS).max(last_auto_import.elapsed() * 20);
            animated_pending = true;
        }
        if animated_pending {
            match db.backfill_animated(ANIMATED_BATCH) {
                Ok((checked, found)) => {
                    animated_pending = checked as i64 >= ANIMATED_BATCH;
                    if !found.is_empty() {
                        ctx.send(LibEvent::Changed(Changed::shown(&found, Aspect::File)));
                    }
                }
                Err(_) => animated_pending = false,
            }
        }
        // removable / network storage: notice roots that come back online or go away
        if last_online_check.elapsed() > Duration::from_secs(30) {
            last_online_check = Instant::now();
            for r in db.roots().unwrap_or_default().into_iter().filter(|r| r.enabled && r.kind != RootKind::Unknown) {
                // an empty mount point stays offline (see `scan_root`)
                let now_online = crate::core::system::filesystem::root_online(Path::new(&r.path)) && (r.online || r.explicit_only || std::fs::read_dir(&r.path).is_ok_and(|mut d| d.next().is_some()));
                if now_online != r.online {
                    if now_online {
                        logging::info(&format!("storage for {} is back; reconciling", r.label));
                        let _ = self_tx.send(ScanJob::Root(r.id));
                    } else {
                        let _ = db.set_root_online(r.id, false);
                        watcher.unwatch(r.id);
                        ctx.send(LibEvent::Changed(Changed::parts(Parts::LOCATIONS)));
                    }
                }
            }
        }
        let Some(job) = job else { continue };
        ctx.cancel.store(false, Ordering::Relaxed);
        let finished = guard("a background job", || match job {
            ScanJob::All => {
                for r in db.roots().unwrap_or_default().into_iter().filter(|r| r.enabled && r.kind != RootKind::Unknown) {
                    scan_one(&mut db, &ctx, &mut watcher, &self_tx, r.id);
                }
            }
            ScanJob::Root(id) => scan_one(&mut db, &ctx, &mut watcher, &self_tx, id),
            ScanJob::Watch(b) => {
                let Some(root) = db.root(b.root_id) else { return };
                if root.kind == RootKind::Unknown {
                    return;
                }
                if root.kind == RootKind::Managed {
                    reconcile_store(&mut db, &ctx, &self_tx, (!b.rescan).then_some(b.paths.as_slice()));
                } else if b.rescan {
                    scan_one(&mut db, &ctx, &mut watcher, &self_tx, root.id);
                } else {
                    let opts = ctx.scan_options();
                    match scan::reconcile_paths(&mut db, &root, &b.paths, &opts) {
                        Ok((_, events)) => {
                            if !events.is_empty() {
                                ctx.send(LibEvent::Changed(Changed::of_scan(&events)));
                            }
                        }
                        Err(e) => logging::error(&e),
                    }
                }
            }
            ScanJob::Maintenance => {
                run_maintenance(&mut db, &ctx);
                ctx.send(LibEvent::Done(Said::new("results.maintenance.done", Value::Null, "Maintenance finished")));
            }
            ScanJob::Backup => {
                if let Ok(dir) = run_backup(&db, &ctx, "manual") {
                    ctx.send(LibEvent::Done(Said::new("results.backup.saved", json!({"folder": dir.display().to_string()}), format!("Backup saved to {}", dir.display()))));
                }
            }
            ScanJob::SyncStore => {
                sync_store(&mut db, &ctx, &self_tx, &mut store_tries);
            }
            ScanJob::RetryStore(ids) => {
                if let Err(e) = db.mark_store_dirty(&ids) {
                    logging::error(&format!("arranging files: {e}"));
                }
                let failed = sync_store(&mut db, &ctx, &self_tx, &mut store_tries);
                for id in ids.iter().filter(|id| !failed.contains(id)) {
                    store_tries.remove(id);
                }
            }
            ScanJob::CheckStore => {
                reconcile_store(&mut db, &ctx, &self_tx, None);
                if let Err(e) = db.drop_empty_loose_roots() {
                    logging::error(&format!("removing unused locations: {e}"));
                }
                match db.mark_store_inconsistent().and_then(|n| Ok(n + crate::core::store::sync::mark_members_inconsistent(&db)?)) {
                    Ok(n) if n > 0 => logging::info(&format!("{n} item(s) in the library's files don't match their folders; arranging them")),
                    Ok(_) => {}
                    Err(e) => logging::error(&format!("checking the library's files: {e}")),
                }
                sync_store(&mut db, &ctx, &self_tx, &mut store_tries);
            }
            ScanJob::FileOps => {
                let moved = crate::core::store::recovery::recover(&mut db, &ctx.paths);
                if !moved.is_empty() {
                    ctx.send(LibEvent::Changed(Changed::files(&moved)));
                }
                *ctx.busy.lock() = Some("Moving deleted copies to the Recycle Bin".into());
                match crate::core::store::journal::run_recycles(&db, 100, &shutdown) {
                    Ok((_, failed, more)) => {
                        if failed > 0 {
                            ctx.send(LibEvent::Error(Said::new("errors.store.notRecycled", json!({"count": failed}), format!("{failed} deleted cop{} couldn't go to the Recycle Bin. See Settings › Storage & backup.", if failed == 1 { "y" } else { "ies" }))));
                        }
                        // the rest after any waiting scans
                        if more {
                            let _ = self_tx.send(ScanJob::FileOps);
                        }
                    }
                    Err(e) => logging::error(&e),
                }
                *ctx.busy.lock() = None;
            }
            ScanJob::Shutdown => {}
            ScanJob::Run(f) => {
                *ctx.busy.lock() = Some("Working".into());
                match f(&mut db, &ctx) {
                    Ok(ch) => ctx.send(LibEvent::Changed(ch)),
                    Err(e) => {
                        logging::error(&e);
                        ctx.send(LibEvent::Error(e.into()));
                    }
                }
                *ctx.busy.lock() = None;
                if db.store_dirty_any() {
                    let _ = self_tx.send(ScanJob::SyncStore);
                }
                // new roots may have been added by the job
                for r in db.roots().unwrap_or_default().into_iter().filter(|r| r.enabled && r.watch && r.kind == RootKind::Linked && !r.explicit_only) {
                    if ctx.settings.read().watch_folders && !watcher.is_watching(r.id) {
                        watcher.watch(r.id, PathBuf::from(&r.path));
                    }
                }
            }
        });
        if finished.is_none() {
            db.rollback_open();
            *ctx.busy.lock() = None;
            *ctx.import.lock() = None;
            ctx.send(LibEvent::Error(Said::new("errors.jobs.failed", Value::Null, "A background job failed unexpectedly. Details are in curio.log.")));
        }
    }
}

/// Copy new files from the auto-import folder into managed storage. A file is imported once
/// it has stopped changing (same size on two polls); imported paths are remembered so files
/// that are kept (delete-source off) are not imported again.
/// How many images each background tick checks for animation.
const ANIMATED_BATCH: i64 = 500;

fn auto_import(db: &mut Db, ctx: &Ctx) {
    let settings = ctx.settings.read().clone();
    let cfg = settings.auto_import.clone();
    if !cfg.enabled || cfg.watches.is_empty() {
        return;
    }
    let approved = crate::core::import::auto_import::load_approvals();
    match crate::core::import::auto_import::run_with(db, &ctx.paths, &cfg, &approved, settings.name_suffix, crate::core::model::now_ms(), &ctx.scan_options().companions) {
        Ok(mut r) => {
            match crate::core::grouping::companions::after_store_items(db, &ctx.paths, &r.ids, &ctx.scan_options().companions) {
                Ok(pairs) => {
                    r.ids.retain(|id| !pairs.iter().any(|p| p.1 == *id));
                    let older: Vec<AssetId> = pairs.iter().map(|p| p.0).filter(|i| !r.ids.contains(i)).collect();
                    if !older.is_empty() {
                        ctx.send(LibEvent::Changed(Changed::items(&older)));
                    }
                }
                Err(e) => logging::warn(&format!("auto-import: grouping files: {e}")),
            }
            for m in &r.messages {
                logging::warn(&format!("auto-import: {m}"));
            }
            if !r.ids.is_empty() {
                logging::info(&format!("auto-import: {} file(s)", r.ids.len()));
                ctx.send(LibEvent::Done(Said::new("results.autoImport.added", json!({"count": r.ids.len()}), format!("Auto-import: added {} item{}", r.ids.len(), if r.ids.len() == 1 { "" } else { "s" }))));
                ctx.send(LibEvent::Changed(Changed::added(&r.ids, "autoImport")));
            }
        }
        Err(e) => logging::error(&format!("auto-import: {e}")),
    }
}

/// How often a scan under way says how far it got.
const SCAN_NOTE_EVERY: Duration = Duration::from_secs(1);

fn scan_one(db: &mut Db, ctx: &Ctx, watcher: &mut FsWatcher, self_tx: &Sender<ScanJob>, root_id: i64) {
    let Some(root) = db.root(root_id) else { return };
    if root.kind == RootKind::Unknown {
        return;
    }
    if root.kind == RootKind::Managed {
        reconcile_store(db, ctx, self_tx, None);
        return;
    }
    let label = root.label.clone();
    *ctx.busy.lock() = Some(format!("Scanning {label}"));
    ctx.scan_started(root_id);
    let opts = ctx.scan_options();
    let started = Instant::now();
    let told = std::cell::Cell::new(Instant::now());
    let progress = |n: usize, what: &str| {
        *ctx.busy.lock() = Some(format!("Scanning {label}: {n} files ({what})"));
        if told.get().elapsed() >= SCAN_NOTE_EVERY {
            told.set(Instant::now());
            ctx.send(LibEvent::Scan(ScanNote { location: root_id, files: n, done: None }));
        }
    };
    // a first scan's items show as they are found
    let found = |ids: &[AssetId]| ctx.send(LibEvent::Changed(Changed { provisional: true, ..Changed::added(ids, "scan") }));
    match scan::scan_root_with(db, &root, &opts, &progress, &found, &ctx.cancel) {
        Ok((stats, events)) => {
            if stats.offline {
                watcher.unwatch(root.id);
                ctx.send(LibEvent::Progress(Said::new("results.locations.offline", json!({"location": label}), format!("{label} is offline; its items are marked unavailable"))));
            } else {
                if root.watch && root.kind == RootKind::Linked && !root.explicit_only && ctx.settings.read().watch_folders {
                    watcher.watch(root.id, PathBuf::from(&root.path));
                }
                let msg = format!(
                    "{label}: {} files checked in {:.1}s - {} new, {} changed, {} moved, {} missing",
                    stats.seen,
                    started.elapsed().as_secs_f32(),
                    stats.added,
                    stats.updated,
                    stats.moved,
                    stats.missing
                );
                logging::info(&msg);
                if stats.added + stats.updated + stats.moved + stats.missing > 0 {
                    let seconds = (started.elapsed().as_secs_f64() * 10.0).round() / 10.0;
                    let args = json!({"location": label, "count": stats.seen, "seconds": seconds, "added": stats.added, "changed": stats.updated, "moved": stats.moved, "missing": stats.missing});
                    let text = format!("{label}: checked {} files in {seconds} s. New: {}, changed: {}, moved: {}, missing: {}.", stats.seen, stats.added, stats.updated, stats.moved, stats.missing);
                    ctx.send(LibEvent::Done(Said::new("results.scan.summary", args, text)));
                }
            }
            ctx.send(LibEvent::Changed(Changed::of_scan(&events).with(Parts::LOCATIONS)));
            ctx.scan_ended(root_id, stats);
        }
        Err(e) => {
            logging::error(&e);
            ctx.send(LibEvent::Error(e.into()));
            ctx.scan_ended(root_id, Default::default());
        }
    }
    *ctx.busy.lock() = None;
}
