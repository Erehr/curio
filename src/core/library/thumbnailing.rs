//! The thumbnail threads: their queue, visible items first, and what each makes.

use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use crossbeam_channel::Sender;
use parking_lot::{Condvar, Mutex};
use crate::core::db::{Db, DbResult};
use crate::core::system::logging;
use crate::core::model::*;
use crate::core::thumbs;
use crate::core::library::writer::guard;
use crate::core::library::WriterJob;
use crate::core::library::changes::Changed;
use crate::core::library::Ctx;
use crate::core::library::ScanJob;

pub(super) struct ThumbQueue {
    pub(super) inner: Mutex<TQ>,
    pub(super) cv: Condvar,
}

/// The library's job thread and thumbnail queue, for work that finishes on another thread.
#[derive(Clone)]
pub struct Poster {
    pub(super) scanner: Sender<ScanJob>,
    pub(super) thumbq: Arc<ThumbQueue>,
}

impl Poster {
    pub fn job(&self, f: impl FnOnce(&mut Db, &Ctx) -> DbResult<Changed> + Send + 'static) {
        let _ = self.scanner.send(ScanJob::Run(Box::new(f)));
    }

    pub fn kick_thumbs(&self) {
        self.thumbq.inner.lock().exhausted_at = None;
        self.thumbq.cv.notify_all();
    }
}

#[derive(Default)]
pub(super) struct TQ {
    pub(super) urgent: VecDeque<AssetId>,
    pub(super) background: VecDeque<AssetId>,
    pub(super) queued: HashSet<AssetId>,
    pub(super) in_flight: HashSet<AssetId>,
    feeding: bool,
    pub(super) exhausted_at: Option<Instant>,
}

pub(super) fn thumb_loop(db: Db, q: Arc<ThumbQueue>, writer: Sender<WriterJob>, ctx: Arc<Ctx>, shutdown: Arc<AtomicBool>) {
    loop {
        if shutdown.load(Ordering::Relaxed) {
            return;
        }
        let next = {
            let mut g = q.inner.lock();
            loop {
                if shutdown.load(Ordering::Relaxed) {
                    return;
                }
                if let Some(id) = g.urgent.pop_front().or_else(|| g.background.pop_front()) {
                    g.queued.remove(&id);
                    g.in_flight.insert(id);
                    break Some(id);
                }
                let recently_empty = g.exhausted_at.map(|t| t.elapsed() < Duration::from_secs(5)).unwrap_or(false) && !ctx.thumbs_wanted.swap(false, Ordering::Relaxed);
                if !g.feeding && !recently_empty {
                    g.feeding = true;
                    let exclude: HashSet<AssetId> = g.in_flight.union(&g.queued).copied().collect();
                    drop(g);
                    let ids = db.pending_thumbs(256, &exclude, &ctx.scanning()).unwrap_or_default();
                    g = q.inner.lock();
                    g.feeding = false;
                    if ids.is_empty() {
                        g.exhausted_at = Some(Instant::now());
                    }
                    for id in ids {
                        if g.queued.insert(id) {
                            g.background.push_back(id);
                        }
                    }
                    q.cv.notify_all();
                    continue;
                }
                q.cv.wait_for(&mut g, Duration::from_secs(2));
            }
        };
        let Some(id) = next else { continue };
        // one the grid asked for first waits, as the rest of its location does, for the location's scan to end
        let under_scan = || {
            let roots = ctx.scanning();
            !roots.is_empty() && db.conn.query_row("SELECT root_id FROM assets WHERE id = ?", [id], |r| r.get::<_, i64>(0)).is_ok_and(|r| roots.contains(&r))
        };
        if under_scan() {
            q.inner.lock().in_flight.remove(&id);
            continue;
        }
        let job = guard("a thumbnail", || make_thumb(&db, &ctx, id)).unwrap_or(Some(WriterJob::ThumbDone { id, ok: false, dims: None, aspect: None, palette: vec![], look: None, duration: None, title: None }));
        q.inner.lock().in_flight.remove(&id);
        if let Some(j) = job {
            let _ = writer.send(j);
            ctx.thumbs_done.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn make_thumb(db: &Db, ctx: &Ctx, id: AssetId) -> Option<WriterJob> {
    let row: Option<(String, String, String, String, String, i64, i64, i64, String)> = db
        .conn
        .query_row(
            "SELECT r.path, a.rel_path, a.ext, a.kind, uid_text(a.uid), a.thumb_ver, a.thumb_state, a.status, a.source_url FROM assets a JOIN roots r ON r.id = a.root_id WHERE a.id = ?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
        )
        .ok();
    let (root, rel, ext, kind, uid, ver, state, status, source_url) = row?;
    let dest = thumbs::thumb_path(&ctx.paths.thumbs, &uid, ver);
    if state == 1 && dest.exists() {
        return None;
    }
    if status != 0 {
        return None;
    }
    // a cover grouped into the item draws it while its file is there
    let cover: Option<(String, String, String, String)> = db
        .conn
        .query_row("SELECT r.path, m.rel_path, m.ext, m.kind FROM assets m JOIN roots r ON r.id = m.root_id WHERE m.part_of = ? AND m.part_cover = 1 AND m.status = 0 ORDER BY m.id LIMIT 1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .ok();
    // a plugin's page draws the item's own file, so it is asked only for an item no cover draws
    let paged = cover.is_none() && ctx.contrib.read().page_thumb(&ext).is_some();
    let (root, rel, ext, kind) = cover.unwrap_or((root, rel, ext, kind));
    let src = crate::core::system::filesystem::db_to_abs(&root, &rel);
    let handlers = ctx.contrib.read().thumb_handlers.clone();
    let _ = std::fs::create_dir_all(&ctx.paths.tmp);
    let kind = AssetKind::parse(&kind);
    if ctx.contrib.read().disabled_exts.contains(&ext) {
        return Some(WriterJob::ThumbDone { id, ok: false, dims: None, aspect: None, palette: vec![], look: None, duration: None, title: None });
    }
    if paged {
        return Some(WriterJob::ThumbPage { id });
    }
    let link = if matches!(ext.as_str(), "url" | "webloc" | "website") && !source_url.is_empty() { Some(source_url.as_str()) } else { None };
    match thumbs::generate_with(&src, &ext, kind, &dest, &handlers, &ctx.paths.tmp, link) {
        Ok(r) => {
            thumbs::remove_older(&ctx.paths.thumbs, &uid, ver);
            Some(WriterJob::ThumbDone { id, ok: r.has_thumb, dims: r.dims, aspect: r.aspect, palette: r.palette, look: r.look, duration: r.duration, title: r.title })
        }
        Err(e) if e == crate::core::media::NEEDS_FFMPEG => Some(WriterJob::ThumbWaiting { id }),
        Err(e) => {
            logging::warn(&format!("thumbnail {}: {e}", src.display()));
            Some(WriterJob::ThumbDone { id, ok: false, dims: None, aspect: None, palette: vec![], look: None, duration: None, title: None })
        }
    }
}
