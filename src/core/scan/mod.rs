//! Incremental reconciliation of a root folder with the index.
//!
//! * Unchanged files (same size + mtime) cost one `stat` — no hashing, no writes.
//! * Changed files keep their identity; derived data (thumbnail, hash) is invalidated.
//! * New files are matched against vanished ones by filesystem identity, then by quick hash
//!   (size + first/last 64 KiB), so moves and renames keep tags and metadata.
//! * Vanished files become "missing", never deleted. An unreachable root marks its assets
//!   "offline" instead.

pub mod watcher;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::consts::SCAN_BATCH;
use crate::core::db::{Db, DbResult, assets::ScannedFile};
use crate::core::metadata::extract;
use crate::core::system::filesystem;
use crate::core::model::*;

#[derive(Clone, Debug, Default)]
pub struct ScanStats {
    pub seen: usize,
    pub added: usize,
    pub updated: usize,
    pub moved: usize,
    pub missing: usize,
    pub restored: usize,
    pub errors: usize,
    pub offline: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssetEvent {
    Discovered(AssetId),
    Modified(AssetId),
    Moved(AssetId),
    Missing(AssetId),
}

impl AssetEvent {
    pub fn id(&self) -> AssetId {
        match self {
            AssetEvent::Discovered(i) | AssetEvent::Modified(i) | AssetEvent::Moved(i) | AssetEvent::Missing(i) => *i,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScanOptions {
    pub index_all_files: bool,
    pub extra_exts: HashSet<String>,
    /// rules grouping files into one item, in order: the library's own, then the plugins'
    pub companions: Vec<crate::core::grouping::companions::Rule>,
    /// the extensions their members have: such a file comes in as a member, never as an item of its own
    pub companion_exts: HashSet<String>,
}

impl ScanOptions {
    pub fn with_companions(mut self, companions: Vec<crate::core::grouping::companions::Rule>) -> Self {
        self.companion_exts = companions.iter().flat_map(|r| r.templates.iter().filter_map(|t| t.ext())).collect();
        self.companions = companions;
        self
    }
}

/// What a root's scan takes in: every file, except in a linked location's folders, where files of no known kind come in only with Settings' "every file type".
fn for_root(root: &Root, opts: &ScanOptions) -> ScanOptions {
    ScanOptions { index_all_files: opts.index_all_files || root.kind != RootKind::Linked || root.explicit_only, ..opts.clone() }
}

pub fn indexable(ext: &str, opts: &ScanOptions) -> bool {
    if ext.is_empty() {
        return opts.index_all_files;
    }
    // a switched-off kind's files still come in, without thumbnails or what its plugin reads; reading them again later fills those in
    opts.index_all_files || crate::core::formats::family_of(ext).is_some() || opts.extra_exts.contains(ext)
}

/// Events about files grouped into an item are about that item: a member never reaches what lists items.
fn items_only(db: &Db, events: Vec<AssetEvent>) -> DbResult<Vec<AssetEvent>> {
    let mut st = db.conn.prepare_cached("SELECT part_of FROM assets WHERE id = ?").map_err(|e| e.to_string())?;
    let mut out: Vec<AssetEvent> = Vec::with_capacity(events.len());
    let mut items: Vec<AssetId> = vec![];
    for ev in events {
        // a file found now is a member only through pairing, which already took its event out
        let part_of = match ev {
            AssetEvent::Discovered(_) => None,
            _ => st.query_row([ev.id()], |r| r.get::<_, Option<AssetId>>(0)).unwrap_or(None),
        };
        match part_of {
            Some(item) => items.push(item),
            None => out.push(ev),
        }
    }
    let listed: HashSet<AssetId> = out.iter().map(|e| e.id()).collect();
    items.sort();
    items.dedup();
    out.extend(items.into_iter().filter(|i| !listed.contains(i)).map(AssetEvent::Modified));
    Ok(out)
}

/// Looked at by a scan: files that come in as items, and those that may be members of one.
fn wanted(ext: &str, opts: &ScanOptions) -> bool {
    indexable(ext, opts) || opts.companion_exts.contains(ext)
}

/// Pair what a scan or an add brought in (`new`, by path in the location), and keep the files that became members out of what it answers: their events go, their items' say they changed. Answers how many discovered files became members.
fn pair_new(db: &mut Db, root: &Root, new: HashSet<String>, opts: &ScanOptions, events: &mut Vec<AssetEvent>) -> DbResult<usize> {
    if opts.companions.is_empty() || new.is_empty() {
        return Ok(0);
    }
    let mut dirs: Vec<String> = new.iter().map(|r| crate::core::grouping::companions::dir_of(r)).collect();
    dirs.sort();
    dirs.dedup();
    let pairs = crate::core::grouping::companions::after(db, root, &dirs, &opts.companions, Some(&new))?;
    let members: HashSet<AssetId> = pairs.iter().map(|p| p.1).collect();
    let before = events.len();
    events.retain(|e| !(matches!(e, AssetEvent::Discovered(_)) && members.contains(&e.id())));
    let dropped = before - events.len();
    events.retain(|e| !members.contains(&e.id()));
    let listed: HashSet<AssetId> = events.iter().map(|e| e.id()).collect();
    let mut items: Vec<AssetId> = pairs.iter().map(|p| p.0).filter(|i| !listed.contains(i)).collect();
    items.sort();
    items.dedup();
    events.extend(items.into_iter().map(AssetEvent::Modified));
    Ok(dropped)
}

struct Known {
    id: AssetId,
    size: i64,
    mtime: Option<i64>,
    file_key: Option<String>,
    quick_hash: Option<String>,
    status: i64,
}

/// File systems reuse identities, so a new file can carry a deleted file's key.
fn same_file(k: &Known, f: &ScannedFile) -> bool {
    k.size == f.size && (k.quick_hash.is_none() || f.quick_hash.is_none() || k.quick_hash == f.quick_hash)
}

fn known_files(db: &Db, root_id: i64) -> DbResult<HashMap<String, Known>> {
    let mut st = db
        .conn
        .prepare("SELECT rel_path, id, size, mtime, file_key, quick_hash, status FROM assets WHERE root_id = ?")
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map([root_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                Known { id: r.get(1)?, size: r.get(2)?, mtime: r.get(3)?, file_key: r.get(4)?, quick_hash: r.get(5)?, status: r.get(6)? },
            ))
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub(crate) fn scanned(root: &Path, path: &Path, st: &filesystem::FileStat) -> ScannedFile {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let ext = filesystem::ext_of(path);
    let dims = if kind_for_ext(&ext) == AssetKind::Image { extract::probe_dims(path, &ext) } else { None };
    let animated = matches!(ext.as_str(), "gif" | "webp" | "png" | "apng").then(|| extract::is_animated(path, &ext));
    ScannedFile {
        rel_path: filesystem::rel_to_db(rel),
        name: filesystem::name_of(path),
        ext,
        size: st.size as i64,
        mtime: st.mtime,
        ctime: st.ctime,
        file_key: st.file_key.clone(),
        quick_hash: filesystem::quick_hash(path, st.size).ok(),
        width: dims.map(|d| d.0),
        height: dims.map(|d| d.1),
        duration: None,
        animated,
    }
}

/// Full reconcile of one root.
pub fn scan_root(db: &mut Db, root: &Root, opts: &ScanOptions, progress: &dyn Fn(usize, &str), cancel: &AtomicBool) -> DbResult<(ScanStats, Vec<AssetEvent>)> {
    scan_root_with(db, root, opts, progress, &|_| {}, cancel)
}

/// `scan_root`, telling `found` of the items a first scan of the root makes while it walks, a batch at a time.
pub fn scan_root_with(db: &mut Db, root: &Root, opts: &ScanOptions, progress: &dyn Fn(usize, &str), found: &dyn Fn(&[AssetId]), cancel: &AtomicBool) -> DbResult<(ScanStats, Vec<AssetEvent>)> {
    if root.kind == RootKind::Unknown {
        return Ok((ScanStats { offline: true, ..Default::default() }, Vec::new()));
    }
    let opts = &for_root(root, opts);
    let mut stats = ScanStats::default();
    let mut events = Vec::new();
    let root_path = PathBuf::from(&root.path);
    if !filesystem::root_online(&root_path) {
        db.set_root_online(root.id, false)?;
        stats.offline = true;
        return Ok((stats, events));
    }
    db.conn.execute("UPDATE roots SET online = 1 WHERE id = ?", [root.id]).map_err(|e| e.to_string())?;
    let known = known_files(db, root.id)?;
    let forgotten = db.forgotten(root.id)?;
    let mut seen: HashSet<AssetId> = HashSet::with_capacity(known.len());
    let mut new_files: Vec<ScannedFile> = Vec::new();
    let mut changed: Vec<(AssetId, ScannedFile)> = Vec::new();
    let mut back_online: Vec<AssetId> = Vec::new();
    let mut unhashed: Vec<(AssetId, String)> = Vec::new();
    // folders the walk couldn't read: their known files aren't missing, just unseen
    let mut unreadable: Vec<String> = Vec::new();
    let mut found_anything = root.explicit_only;
    // a root that knows no files has none that moved within it, so its new files go in as the walk finds them rather than after it
    let stream = known.is_empty() && !root.explicit_only;
    let mut streamed: Vec<String> = Vec::new();
    let mut used: HashSet<AssetId> = HashSet::new();

    let mut dir_names = crate::core::grouping::companions::DirNames::default();
    // a file's identity is read only for files new or changed, which need it to be matched to moved ones: opening every file for it made an unchanged rescan a hundred times slower than its walk
    let mut visit = |path: &Path, st: std::io::Result<filesystem::FileStat>, stats: &mut ScanStats, new_files: &mut Vec<ScannedFile>| {
        let rel = filesystem::rel_to_db(path.strip_prefix(&root_path).unwrap_or(path));
        let Ok(mut st) = st else {
            stats.errors += 1;
            // a file the walk lists but can't read is there, not missing
            if let Some(k) = known.get(&rel) {
                seen.insert(k.id);
            }
            return;
        };
        if st.is_dir {
            return;
        }
        stats.seen += 1;
        if stats.seen % 500 == 0 {
            progress(stats.seen, &rel);
        }
        match known.get(&rel) {
            Some(k) => {
                seen.insert(k.id);
                if k.size == st.size as i64 && k.mtime == st.mtime {
                    if k.status != 0 {
                        back_online.push(k.id);
                    }
                    // a quick hash cleared by a migration, or one the file couldn't give before: moves are matched by it
                    if k.quick_hash.is_none() {
                        if let Ok(h) = filesystem::quick_hash(path, st.size) {
                            unhashed.push((k.id, h));
                        }
                    }
                } else {
                    st.file_key = st.file_key.take().or_else(|| filesystem::file_key(path));
                    changed.push((k.id, scanned(&root_path, path, &st)));
                }
            }
            None => {
                // a file only rules take in is looked at only beside a file that would take it
                let mut wanted_here = || indexable(&filesystem::ext_of(path), opts) || crate::core::grouping::companions::may_pair(path, &opts.companions, &mut dir_names);
                if !root.explicit_only && !is_forgotten(forgotten.get(&rel).copied(), &st) && wanted_here() {
                    st.file_key = st.file_key.take().or_else(|| filesystem::file_key(path));
                    new_files.push(scanned(&root_path, path, &st));
                }
            }
        }
    };

    if root.explicit_only {
        for rel in known.keys() {
            if cancel.load(Ordering::Relaxed) {
                return Ok((stats, events));
            }
            let p = filesystem::db_to_abs(&root.path, rel);
            match std::fs::metadata(&p) {
                Ok(md) => visit(&p, Ok(filesystem::listed(&md)), &mut stats, &mut new_files),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => visit(&p, Err(e), &mut stats, &mut new_files),
            }
        }
    } else {
        let walker = walkdir::WalkDir::new(&root_path).follow_links(false).into_iter().filter_entry(|e| {
            e.depth() == 0 || !filesystem::is_ignored_name(&e.file_name().to_string_lossy())
        });
        for entry in walker {
            if cancel.load(Ordering::Relaxed) {
                return Ok((stats, events));
            }
            let entry = match entry {
                Ok(e) => e,
                Err(err) => {
                    stats.errors += 1;
                    if let Some(p) = err.path() {
                        unreadable.push(filesystem::rel_to_db(p.strip_prefix(&root_path).unwrap_or(p)));
                    }
                    continue;
                }
            };
            found_anything |= entry.depth() > 0;
            if !entry.file_type().is_file() {
                continue;
            }
            let ext = filesystem::ext_of(entry.path());
            if !wanted(&ext, opts) {
                continue;
            }
            visit(entry.path(), entry.metadata().map(|m| filesystem::listed(&m)).map_err(std::io::Error::from), &mut stats, &mut new_files);
            if stream && new_files.len() >= SCAN_BATCH {
                let chunk: Vec<ScannedFile> = new_files.drain(..).collect();
                let got = insert_new(db, root, &chunk, opts, &HashMap::new(), &mut HashMap::new(), &mut used, &mut stats)?;
                found(&got.iter().map(|e| e.id()).collect::<Vec<_>>());
                streamed.extend(chunk.into_iter().map(|f| f.rel_path));
                events.extend(got);
            }
        }
    }

    // vanished entries of this root, candidates for move matching
    // the walk skips what isn't indexable right now, which says nothing about whether those files exist
    let looked_for = |rel: &str| root.explicit_only || wanted(&filesystem::ext_of(Path::new(rel)), opts);
    let under_unreadable = |rel: &str| unreadable.iter().any(|d| d.is_empty() || rel == d || rel.starts_with(&format!("{d}/")));
    let mut vanished: HashMap<AssetId, &Known> =
        known.iter().filter(|(rel, k)| !seen.contains(&k.id) && k.status != 2 && looked_for(rel) && !under_unreadable(rel)).map(|(_, k)| (k.id, k)).collect();
    // an empty folder where files were known is most likely an unmounted drive or share, not a deletion
    if !found_anything && !vanished.is_empty() {
        db.set_root_online(root.id, false)?;
        stats.offline = true;
        return Ok((stats, events));
    }
    let mut by_key: HashMap<&str, &Known> = HashMap::new();
    // identical files share a hash, so each keeps its own identity only while every one of them is taken in turn
    let mut by_hash: HashMap<(String, i64), Vec<AssetId>> = HashMap::new();
    for (id, k) in &vanished {
        if let Some(fk) = &k.file_key {
            by_key.insert(fk, *k);
        }
        if let Some(h) = &k.quick_hash {
            by_hash.entry((h.clone(), k.size)).or_default().push(*id);
        }
    }
    // a file back at its own path is itself, never a moved file's new place: a copy of it must not take its row
    used.extend(back_online.iter().copied());
    // also previously-missing assets of this root can come back under a new name
    for k in known.values().filter(|k| k.status == 2 && !seen.contains(&k.id)) {
        if let Some(fk) = &k.file_key {
            by_key.entry(fk).or_insert(k);
        }
        if let Some(h) = &k.quick_hash {
            by_hash.entry((h.clone(), k.size)).or_default().push(k.id);
        }
    }

    let total_batches = new_files.len() + changed.len();
    let mut done = 0usize;
    for chunk in changed.chunks(SCAN_BATCH) {
        db.tx(|db| {
            for (id, f) in chunk {
                db.update_scanned(*id, f)?;
            }
            Ok(())
        })?;
        for (id, _) in chunk {
            events.push(AssetEvent::Modified(*id));
        }
        stats.updated += chunk.len();
        done += chunk.len();
        progress(stats.seen, &format!("updating {done}/{total_batches}"));
    }
    for chunk in new_files.chunks(SCAN_BATCH) {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let got = insert_new(db, root, chunk, opts, &by_key, &mut by_hash, &mut used, &mut stats)?;
        if stream {
            found(&got.iter().map(|e| e.id()).collect::<Vec<_>>());
        }
        events.extend(got);
        done += chunk.len();
        progress(stats.seen, &format!("indexing {done}/{total_batches}"));
    }
    for id in &used {
        vanished.remove(id);
    }
    let missing: Vec<AssetId> = vanished.keys().copied().collect();
    // in batches, so a subtree gone or back never holds the writer for long
    for chunk in missing.chunks(SCAN_BATCH) {
        db.tx(|db| db.set_status(chunk, Availability::Missing))?;
        events.extend(chunk.iter().map(|id| AssetEvent::Missing(*id)));
        stats.missing += chunk.len();
    }
    for chunk in back_online.chunks(SCAN_BATCH) {
        db.tx(|db| db.set_status(chunk, Availability::Ok))?;
        stats.restored += chunk.len();
    }
    for chunk in unhashed.chunks(SCAN_BATCH) {
        db.tx(|db| chunk.iter().try_for_each(|(id, h)| db.fill_quick_hash(*id, h)))?;
    }
    if !cancel.load(Ordering::Relaxed) {
        let new = streamed.into_iter().chain(new_files.iter().map(|f| f.rel_path.clone())).collect();
        stats.added -= pair_new(db, root, new, opts, &mut events)?;
    }
    db.touch_root_scan(root.id)?;
    Ok((stats, items_only(db, events)?))
}

/// New files in, in one transaction: each takes the place of an item that moved (by its file key, then its quick hash), or becomes a new item.
#[allow(clippy::too_many_arguments)]
fn insert_new(db: &mut Db, root: &Root, chunk: &[ScannedFile], opts: &ScanOptions, by_key: &HashMap<&str, &Known>, by_hash: &mut HashMap<(String, i64), Vec<AssetId>>, used: &mut HashSet<AssetId>, stats: &mut ScanStats) -> DbResult<Vec<AssetEvent>> {
    let mut events = Vec::new();
    db.tx(|db| {
        for f in chunk {
            let mut target = f.file_key.as_deref().and_then(|k| by_key.get(k)).filter(|k| same_file(k, f)).map(|k| k.id).filter(|id| !used.contains(id));
            if target.is_none() {
                // claimed ids leave the bucket as they are met, so identical files moved together cost one visit each
                if let Some(ids) = f.quick_hash.as_ref().and_then(|h| by_hash.get_mut(&(h.clone(), f.size))) {
                    while ids.last().is_some_and(|id| used.contains(id)) {
                        ids.pop();
                    }
                    target = ids.last().copied();
                }
            }
            if target.is_none() {
                target = db.find_move_candidates(f.file_key.as_deref(), f.quick_hash.as_deref(), f.size).into_iter().find(|id| !used.contains(id));
            }
            match target {
                Some(id) => {
                    used.insert(id);
                    db.move_asset(id, root.id, &f.rel_path, &f.name)?;
                    db.update_scanned(id, f)?;
                    events.push(AssetEvent::Moved(id));
                }
                // a file only rules take in waits for pairing, which makes its row
                None if !indexable(&f.ext, opts) => {}
                None => events.push(AssetEvent::Discovered(db.insert_scanned(root.id, f)?)),
            }
        }
        Ok(())
    })?;
    stats.moved += events.iter().filter(|e| matches!(e, AssetEvent::Moved(_))).count();
    stats.added += events.iter().filter(|e| matches!(e, AssetEvent::Discovered(_))).count();
    Ok(events)
}

/// Purged from the library and unchanged since. A changed file counts as new.
pub(crate) fn is_forgotten(f: Option<(i64, Option<i64>)>, st: &filesystem::FileStat) -> bool {
    f.is_some_and(|(size, mtime)| size == st.size as i64 && mtime == st.mtime)
}

/// Reconcile only the given paths (from filesystem notifications).
pub fn reconcile_paths(db: &mut Db, root: &Root, paths: &[PathBuf], opts: &ScanOptions) -> DbResult<(ScanStats, Vec<AssetEvent>)> {
    if root.kind == RootKind::Unknown {
        return Ok((ScanStats::default(), Vec::new()));
    }
    let opts = &for_root(root, opts);
    let mut stats = ScanStats::default();
    let mut events = Vec::new();
    let root_path = PathBuf::from(&root.path);
    let mut gone = Vec::new();
    // with what the listing says of each, so unchanged files are never opened
    let mut present: Vec<(PathBuf, Option<filesystem::FileStat>)> = Vec::new();
    for p in paths {
        match std::fs::metadata(p) {
            Ok(md) if md.is_dir() => {
                for e in walkdir::WalkDir::new(p).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {
                    let st = e.metadata().ok().map(|m| filesystem::listed(&m));
                    present.push((e.into_path(), st));
                }
            }
            Ok(md) => present.push((p.clone(), Some(filesystem::listed(&md)))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => gone.push(p.clone()),
            Err(_) => stats.errors += 1,
        }
    }
    db.tx(|db| {
        // removals first so a rename (remove + create) in the same batch is matched as a move
        for p in &gone {
            let rel = filesystem::rel_to_db(p.strip_prefix(&root_path).unwrap_or(p));
            let mut st = db
                .conn
                .prepare_cached("SELECT id FROM assets WHERE root_id = ? AND (rel_path = ? OR rel_path LIKE ? ESCAPE '\\') AND status <> 2")
                .map_err(|e| e.to_string())?;
            let like = format!("{}/%", crate::core::query::compile::escape_like(&rel));
            let ids: Vec<AssetId> = st
                .query_map(rusqlite::params![root.id, rel, like], |r| r.get(0))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            drop(st);
            db.set_status(&ids, Availability::Missing)?;
            for id in ids {
                events.push(AssetEvent::Missing(id));
                stats.missing += 1;
            }
        }
        Ok(())
    })?;
    // stat, hash and probe before taking the write lock; the transactions only write
    let lookup = |db: &Db, rel: &str| -> Option<(AssetId, i64, Option<i64>, i64)> {
        db.conn
            .prepare_cached("SELECT id, size, mtime, status FROM assets WHERE root_id = ? AND rel_path = ?")
            .and_then(|mut st| st.query_row(rusqlite::params![root.id, rel], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))))
            .ok()
    };
    enum Step {
        Update(AssetId, ScannedFile),
        Restore(AssetId),
        Add(ScannedFile),
    }
    let mut new: HashSet<String> = HashSet::new();
    let mut dir_names = crate::core::grouping::companions::DirNames::default();
    for chunk in present.chunks(SCAN_BATCH) {
        let mut steps = Vec::with_capacity(chunk.len());
        for (p, listed) in chunk {
            let name = filesystem::name_of(p);
            if filesystem::is_ignored_name(&name) || !wanted(&filesystem::ext_of(p), opts) {
                continue;
            }
            let Some(mut st) = listed.clone().or_else(|| std::fs::metadata(p).ok().map(|m| filesystem::listed(&m))) else { continue };
            let rel = filesystem::rel_to_db(p.strip_prefix(&root_path).unwrap_or(p));
            stats.seen += 1;
            // the identity, which opens the file, only for a file to be written: moves are matched by it
            let with_key = |st: &mut filesystem::FileStat| st.file_key = filesystem::file_key(p);
            match lookup(db, &rel) {
                Some((id, size, mtime, _)) if size != st.size as i64 || mtime != st.mtime => {
                    with_key(&mut st);
                    steps.push((rel, Step::Update(id, scanned(&root_path, p, &st))))
                }
                Some((id, _, _, status)) if status != 0 => steps.push((rel, Step::Restore(id))),
                Some(_) => {}
                None if root.explicit_only || is_forgotten(db.forgotten_one(root.id, &rel), &st) => {}
                None if !indexable(&filesystem::ext_of(p), opts) && !crate::core::grouping::companions::may_pair(p, &opts.companions, &mut dir_names) => {}
                None => {
                    with_key(&mut st);
                    steps.push((rel, Step::Add(scanned(&root_path, p, &st))))
                }
            }
        }
        // files back at their own paths first, so a copy of one added in the same batch can't take its row as a move
        steps.sort_by_key(|(_, s)| !matches!(s, Step::Restore(_)));
        if steps.is_empty() {
            continue;
        }
        // a file written in steps may only now be whole enough to be checked
        new.extend(steps.iter().filter(|(_, s)| !matches!(s, Step::Restore(_))).map(|(rel, _)| rel.clone()));
        db.tx(|db| {
            for (rel, step) in steps {
                match step {
                    Step::Update(id, f) => {
                        db.update_scanned(id, &f)?;
                        events.push(AssetEvent::Modified(id));
                        stats.updated += 1;
                    }
                    Step::Restore(id) => {
                        db.set_status(&[id], Availability::Ok)?;
                        stats.restored += 1;
                    }
                    // added by something else in the meantime: the next event about it sorts it out
                    Step::Add(_) if lookup(db, &rel).is_some() => {}
                    Step::Add(f) => match db.find_move_candidate(f.file_key.as_deref(), f.quick_hash.as_deref(), f.size) {
                        Some(id) => {
                            db.move_asset(id, root.id, &f.rel_path, &f.name)?;
                            db.update_scanned(id, &f)?;
                            events.push(AssetEvent::Moved(id));
                            stats.moved += 1;
                        }
                        None if !indexable(&f.ext, opts) => {}
                        None => {
                            let id = db.insert_scanned(root.id, &f)?;
                            events.push(AssetEvent::Discovered(id));
                            stats.added += 1;
                        }
                    },
                }
            }
            Ok(())
        })?;
    }
    stats.added -= pair_new(db, root, new, opts, &mut events)?;
    Ok((stats, items_only(db, events)?))
}

/// Add individual files (outside any folder root) as linked assets: each parent folder
/// becomes an "explicit" root that only tracks the files added to it.
/// The files the rules give one of them come in as its members, and are left out of the answer.
pub fn add_loose_files(db: &mut Db, files: &[PathBuf], rules: &[crate::core::grouping::companions::Rule]) -> DbResult<Vec<AssetId>> {
    let mut ids = Vec::new();
    let roots = db.roots()?;
    let mut given: HashMap<i64, HashSet<String>> = HashMap::new();
    for f in files {
        let f = filesystem::canonical(f);
        let Ok(st) = filesystem::stat(&f) else { continue };
        if st.is_dir {
            continue;
        }
        // inside an existing root? just reconcile that path
        if let Some(r) = roots.iter().find(|r| f.starts_with(&r.path)) {
            let rel = filesystem::rel_to_db(f.strip_prefix(&r.path).unwrap_or(&f));
            given.entry(r.id).or_default().insert(rel.clone());
            if let Some(id) = db.conn.query_row("SELECT id FROM assets WHERE root_id = ? AND rel_path = ?", rusqlite::params![r.id, rel], |row| row.get(0)).ok() {
                ids.push(id);
                continue;
            }
            let sf = scanned(Path::new(&r.path), &f, &st);
            ids.push(db.insert_scanned(r.id, &sf)?);
            continue;
        }
        let parent = f.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let root_id = db.add_root(&parent.to_string_lossy(), RootKind::Linked, true)?;
        let sf = scanned(&parent, &f, &st);
        given.entry(root_id).or_default().insert(sf.rel_path.clone());
        ids.push(db.insert_scanned(root_id, &sf)?);
    }
    if !rules.is_empty() {
        let mut members = HashSet::new();
        for (root_id, rels) in given {
            let Some(root) = db.root(root_id) else { continue };
            let dirs: Vec<String> = rels.iter().map(|r| crate::core::grouping::companions::dir_of(r)).collect();
            members.extend(crate::core::grouping::companions::after(db, &root, &dirs, rules, Some(&rels))?.into_iter().map(|p| p.1));
        }
        ids.retain(|id| !members.contains(id));
    }
    Ok(ids)
}
