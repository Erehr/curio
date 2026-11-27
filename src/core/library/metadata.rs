//! The metadata thread: files read by their readers, and what they found applied to items.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use crossbeam_channel::Sender;
use crate::core::db::Db;
use crate::core::system::logging;
use crate::core::model::*;
use crate::core::plugins::{self};
use crate::core::query::fields::FieldCatalog;
use crate::core::metadata::readers::{self, Mode, Outcome, Reader};
use crate::core::library::plugin_host::record_failure;
use crate::core::library::writer::guard;
use crate::core::library::changes::Changed;
use crate::core::library::Ctx;
use crate::core::library::WriterJob;

/// Writes what readers found: fields, and `__source_url`, `__annotation` and `__title` into an item's source, notes and shown name when they are empty; answers whether searchable text changed. Fields that aren't indexed and values set by hand are left alone.
pub fn apply_extracted(db: &Db, cat: &crate::core::query::fields::FieldCatalog, id: AssetId, plan: &readers::Plan) -> Result<bool, String> {
    // grouped while it was being read: its item's readers read it now
    if db.conn.query_row("SELECT part_of IS NOT NULL FROM assets WHERE id = ?", [id], |r| r.get::<_, bool>(0)).unwrap_or(false) {
        return Ok(false);
    }
    let mut searchable = false;
    let (held, by_hand) = (db.fields_with_values(id), db.overridden_ids(id));
    for (k, v) in &plan.values {
        let column = match k.as_str() {
            "__source_url" => Some("source_url"),
            "__annotation" => Some("annotation"),
            "__title" => Some("display_name"),
            _ => None,
        };
        if let Some(column) = column {
            if let RawValue::Text(t) = v {
                db.conn.execute(&format!("UPDATE assets SET {column} = ? WHERE id = ? AND {column} = ''"), rusqlite::params![t, id]).map_err(|e| e.to_string())?;
                searchable = true;
            }
            continue;
        }
        let Some(f) = cat.custom(k).filter(|f| f.indexed) else { continue };
        if by_hand.contains(&f.id) || (plan.only_empty.contains(k) && held.contains(&f.id)) {
            continue;
        }
        if let Err(e) = db.set_field(&[id], f, v) {
            logging::warn(&format!("asset {id} field {k}: {e}"));
        }
    }
    for k in &plan.clear {
        let Some(f) = cat.custom(k).filter(|f| f.indexed) else { continue };
        if held.contains(&f.id) && !by_hand.contains(&f.id) {
            db.clear_field(&[id], f.id)?;
        }
    }
    Ok(searchable)
}

/// What a plugin's page read from one file in the window.
pub struct PageAnswer {
    pub id: AssetId,
    pub reader: String,
    pub mode: Mode,
    pub outcome: Outcome,
}

/// Reads in flight past this many wait for the writer to save them, so the set doesn't grow over a long rescan.
const INFLIGHT_MAX: usize = 4096;

/// New and changed files are read whole, by every reader that applies; then the files queued for chosen readers.
pub(super) fn meta_loop(db: Db, writer: Sender<WriterJob>, ctx: Arc<Ctx>, shutdown: Arc<AtomicBool>) {
    // a whole read is in flight under its item and an empty reader, a queued one under its item and reader
    let mut inflight: HashSet<(AssetId, String)> = HashSet::new();
    loop {
        if shutdown.load(Ordering::Relaxed) {
            return;
        }
        if inflight.len() > INFLIGHT_MAX {
            // the writer saves the results sent before this edit first, so what was in flight is written by the time it answers
            let (tx, rx) = crossbeam_channel::bounded(1);
            let _ = writer.send(WriterJob::Run(Box::new(move |_, _| {
                let _ = tx.send(());
                Ok(Changed::default())
            })));
            loop {
                match rx.recv_timeout(Duration::from_millis(500)) {
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) if !shutdown.load(Ordering::Relaxed) => {}
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => return,
                    _ => break,
                }
            }
            inflight.clear();
        }
        let readers = ctx.contrib.read().readers.clone();
        let has_ffprobe = crate::core::media::engine() == crate::core::media::Engine::Ffmpeg;
        // a cancelled read or a plugin switched off since takes the file off the queue, and its answer is dropped
        let mut answers: Vec<PageAnswer> = std::mem::take(&mut *ctx.page_answers.lock());
        // a file asked of its page twice before the first answer was applied is applied once
        let mut seen = HashSet::new();
        answers.retain(|a| seen.insert((a.id, a.reader.clone())));
        if !answers.is_empty() {
            let cat = db.field_catalog();
            for a in answers.into_iter().filter(|a| db.is_queued(a.id, &a.reader) && readers.iter().any(|r| r.id == a.reader)) {
                if let Some(job) = read_file(&db, &ctx, &readers, &cat, a.id, &[(a.reader.clone(), a.mode)], has_ffprobe, Some(&a.outcome)) {
                    let _ = writer.send(job);
                }
            }
        }
        let whole: Vec<AssetId> = db.pending_meta(64, &ctx.scanning()).unwrap_or_default().into_iter().filter(|id| !inflight.contains(&(*id, String::new()))).collect();
        let jobs: Vec<(AssetId, Vec<(String, Mode)>)> = if !whole.is_empty() {
            whole.into_iter().map(|id| (id, vec![])).collect()
        } else {
            // a page's files wait in the queue for the window
            let ids: Vec<String> = readers.iter().filter(|r| r.page().is_none()).map(|r| r.id.clone()).collect();
            let mut by: Vec<(AssetId, Vec<(String, Mode)>)> = vec![];
            for (id, r, m) in db.pending_queue(&ids, 256, &[]).unwrap_or_default() {
                if inflight.contains(&(id, r.clone())) {
                    continue;
                }
                match by.last_mut() {
                    Some((last, list)) if *last == id => list.push((r, m)),
                    _ => by.push((id, vec![(r, m)])),
                }
            }
            by
        };
        if jobs.is_empty() {
            inflight.clear();
            let mut guard = ctx.page_answers.lock();
            if guard.is_empty() && !shutdown.load(Ordering::Relaxed) {
                ctx.meta_wake.wait_for(&mut guard, Duration::from_millis(800));
            }
            continue;
        }
        let cat = db.field_catalog();
        for (id, queued) in jobs {
            if shutdown.load(Ordering::Relaxed) {
                return;
            }
            if queued.is_empty() {
                inflight.insert((id, String::new()));
            }
            inflight.extend(queued.iter().map(|(r, _)| (id, r.clone())));
            if let Some(job) = read_file(&db, &ctx, &readers, &cat, id, &queued, has_ffprobe, None) {
                let _ = writer.send(job);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Reads one file: whole when `queued` is empty, otherwise with the queued readers and those filling a key of theirs, so a value another reader gives isn't taken for one that is gone. `page` is what the queued page reader answered.
#[allow(clippy::too_many_arguments)]
fn read_file(db: &Db, ctx: &Ctx, all: &[Reader], cat: &FieldCatalog, id: AssetId, queued: &[(String, Mode)], has_ffprobe: bool, page: Option<&Outcome>) -> Option<WriterJob> {
    let a = db.asset(id).ok().flatten()?;
    let whole = queued.is_empty();
    let mode_of = |r: &Reader| if whole { Some(Mode::All) } else { queued.iter().find(|(q, _)| *q == r.id).map(|(_, m)| *m) };
    let chosen: Vec<&Reader> = all.iter().filter(|r| r.applies(&a.ext)).collect();
    let primary: Vec<&Reader> = chosen.iter().copied().filter(|r| mode_of(r).is_some()).collect();
    let chosen: Vec<&Reader> = chosen.into_iter().filter(|r| whole || mode_of(r).is_some() || r.keys.iter().any(|k| primary.iter().any(|p| p.fills(k)))).collect();
    let done: Vec<String> = queued.iter().map(|(r, _)| r.clone()).collect();
    let ran: Vec<String> = chosen.iter().map(|r| r.id.clone()).collect();
    // a reader none of whose fields is indexed isn't run
    let wanted = |r: &Reader| r.keys.is_empty() || r.keys.iter().any(|k| cat.custom(k).is_some_and(|f| f.indexed));
    let later: Vec<String> = if whole { chosen.iter().filter(|r| r.page().is_some() && wanted(r)).map(|r| r.id.clone()).collect() } else { vec![] };
    let read = guard("reading metadata", || {
        let path = PathBuf::from(&a.abs_path);
        let parts = if a.parts > 0 { readers::parts_of(db, id) } else { vec![] };
        let info = readers::FileInfo { ext: &a.ext, size: a.size, duration: a.duration, parts: &parts };
        let mut cache = readers::Cache::default();
        let runs: Vec<readers::Ran> = chosen
            .iter()
            .map(|r| {
                let plugin = matches!(r.source, readers::Source::Extractor(_));
                let outcome = if !wanted(r) {
                    Outcome::Skipped
                } else {
                    let got = match page {
                        Some(o) if r.page().is_some() && mode_of(r).is_some() => o.clone(),
                        _ => readers::run(r, &path, &info, &mut cache),
                    };
                    // a page's answer keeps only the fields its extractor lists, when it lists them
                    let declared = |k: &str| r.page().is_none() || r.keys.is_empty() || r.fills(k);
                    match got {
                        Outcome::Read(v) if plugin => Outcome::Read(v.into_iter().filter(|(k, _)| readers::ITEM_KEYS.contains(&k.as_str()) || (declared(k) && cat.custom(k).is_some_and(|f| plugins::may_write_field(&r.owner, f)))).collect()),
                        Outcome::Failed(e) => {
                            if plugin {
                                record_failure(ctx, &r.owner, &e);
                            }
                            Outcome::Failed(e)
                        }
                        o => o,
                    }
                };
                readers::Ran { reader: r, outcome, mode: mode_of(r) }
            })
            .collect();
        let failed: Vec<(String, String)> = runs.iter().filter_map(|r| if let Outcome::Failed(e) = &r.outcome { Some((r.reader.id.clone(), e.clone())) } else { None }).collect();
        let mut plan = readers::plan(&runs);
        // a plugin's program that lists no fields may have written any field it may write; one that didn't read the file this time leaves those alone
        for r in all.iter().filter(|r| r.keys.is_empty() && r.applies(&a.ext) && matches!(r.source, readers::Source::Extractor(_))) {
            if !runs.iter().any(|x| x.reader.id == r.id && matches!(x.outcome, Outcome::Read(_))) {
                plan.clear.retain(|k| !cat.custom(k).is_some_and(|f| plugins::may_write_field(&r.owner, f)));
            }
        }
        let (duration, dims) = if whole {
            let sound = if has_ffprobe && a.kind == AssetKind::Audio && a.duration.is_none() { crate::core::metadata::extract::probe_duration(&path) } else { None };
            // a video's size comes with its thumbnail, unless FFmpeg was too slow to read it then, as on its first runs while a Mac checks the new program
            let dims = match a.kind {
                _ if a.width.is_some() => None,
                AssetKind::Image => crate::core::metadata::extract::probe_dims(&path, &a.ext),
                AssetKind::Video => cache.dims,
                _ => None,
            };
            (cache.duration.or(sound), dims)
        } else {
            (None, None)
        };
        (plan, failed, duration, dims)
    });
    Some(match read {
        Some((plan, failed, duration, dims)) => WriterJob::Meta { id, plan, whole, done, ran, failed, duration, dims, later },
        // a file that crashes a reader counts as read, or it would be tried forever
        None => {
            let failed = ran.iter().map(|r| (r.clone(), "Reading this file failed unexpectedly".to_string())).collect();
            WriterJob::Meta { id, plan: readers::Plan::default(), whole, done, ran, failed, duration: None, dims: None, later }
        }
    })
}
