//! Bringing files into a library from outside: auto-import of watched folders, downloads from addresses, and the notices that tell of them.

use crate::core::import::links::check_web_url;
use crate::core::export::asset_json;
use crate::core::library::paths::LibraryPaths;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use crate::core::db::{Db, DbResult, assets::ScannedFile};
use crate::core::system::filesystem;
use crate::core::model::*;
use crate::core::store::{self, OnClash, SuffixStyle};

pub mod auto_import;
pub mod clipboard;
pub mod downloads;
pub mod eagle;
pub mod links;
pub mod notices;

pub fn sanitize_name(name: &str) -> String {
    if name.trim().is_empty() {
        return "file".into();
    }
    crate::core::store::clean_name(name, false)
}

pub fn managed_root(db: &Db, paths: &LibraryPaths) -> DbResult<i64> {
    db.add_root(&paths.managed.to_string_lossy(), RootKind::Managed, true)
}

/// Record a file already in the store as a new item placed in `folder` (`None` = store root), and its membership of that folder.
pub fn insert_managed_file(db: &Db, paths: &LibraryPaths, root_id: i64, target: &Path, source_url: Option<&str>, folder: Option<i64>) -> DbResult<AssetId> {
    let st = filesystem::stat(target).map_err(|e| e.to_string())?;
    let rel = filesystem::rel_to_db(target.strip_prefix(&paths.managed).unwrap_or(target));
    let ext = filesystem::ext_of(target);
    let dims = crate::core::metadata::extract::probe_dims(target, &ext);
    let animated = matches!(ext.as_str(), "gif" | "webp" | "png" | "apng").then(|| crate::core::metadata::extract::is_animated(target, &ext));
    let display = filesystem::name_of(target);
    let f = ScannedFile {
        rel_path: rel,
        name: display,
        ext,
        size: st.size as i64,
        mtime: st.mtime,
        ctime: st.ctime,
        file_key: st.file_key,
        quick_hash: filesystem::quick_hash(target, st.size).ok(),
        width: dims.map(|d| d.0),
        height: dims.map(|d| d.1),
        duration: None,
        animated,
    };
    // a savepoint nests inside a caller's transaction and also works without one
    db.conn.execute_batch("SAVEPOINT managed_file").map_err(|e| e.to_string())?;
    let rel = f.rel_path.clone();
    let res = db
        .insert_scanned(root_id, &f)
        .and_then(|id| db.add_placement(id, folder, &rel, true).map(|_| id))
        .and_then(|id| match folder {
            Some(fid) => db.add_to_folder(&[id], fid).map(|_| id),
            None => Ok(id),
        })
        .and_then(|id| match source_url {
        Some(u) => db.set_source_url(&[id], u).map(|_| id),
        None => Ok(id),
    });
    if res.is_err() {
        let _ = db.conn.execute_batch("ROLLBACK TO managed_file");
    }
    let _ = db.conn.execute_batch("RELEASE managed_file");
    res
}

/// What happened to one incoming file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    New,
    Replaced,
    Skipped,
}

/// The content of an incoming file.
pub enum Incoming<'a> {
    File(&'a Path),
    /// A file whose data the library shares through a hard link, such as one in another program's library on the same drive; a copy where a link can't be made.
    Linked(&'a Path),
    /// A file the library takes rather than copies, such as a download in a temporary folder.
    Moved(&'a Path),
    Bytes(&'a [u8]),
}

/// Bring one file into the store straight into `folder`'s directory (`None` = store root) under `name`. A name taken by an item there follows `clash`. Returns the item it became, replaced or was skipped for.
pub fn import_one(db: &mut Db, paths: &LibraryPaths, src: Incoming, name: &str, folder: Option<i64>, clash: OnClash, style: SuffixStyle, source_url: Option<&str>) -> DbResult<(AssetId, Outcome)> {
    let root_id = managed_root(db, paths)?;
    let rel = match store::target_for(db, paths, folder, name, clash, style)? {
        store::Target::Existing(p) if clash == OnClash::Skip => return Ok((p.asset_id, Outcome::Skipped)),
        store::Target::Existing(p) => {
            match src {
                Incoming::File(f) | Incoming::Linked(f) => crate::core::store::journal::replace_with(db, p.asset_id, f)?,
                Incoming::Moved(f) => {
                    crate::core::store::journal::replace_with(db, p.asset_id, f)?;
                    let _ = std::fs::remove_file(f);
                }
                Incoming::Bytes(b) => {
                    let tmp = paths.tmp.join(format!("incoming-{}", fastrand::u64(..)));
                    std::fs::create_dir_all(&paths.tmp).map_err(|e| e.to_string())?;
                    std::fs::write(&tmp, b).map_err(|e| e.to_string())?;
                    let r = crate::core::store::journal::replace_with(db, p.asset_id, &tmp);
                    let _ = std::fs::remove_file(&tmp);
                    r?
                }
            }
            if let Err(e) = store::relink(db, paths, p.asset_id) {
                crate::core::system::logging::warn(&e);
            }
            return Ok((p.asset_id, Outcome::Replaced));
        }
        store::Target::New(rel) => rel,
    };
    let dest = store::abs(paths, &rel);
    let what = match &src {
        Incoming::File(f) | Incoming::Linked(f) | Incoming::Moved(f) => f.display().to_string(),
        Incoming::Bytes(_) => name.to_string(),
    };
    match src {
        Incoming::File(f) => store::copy_in(f, &dest),
        Incoming::Linked(f) => store::disk::link_or_copy(f, &dest, true).map(|_| ()),
        Incoming::Moved(f) => store::move_in(f, &dest),
        Incoming::Bytes(b) => store::write_in(b, &dest),
    }
    .map_err(|e| format!("copy {what}: {e}"))?;
    let id = db.tx(|db| {
        let id = insert_managed_file(db, paths, root_id, &dest, source_url, folder)?;
        // a numbered file keeps the item's own name; the number is only on disk
        let clean = store::clean_name(name, false);
        if filesystem::name_of(&dest) != clean {
            db.set_file_name(id, &clean)?;
        }
        Ok(id)
    })?;
    Ok((id, Outcome::New))
}

/// Ctrl-drag onto a library folder: each item becomes a new item there with its own copy of the bytes and the item's metadata as it is now; from then on the two are unrelated. Works for items indexed in place too, whose copy is a library item. Returns the new items.
pub fn copy_to_folder(db: &mut Db, paths: &LibraryPaths, ids: &[AssetId], folder: i64, style: SuffixStyle) -> DbResult<Vec<AssetId>> {
    let fields: HashMap<i64, FieldDef> = db.fields()?.into_iter().map(|f| (f.id, f)).collect();
    let copies: Vec<(PathBuf, String, serde_json::Value)> = db.assets(ids)?.iter().map(|a| (PathBuf::from(&a.abs_path), a.name.clone(), asset_json(db, a, &fields))).collect();
    let mut out = Vec::new();
    // the file and its row first, then the metadata; a crash between leaves an item without it, never a lost file
    for ((src, name, meta), from) in copies.iter().zip(ids) {
        let (id, _) = import_one(db, paths, Incoming::File(src), name, Some(folder), OnClash::Keep, style, None)?;
        db.tx(|db| apply_incoming(db, id, meta))?;
        copy_members(db, paths, *from, id, style)?;
        out.push(id);
    }
    Ok(out)
}

/// The files grouped into `from` copied into the store beside `to`, its copy, and grouped into it the same way. A file that can't be copied is left out with a line in the log.
pub fn copy_members(db: &mut Db, paths: &LibraryPaths, from: AssetId, to: AssetId, style: SuffixStyle) -> DbResult<()> {
    let name: String = db.conn.query_row("SELECT name FROM assets WHERE id = ?", [to], |r| r.get(0)).map_err(|e| e.to_string())?;
    for p in db.members_of(from)? {
        let Some(src) = db.abs_path(p.id) else { continue };
        let want = match &p.suffix {
            Some(s) => format!("{}{s}", crate::core::grouping::companions::stem(&name)),
            None => p.name.clone(),
        };
        match import_one(db, paths, Incoming::File(&src), &want, None, OnClash::Keep, style, None) {
            Ok((copy, _)) => {
                db.group(to, copy, &p.role, &p.rule, p.suffix.as_deref(), p.cover)?;
            }
            Err(e) => crate::core::system::logging::warn(&format!("{name}: copying the file grouped into it: {e}")),
        }
    }
    Ok(())
}

/// Copy files into the managed store, into `folder`; a folder among them brings its files without its structure. Skipped files are left out of the result.
pub fn import_managed(db: &mut Db, paths: &LibraryPaths, files: &[PathBuf], folder: Option<i64>, clash: OnClash, style: SuffixStyle) -> DbResult<Vec<AssetId>> {
    let mut ids = Vec::new();
    for src in files {
        if src.is_dir() {
            let inner: Vec<PathBuf> = walkdir::WalkDir::new(src)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file() && !filesystem::is_ignored_name(&e.file_name().to_string_lossy()))
                .map(|e| e.into_path())
                .collect();
            ids.extend(import_managed(db, paths, &inner, folder, clash, style)?);
            continue;
        }
        let (id, outcome) = import_one(db, paths, Incoming::File(src), &filesystem::name_of(src), folder, clash, style, None)?;
        if outcome != Outcome::Skipped {
            ids.push(id);
        }
    }
    Ok(ids)
}

/// Save bytes (clipboard image, downloaded URL) as a new managed item in `folder`; a taken name gets a number.
pub fn import_bytes(db: &mut Db, paths: &LibraryPaths, name: &str, bytes: &[u8], source_url: Option<&str>, folder: Option<i64>, style: SuffixStyle) -> DbResult<AssetId> {
    import_one(db, paths, Incoming::Bytes(bytes), name, folder, OnClash::Keep, style, source_url).map(|(id, _)| id)
}

/// Save a link as a bookmark item: an InternetShortcut `.url` file in managed storage with the
/// address as its source URL. Its thumbnail is generated from the
/// page (or the YouTube poster).
pub fn import_link(db: &mut Db, paths: &LibraryPaths, url: &str, title: Option<&str>, folder: Option<i64>, style: SuffixStyle) -> DbResult<AssetId> {
    check_web_url(url)?;
    let name = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| url.trim_start_matches("https://").trim_start_matches("http://").trim_start_matches("www.").chars().take(80).collect());
    let file = format!("{}.url", sanitize_name(&name).trim_end_matches('.'));
    let body = format!("[InternetShortcut]\r\nURL={url}\r\n");
    import_bytes(db, paths, &file, body.as_bytes(), Some(url), folder, style)
}

/// What another library knows about a file, in the shape of `asset_json`.
fn neighbour_metadata(src: &Path) -> Option<serde_json::Value> {
    let dir = src.parent()?;
    if !dir.file_name()?.to_string_lossy().to_lowercase().ends_with(".info") {
        return None;
    }
    let meta: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("metadata.json")).ok()?).ok()?;
    Some(serde_json::json!({
        "display_name": meta["name"], "tags": meta["tags"], "rating": meta["star"],
        "annotation": meta["annotation"], "source_url": meta["url"],
    }))
}

/// Apply tags, rating, notes, link and matching custom fields that came with a file.
pub fn apply_incoming(db: &Db, id: AssetId, meta: &serde_json::Value) -> DbResult<()> {
    if let Some(list) = meta["tags"].as_array() {
        let names: Vec<String> = list.iter().filter_map(|t| t.as_str().map(String::from)).collect();
        if !names.is_empty() {
            db.add_tags(&[id], &names)?;
        }
    }
    if let Some(r) = meta["rating"].as_i64().filter(|r| *r > 0) {
        db.set_rating(&[id], r)?;
    }
    if let Some(t) = meta["annotation"].as_str().filter(|t| !t.is_empty()) {
        db.set_annotation(&[id], t)?;
    }
    if let Some(t) = meta["source_url"].as_str().filter(|t| !t.is_empty()) {
        db.set_source_url(&[id], t)?;
    }
    if let Some(t) = meta["display_name"].as_str().filter(|t| !t.is_empty()) {
        db.set_display_name(id, t)?;
    }
    if let Some(fields) = meta["fields"].as_object() {
        let cat = db.field_catalog();
        let read = db.known_readers();
        let by_hand: Vec<&str> = meta["overrides"].as_array().into_iter().flatten().filter_map(|k| k.as_str()).collect();
        for (k, v) in fields {
            // a field this library doesn't index is left out, as is a value its field doesn't accept; the rest still comes, with the marks of values set by hand
            let Some(f) = cat.custom(k).filter(|f| f.indexed) else { continue };
            if db.set_field(&[id], f, &crate::core::plugins::run::json_to_raw(v)).is_ok() && by_hand.contains(&k.as_str()) && read.iter().any(|r| r.fills(&f.key)) {
                db.set_overridden(&[id], f.id, true)?;
            }
        }
    }
    Ok(())
}

/// Add files to a library: anything living inside another library's store is copied, the rest
/// follows `copy`. Metadata from the source library comes along, and `folder` receives them all.
#[allow(clippy::too_many_arguments)]
pub fn bring_in(
    db: &mut Db,
    paths: &LibraryPaths,
    files: &[PathBuf],
    copy: bool,
    folder: Option<i64>,
    style: SuffixStyle,
    clash: OnClash,
    transfer: &serde_json::Map<String, serde_json::Value>,
    scan: &crate::core::scan::ScanOptions,
    progress: &dyn Fn(usize, &str),
    cancel: &std::sync::atomic::AtomicBool,
) -> DbResult<Vec<AssetId>> {
    let step = |s: ImportStep| progress(s.done, s.name);
    bring_in_stepped(db, paths, files, copy, folder, style, clash, transfer, scan, &step, cancel)
}

/// Where an import has got to, reported after every file. `total` is 0 while a location's scan counts as it goes.
pub struct ImportStep<'a> {
    pub done: usize,
    pub total: usize,
    pub name: &'a str,
    /// what arrived since the last report
    pub new: &'a [AssetId],
    pub location: Option<Located>,
}

/// A location an import makes of a folder: told as soon as it exists, and again once its scan ends.
pub enum Located {
    Started(i64),
    Scanned(i64, crate::core::scan::ScanStats),
}

/// Files inside a dropped folder that would be taken in: all of a folder copied into the library, a linked one's as its location's scan takes them.
fn files_under(dir: &Path, scan: &crate::core::scan::ScanOptions, copied: bool) -> Vec<PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && !filesystem::is_ignored_name(&e.file_name().to_string_lossy()))
        .filter(|e| copied || crate::core::scan::indexable(&e.path().extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default(), scan))
        .map(|e| e.into_path())
        .collect()
}

/// The folder locations a dropped file may already be in; a location made for files added one by one isn't one.
fn folder_locations(db: &Db) -> DbResult<Vec<PathBuf>> {
    Ok(db.roots()?.into_iter().filter(|r| !r.explicit_only).map(|r| PathBuf::from(r.path)).collect())
}

/// Whether an add copies this path into the library: a folder is linked as a location unless `copy`, as only folders become locations; a file is linked only where a location already holds it.
fn copies_in(f: &Path, copy: bool, locations: &[PathBuf]) -> bool {
    if copy || crate::core::library::instance::in_library_store(f) {
        return true;
    }
    if f.is_dir() {
        return false;
    }
    let f = filesystem::canonical(f);
    !locations.iter().any(|l| f.starts_with(l))
}

/// How many of the files an import would copy take a name already used in the folder they go to (by an item there, or by an earlier file of the same import), and the first few such names. Nothing is created.
pub fn count_clashes(db: &Db, files: &[PathBuf], copy: bool, folder: Option<i64>, scan: &crate::core::scan::ScanOptions) -> DbResult<(usize, Vec<String>)> {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let (mut n, mut names) = (0, vec![]);
    let locations = folder_locations(db)?;
    let mut check = |target: Result<Option<i64>, String>, name: String| {
        // a folder not made yet has no items, only earlier files of this import
        let taken = match &target {
            Ok(f) => store::clash_path(db, *f, &name).is_some_and(|rel| db.member_placement_at(&rel, *f).is_some()),
            Err(_) => false,
        };
        let key = (format!("{target:?}"), store::clean_name(&name, false).to_lowercase());
        if !seen.insert(key) || taken {
            n += 1;
            if names.len() < 5 {
                names.push(name);
            }
        }
    };
    for f in files {
        if !copies_in(f, copy, &locations) {
            continue;
        }
        if !f.is_dir() {
            check(Ok(folder), filesystem::name_of(f));
            continue;
        }
        let top = f.parent().unwrap_or(f);
        for file in files_under(f, scan, true) {
            let chain: Vec<String> = file.parent().and_then(|p| p.strip_prefix(top).ok()).map(|p| p.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect()).unwrap_or_default();
            let mut at = Ok(folder);
            for part in &chain {
                at = match at {
                    Ok(parent) => db.child_folder(parent, part)?.map(Some).ok_or_else(|| chain.join("/")),
                    missing => missing,
                };
            }
            check(at, filesystem::name_of(&file));
        }
    }
    Ok((n, names))
}

/// Bring files in one at a time, so they show up (and land in `folder`) while the rest still copy. Copies go straight into their folder's directory; a name already taken there follows `clash`.
#[allow(clippy::too_many_arguments)]
pub fn bring_in_stepped(
    db: &mut Db,
    paths: &LibraryPaths,
    files: &[PathBuf],
    copy: bool,
    folder: Option<i64>,
    style: SuffixStyle,
    clash: OnClash,
    transfer: &serde_json::Map<String, serde_json::Value>,
    scan: &crate::core::scan::ScanOptions,
    step: &dyn Fn(ImportStep),
    cancel: &std::sync::atomic::AtomicBool,
) -> DbResult<Vec<AssetId>> {
    use std::sync::atomic::Ordering;
    // a folder being copied is opened up first, so the total is known from the start
    let mut work: Vec<PathBuf> = Vec::new();
    // a copied folder becomes a library folder of the same name, its subfolders too: each file's chain of names
    let mut chains: std::collections::HashMap<PathBuf, Vec<String>> = std::collections::HashMap::new();
    let mut total = 0;
    // a location's scan counts its files as it walks them, so an import with one has no total
    let mut endless = false;
    let locations = folder_locations(db)?;
    for f in files {
        let as_copy = copies_in(f, copy, &locations);
        if f.is_dir() && !as_copy {
            endless = true;
            work.push(f.clone());
        } else if f.is_dir() {
            let inner = files_under(f, scan, as_copy);
            total += inner.len();
            let top = f.parent().unwrap_or(f);
            for file in &inner {
                let rel = file.parent().and_then(|p| p.strip_prefix(top).ok()).map(|p| p.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect()).unwrap_or_default();
                chains.insert(file.clone(), rel);
            }
            work.extend(inner);
        } else {
            total += 1;
            work.push(f.clone());
        }
    }
    // a copied file's companions in its folder come with it, and join it once both are in, under whatever name it gets
    let copies: Vec<PathBuf> = work.iter().filter(|f| !f.is_dir() && copies_in(f, copy, &locations)).cloned().collect();
    let planned = crate::core::grouping::companions::plan_copies(&copies, &scan.companions);
    let given: HashSet<PathBuf> = work.iter().cloned().collect();
    let mut brought: HashSet<PathBuf> = HashSet::new();
    for (_, member, _) in &planned {
        if !given.contains(member) && brought.insert(member.clone()) {
            work.push(member.clone());
            total += 1;
        }
    }
    if endless {
        total = 0;
    }
    let mut copied: std::collections::HashMap<PathBuf, AssetId> = std::collections::HashMap::new();
    let mut made: std::collections::HashMap<(Option<i64>, String), i64> = std::collections::HashMap::new();
    let mut ids = Vec::new();
    let mut done = 0;
    for f in &work {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let as_copy = copies_in(f, copy, &locations);
        // a companion comes only with an item that came in, and never replaces a file
        if brought.contains(f) && !planned.iter().any(|(i, m, _)| m == f && copied.contains_key(i)) {
            done += 1;
            continue;
        }
        let mut replaced = false;
        let mut seen = 1;
        // what a location's scan told of while it ran, not told again below
        let mut streamed: HashSet<AssetId> = HashSet::new();
        let new_ids = if f.is_dir() {
            let root_path = filesystem::canonical(f);
            let rid = db.add_root(&root_path.to_string_lossy(), RootKind::Linked, false)?;
            step(ImportStep { done, total, name: "", new: &[], location: Some(Located::Started(rid)) });
            let root = db.root(rid).ok_or("no such location")?;
            let base = done;
            let walked = std::cell::Cell::new(0);
            let told = std::cell::RefCell::new(Vec::new());
            let during = |n: usize, what: &str| {
                walked.set(n);
                step(ImportStep { done: base + n, total, name: what, new: &[], location: None })
            };
            let found = |ids: &[AssetId]| {
                told.borrow_mut().extend_from_slice(ids);
                step(ImportStep { done: base + walked.get(), total, name: "", new: ids, location: None })
            };
            let (stats, ev) = crate::core::scan::scan_root_with(db, &root, scan, &during, &found, cancel)?;
            seen = stats.seen;
            streamed = told.into_inner().into_iter().collect();
            step(ImportStep { done: base + seen, total, name: "", new: &[], location: Some(Located::Scanned(rid, stats)) });
            let mut list: Vec<AssetId> = ev.iter().map(|e| e.id()).collect();
            // a folder that was already known still belongs in the chosen folder
            if list.is_empty() && folder.is_some() {
                list = db.search(&crate::core::model::Scope::Root(rid), "", &Default::default(), false, &db.field_catalog())?.into_iter().map(|(i, _)| i).collect();
            }
            list
        } else if as_copy {
            let target = match chains.get(f) {
                _ if brought.contains(f) => None,
                Some(chain) => Some(folder_chain(db, folder, chain, &mut made)?),
                None => folder,
            };
            let clash = if brought.contains(f) { OnClash::Keep } else { clash };
            let (id, outcome) = import_one(db, paths, Incoming::File(f), &filesystem::name_of(f), target, clash, style, None)?;
            replaced = outcome == Outcome::Replaced;
            if outcome == Outcome::New {
                copied.insert(f.clone(), id);
            }
            if outcome == Outcome::Skipped {
                vec![]
            } else {
                vec![id]
            }
        } else {
            crate::core::scan::add_loose_files(db, std::slice::from_ref(f), &scan.companions)?
        };
        // a replaced item keeps its own metadata
        let meta = match new_ids.as_slice() {
            _ if brought.contains(f) => None,
            [_] if !replaced => transfer.get(&crate::core::library::instance::transfer_key(&f.to_string_lossy())).cloned().or_else(|| neighbour_metadata(f)),
            _ => None,
        };
        db.tx(|db| {
            if let (Some(m), [id]) = (&meta, new_ids.as_slice()) {
                apply_incoming(db, *id, m)?;
            }
            let target = match chains.get(f) {
                _ if brought.contains(f) => None,
                Some(chain) if !new_ids.is_empty() => Some(folder_chain(db, folder, chain, &mut made)?),
                _ => folder,
            };
            if let Some(fid) = target {
                db.add_to_folder(&new_ids, fid)?;
            }
            Ok(())
        })?;
        done += seen;
        let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let untold: Vec<AssetId> = new_ids.iter().copied().filter(|id| !streamed.contains(id)).collect();
        step(ImportStep { done, total, name: &name, new: &untold, location: None });
        ids.extend(new_ids);
    }
    let mut members = HashSet::new();
    for (item, member, p) in &planned {
        let (Some(&i), Some(&m)) = (copied.get(item), copied.get(member)) else { continue };
        if db.group(i, m, &p.role, &p.rule, p.suffix.as_deref(), p.cover).is_ok() {
            members.insert(m);
        }
    }
    ids.retain(|id| !members.contains(id));
    Ok(ids)
}

/// The library folder at `chain` below `parent`, made where missing; a folder of that name already there is used.
fn folder_chain(db: &Db, parent: Option<i64>, chain: &[String], made: &mut std::collections::HashMap<(Option<i64>, String), i64>) -> DbResult<i64> {
    let mut at = parent;
    for name in chain {
        let key = (at, name.to_lowercase());
        let id = match made.get(&key) {
            Some(id) => *id,
            None => {
                let existing = db.subfolders(at)?.into_iter().find(|(_, n)| n.to_lowercase() == key.1).map(|(id, _)| id);
                let id = match existing {
                    Some(id) => id,
                    None => db.create_folder(name, at)?,
                };
                made.insert(key, id);
                id
            }
        };
        at = Some(id);
    }
    at.ok_or_else(|| "empty folder chain".to_string())
}
