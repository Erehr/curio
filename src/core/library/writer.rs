//! The writer thread: edits and workers' results batched into short transactions, one writer at a time.

use std::sync::Arc;
use std::time::Duration;
use crossbeam_channel::Receiver;
use crate::core::db::{Db, DbResult};
use crate::core::system::logging;
use crate::core::model::*;
use crate::core::library::changes::Aspect;
use crate::core::library::changes::Changed;
use crate::core::library::metadata::apply_extracted;
use crate::core::library::LibEvent;
use crate::core::library::WriteFn;
use crate::core::library::Ctx;
use crate::core::library::WriterJob;

/// One corrupt file or bad job must not stop a worker for the rest of the session. The panic hook has already logged the details.
pub(super) fn guard<T>(what: &str, f: impl FnOnce() -> T) -> Option<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => Some(v),
        Err(_) => {
            logging::error(&format!("{what} failed unexpectedly; the worker carries on"));
            None
        }
    }
}

fn flush_guarded(db: &mut Db, results: &mut Vec<WriterJob>, ctx: &Ctx, ready: &mut Vec<AssetId>) {
    if guard("saving worker results", || flush_results(db, results, ctx, ready)).is_none() {
        db.rollback_open();
        results.clear();
    }
}

pub(super) fn writer_loop(mut db: Db, rx: Receiver<WriterJob>, ctx: Arc<Ctx>) {
    let mut pending_ready: Vec<AssetId> = Vec::new();
    while let Ok(first) = rx.recv() {
        let mut batch = vec![first];
        while batch.len() < 256 {
            match rx.try_recv() {
                Ok(j) => batch.push(j),
                Err(_) => break,
            }
        }
        // worker results go into one transaction; user edits run individually in order
        let mut results: Vec<WriterJob> = Vec::new();
        for job in batch {
            match job {
                WriterJob::Run(f) => {
                    flush_guarded(&mut db, &mut results, &ctx, &mut pending_ready);
                    let _ = run_edit(&mut db, &ctx, f);
                }
                WriterJob::RunThen(f, done) => {
                    flush_guarded(&mut db, &mut results, &ctx, &mut pending_ready);
                    done(run_edit(&mut db, &ctx, f));
                }
                other => results.push(other),
            }
        }
        flush_guarded(&mut db, &mut results, &ctx, &mut pending_ready);
    }
}

/// Wait for a reply from `Library::write_reply`; on timeout the edit stays queued.
pub fn reply_within<T>(rx: &Receiver<DbResult<T>>, timeout: Duration) -> DbResult<T> {
    match rx.recv_timeout(timeout) {
        Ok(r) => r,
        Err(crossbeam_channel::RecvTimeoutError::Timeout) => Err("The library is busy; the change is still queued".into()),
        Err(_) => Err("The library closed before the change was made".into()),
    }
}

/// One transaction per edit: a bulk change lands whole or not at all.
fn run_edit(db: &mut Db, ctx: &Ctx, f: WriteFn) -> DbResult<()> {
    match guard("an edit", || db.in_tx(|db| f(db, ctx))) {
        Some(Ok(ch)) => {
            // the scanner reads the queue on a connection of its own, so only once the edit is committed
            if ch.purged {
                ctx.kick_file_ops();
            }
            if !ch.is_empty() {
                ctx.send(LibEvent::Changed(ch));
            }
            if db.store_dirty_any() {
                ctx.kick_store();
            }
            Ok(())
        }
        Some(Err(e)) => {
            logging::error(&e);
            ctx.send(LibEvent::Error(e.clone().into()));
            Err(e)
        }
        None => {
            db.rollback_open();
            let e = "An edit failed unexpectedly. Details are in curio.log.".to_string();
            ctx.send(LibEvent::Error(e.clone().into()));
            Err(e)
        }
    }
}

fn flush_results(db: &mut Db, results: &mut Vec<WriterJob>, ctx: &Ctx, ready: &mut Vec<AssetId>) {
    if results.is_empty() {
        return;
    }
    let jobs = std::mem::take(results);
    let cat = db.field_catalog();
    let mut meta_changed = Vec::new();
    let mut searchable = Vec::new();
    let mut reshaped = false;
    let r = db.tx(|db| {
        for j in jobs {
            match j {
                WriterJob::ThumbWaiting { id } => db.set_thumb_waiting(id)?,
                WriterJob::ThumbPage { id } => db.set_thumb_page(id)?,
                WriterJob::ThumbDone { id, ok, dims, aspect, palette, look, duration, title } => {
                    db.set_thumb_done(id, ok, dims, if palette.is_empty() { None } else { Some(&palette) }, look.as_ref())?;
                    if let Some(d) = duration {
                        db.set_duration(id, d)?;
                    }
                    reshaped |= dims.is_some() || aspect.is_some_and(|a| a.is_finite() && a > 0.0);
                    if let Some((w, h)) = dims {
                        // images/videos: real dimensions come from the probe or decoded picture
                        db.conn.execute("UPDATE assets SET width = ?, height = ?, aspect = ? WHERE id = ? AND (width IS NULL OR kind <> 'image')",
                            rusqlite::params![w, h, w as f64 / (h.max(1) as f64), id]).map_err(|e| e.to_string())?;
                    } else if let Some(asp) = aspect.filter(|a| a.is_finite() && *a > 0.0) {
                        // non-visual items (books, comics, PDFs, links) take their cover's shape for the grid without storing thumbnail dimensions
                        db.conn.execute("UPDATE assets SET aspect = ?, width = CASE WHEN kind IN ('image', 'video') THEN width ELSE NULL END, height = CASE WHEN kind IN ('image', 'video') THEN height ELSE NULL END WHERE id = ?",
                            rusqlite::params![asp, id]).map_err(|e| e.to_string())?;
                    }
                    if let Some(t) = title.filter(|t| !t.trim().is_empty()) {
                        db.conn.execute("UPDATE assets SET display_name = ? WHERE id = ? AND display_name = ''", rusqlite::params![t.trim(), id]).map_err(|e| e.to_string())?;
                        searchable.push(id);
                    }
                    ready.push(id);
                }
                WriterJob::Meta { id, plan, whole, done, ran, failed, duration, dims, later } => {
                    if apply_extracted(db, &cat, id, &plan)? {
                        searchable.push(id);
                    }
                    if let Some(d) = duration {
                        db.set_duration(id, d)?;
                    }
                    if let Some((w, h)) = dims {
                        db.set_dims(id, w, h)?;
                    }
                    db.meta_read(id, &done, &ran, &failed, whole)?;
                    db.queue_item(id, &later)?;
                    meta_changed.push(id);
                }
                WriterJob::Run(_) | WriterJob::RunThen(..) => {}
            }
        }
        db.refresh_fts(&searchable)
    });
    if let Err(e) = r {
        logging::error(&format!("writer batch failed: {e}"));
    }
    for id in ready.drain(..) {
        ctx.send(LibEvent::ThumbReady(id));
    }
    // the covers made were told of as ready just now
    if reshaped {
        ctx.send(LibEvent::Changed(Changed::reshaped(&[])));
    }
    if !meta_changed.is_empty() {
        ctx.send(LibEvent::Changed(Changed::shown(&meta_changed, Aspect::File)));
    }
}
