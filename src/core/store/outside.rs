//! Changes made in the managed store (`files/`) outside Curio: Explorer renames, moves and deletes, files dropped in by hand, and editors that save by replacing a file.
//!
//! Files are matched to placements by filesystem identity. A file moved or renamed takes its placement, folder membership and (renamed) item name along; a directory moved or renamed takes its library folder along; a deleted file leaves its folder, or marks its item missing when it was the item's last; a new file joins the folder of its directory; a placement whose file was replaced makes the item's other placements links to the new file.
//! The links of an item indexed in place are views of its location file: renaming one doesn't rename the item, deleting one only takes the item out of that folder, and a replaced one is settled by the store's next pass over the item.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::core::db::{fold_path, Db, DbResult};
use crate::core::system::filesystem;
use crate::core::import::insert_managed_file;
use crate::core::import::managed_root;
use crate::core::library::paths::LibraryPaths;
use crate::core::system::logging;
use crate::core::model::{AssetId, Availability, Placement};
use crate::core::store::{self, TRASH_DIR};

/// A new file younger than this may still be being written; it is looked at again later.
pub const SETTLE_MS: i64 = 2000;

#[derive(Clone, Debug)]
struct OnDisk {
    key: Option<String>,
    size: i64,
    mtime: Option<i64>,
}

#[derive(Debug, Default)]
pub struct Reconciled {
    /// Items whose files, folders, name or availability changed.
    pub changed: Vec<AssetId>,
    /// Items made for files dropped in by hand.
    pub adopted: Vec<AssetId>,
    /// New files still being written: look at these again in a moment.
    pub unsettled: Vec<PathBuf>,
    pub problems: Vec<String>,
}

impl Reconciled {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.adopted.is_empty()
    }
}

fn parent_of(rel: &str) -> &str {
    rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

fn base_of(rel: &str) -> &str {
    rel.rsplit_once('/').map(|(_, n)| n).unwrap_or(rel)
}

fn in_trash(rel: &str) -> bool {
    rel == TRASH_DIR || rel.starts_with(&format!("{TRASH_DIR}/"))
}

fn rel_of(paths: &LibraryPaths, p: &Path) -> Option<String> {
    p.strip_prefix(&paths.managed).ok().map(filesystem::rel_to_db).filter(|r| !r.is_empty())
}

/// Files under `dir` (store-relative, `""` = all of it), our own temporaries and hidden entries left out; `.trash` is walked too.
fn walk(paths: &LibraryPaths, dir: &str, out: &mut HashMap<String, OnDisk>) {
    let start = store::abs(paths, dir);
    let walker = walkdir::WalkDir::new(&start).follow_links(false).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        e.depth() == 0 || name == TRASH_DIR || !filesystem::is_ignored_name(&name)
    });
    for e in walker.filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {
        if filesystem::is_temp_name(&e.file_name().to_string_lossy()) {
            continue;
        }
        if let (Some(rel), Ok(st)) = (rel_of(paths, e.path()), filesystem::stat(e.path())) {
            out.insert(rel, OnDisk { key: st.file_key, size: st.size as i64, mtime: st.mtime });
        }
    }
}

/// Bring the index in line with what is in `files/`. `only` limits the look to these changed paths (from filesystem notifications); `None` looks at everything (at open, after lost notifications).
pub fn reconcile(db: &mut Db, paths: &LibraryPaths, only: Option<&[PathBuf]>, links: bool, now: i64) -> DbResult<Reconciled> {
    let mut out = Reconciled::default();
    let unknown = store::recovery::unknown_pending(db);
    if !unknown.is_empty() {
        logging::warn(&format!("outside-change handling paused: pending file operations from a newer Curio ({})", unknown.join(", ")));
        return Ok(out);
    }
    // finish interrupted store operations first, so their half-done files aren't taken for outside changes
    out.changed.extend(store::recovery::recover(db, paths));
    let root = managed_root(db, paths)?;
    let mut disk: HashMap<String, OnDisk> = HashMap::new();
    let known: Vec<Placement> = match only {
        None => {
            walk(paths, "", &mut disk);
            db.placements()?
        }
        Some(changed) => {
            let mut scope: Vec<String> = vec![];
            for p in changed {
                let Some(rel) = rel_of(paths, p) else { continue };
                if filesystem::is_temp_name(base_of(&rel)) {
                    continue;
                }
                match std::fs::metadata(p) {
                    Ok(m) if m.is_dir() => walk(paths, &rel, &mut disk),
                    Ok(_) => {
                        if let Ok(st) = filesystem::stat(p) {
                            disk.insert(rel.clone(), OnDisk { key: st.file_key, size: st.size as i64, mtime: st.mtime });
                        }
                    }
                    Err(_) => {}
                }
                scope.push(rel);
            }
            if scope.is_empty() {
                return Ok(out);
            }
            let keys: Vec<(String, String)> = scope.iter().map(|r| (fold_path(r), fold_path(&format!("{r}/")))).collect();
            db.placements()?.into_iter().filter(|p| {
                let k = fold_path(&p.rel_path);
                disk.contains_key(&p.rel_path) || keys.iter().any(|(exact, under)| &k == exact || k.starts_with(under))
            }).collect()
        }
    };
    let ids: Vec<i64> = known.iter().map(|p| p.id).collect();
    follow_folder_moves(db, paths, &ids, &disk, &mut out)?;
    follow_files(db, paths, root, &ids, &mut disk, links, now, &mut out)?;
    out.changed.sort();
    out.changed.dedup();
    Ok(out)
}

/// A library folder whose directory is gone while its files turn up, by identity and with the same layout, under one other directory: the folder moved or was renamed there.
fn follow_folder_moves(db: &mut Db, paths: &LibraryPaths, ids: &[i64], disk: &HashMap<String, OnDisk>, out: &mut Reconciled) -> DbResult<()> {
    // links of one item share an identity, so one key can have several paths
    let mut by_key: HashMap<&str, Vec<&str>> = HashMap::new();
    for (r, d) in disk {
        if let Some(k) = d.key.as_deref() {
            by_key.entry(k).or_default().push(r.as_str());
        }
    }
    let mut folders: Vec<(i64, String)> = {
        let mut st = db.conn.prepare("SELECT id, dir FROM folders WHERE dir IS NOT NULL").map_err(|e| e.to_string())?;
        let q = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(|e| e.to_string())?;
        q.filter_map(|r| r.ok()).collect()
    };
    // parents first: moving one takes its subfolders along
    folders.sort_by_key(|(_, d): &(i64, String)| d.matches('/').count());
    for (fid, _) in folders {
        let Some(dir) = db.conn.query_row("SELECT dir FROM folders WHERE id = ?", [fid], |r| r.get::<_, Option<String>>(0)).ok().flatten() else { continue };
        if store::abs(paths, &dir).exists() {
            continue;
        }
        let prefix = format!("{dir}/");
        let mut target: Option<String> = None;
        let mut agree = true;
        for p in ids.iter().filter_map(|id| db.placement(*id)).filter(|p| p.rel_path.starts_with(&prefix) && !disk.contains_key(&p.rel_path)) {
            let Some(key) = fsys_key_of(db, p.asset_id) else { continue };
            let Some(found) = by_key.get(key.as_str()) else { continue };
            let tail = &p.rel_path[prefix.len()..];
            let Some(new_dir) = found.iter().find_map(|f| f.strip_suffix(tail).and_then(|d| d.strip_suffix('/'))) else {
                agree = false;
                break;
            };
            match &target {
                None => target = Some(new_dir.to_string()),
                Some(t) if t == new_dir => {}
                Some(_) => {
                    agree = false;
                    break;
                }
            }
        }
        let Some(to) = target.filter(|t| agree && !in_trash(t) && t != &dir) else { continue };
        let taken: bool = db.conn.query_row("SELECT EXISTS(SELECT 1 FROM folders WHERE dir IS NOT NULL AND fold_path(dir) = ?)", [fold_path(&to)], |r| r.get(0)).unwrap_or(false);
        if taken {
            continue;
        }
        let parent = match to.rsplit_once('/') {
            Some((up, _)) => Some(store::folder_for_dir(db, up)?),
            None => None,
        };
        let name: String = db.conn.query_row("SELECT name FROM folders WHERE id = ?", [fid], |r| r.get(0)).map_err(|e| e.to_string())?;
        let new_name = if store::is_variant(base_of(&to), &store::clean_name(&name, true)) { name } else { base_of(&to).to_string() };
        let moved: Vec<AssetId> = ids.iter().filter_map(|id| db.placement(*id)).filter(|p| p.rel_path.starts_with(&prefix)).map(|p| p.asset_id).collect();
        db.tx(|db| {
            db.move_placement_dir(&dir, &to)?;
            db.move_folder_dirs(&dir, &to)?;
            db.conn.execute("UPDATE folders SET name = ?, parent_id = ? WHERE id = ?", rusqlite::params![new_name, parent, fid]).map_err(|e| e.to_string())?;
            Ok(())
        })?;
        logging::info(&format!("the folder {dir} was moved to {to} outside Curio; the library folder follows"));
        out.changed.extend(moved);
    }
    Ok(())
}

/// The identity recorded for an item's data (the same for all its linked placements).
fn fsys_key_of(db: &Db, asset: AssetId) -> Option<String> {
    db.conn.query_row("SELECT file_key FROM assets WHERE id = ?", [asset], |r| r.get::<_, Option<String>>(0)).ok().flatten()
}

fn folder_of_dir(db: &Db, dir: &str) -> DbResult<Option<i64>> {
    if dir.is_empty() {
        return Ok(None);
    }
    store::folder_for_dir(db, dir).map(Some)
}

/// Point placement `p` at `rel` and move its item's membership from the old folder to the one of `rel`'s directory. A file renamed outside renames its item.
fn move_to(db: &mut Db, p: &Placement, rel: &str, d: &OnDisk) -> DbResult<()> {
    if is_member(db, p.asset_id) {
        return member_moved(db, p, rel, d);
    }
    let folder = folder_of_dir(db, parent_of(rel))?;
    let name: String = db.conn.query_row("SELECT name FROM assets WHERE id = ?", [p.asset_id], |r| r.get(0)).map_err(|e| e.to_string())?;
    let indexed = !db.in_store(p.asset_id);
    let renamed = !store::is_variant(base_of(rel), &store::clean_name(&name, false));
    db.tx(|db| {
        let clash = folder.is_some() && folder != p.folder_id && db.placements_of(p.asset_id)?.iter().any(|o| o.folder_id == folder);
        // a second entry of the item in a folder that already has one: the store removes the extra link
        let record = if clash { None } else { folder };
        db.move_placement(p.id, record, rel)?;
        if !clash && p.folder_id != folder {
            if let Some(old) = p.folder_id {
                db.remove_from_folder(&[p.asset_id], old)?;
            }
            if let Some(new) = folder {
                db.add_to_folder(&[p.asset_id], new)?;
            }
        }
        if indexed {
            // the link takes the item's name again
            return db.mark_store_dirty(&[p.asset_id]);
        }
        if renamed && !clash {
            db.set_file_name(p.asset_id, base_of(rel))?;
        }
        db.conn.execute("UPDATE assets SET status = 0, file_key = COALESCE(?, file_key) WHERE id = ?", rusqlite::params![d.key, p.asset_id]).map_err(|e| e.to_string())?;
        Ok(())
    })
}

fn is_member(db: &Db, asset: AssetId) -> bool {
    db.conn.query_row("SELECT part_of IS NOT NULL FROM assets WHERE id = ?", [asset], |r| r.get(0)).unwrap_or(false)
}

/// A grouped file's entry moved or renamed outside: it keeps the entry, and the store's next pass puts it back beside its item under its name. Its item's folders and its own name stay.
fn member_moved(db: &mut Db, p: &Placement, rel: &str, d: &OnDisk) -> DbResult<()> {
    let folder = p.folder_id.filter(|_| parent_of(rel) == parent_of(&p.rel_path));
    db.tx(|db| {
        db.move_placement(p.id, folder, rel)?;
        db.conn.execute("UPDATE assets SET status = 0, file_key = COALESCE(?, file_key) WHERE id = ?", rusqlite::params![d.key, p.asset_id]).map_err(|e| e.to_string())?;
        db.mark_store_dirty(&[p.asset_id])
    })
}

#[allow(clippy::too_many_arguments)]
fn follow_files(db: &mut Db, paths: &LibraryPaths, root: i64, ids: &[i64], disk: &mut HashMap<String, OnDisk>, links: bool, now: i64, out: &mut Reconciled) -> DbResult<()> {
    let known: Vec<Placement> = ids.iter().filter_map(|id| db.placement(*id)).collect();
    let known_rels: HashSet<String> = db.placements()?.into_iter().map(|p| p.rel_path).collect();
    // an interrupted relink left the new link beside its place
    for p in &known {
        if !disk.contains_key(&p.rel_path) {
            let tmp = PathBuf::from(format!("{}.relinking", store::abs(paths, &p.rel_path).display()));
            if tmp.exists() && std::fs::rename(&tmp, store::abs(paths, &p.rel_path)).is_ok() {
                if let Ok(st) = filesystem::stat(&store::abs(paths, &p.rel_path)) {
                    disk.insert(p.rel_path.clone(), OnDisk { key: st.file_key, size: st.size as i64, mtime: st.mtime });
                }
            }
        }
    }
    let missing: Vec<Placement> = known.iter().filter(|p| !disk.contains_key(&p.rel_path)).cloned().collect();
    let mut fresh: HashMap<String, OnDisk> = disk.iter().filter(|(r, _)| !known_rels.contains(*r)).map(|(r, d)| (r.clone(), d.clone())).collect();
    // moved or renamed: the same file under another name
    let mut gone = vec![];
    for p in missing {
        let key = fsys_key_of(db, p.asset_id);
        let found = key.as_deref().and_then(|k| fresh.iter().find(|(r, d)| d.key.as_deref() == Some(k) && !in_trash(r)).map(|(r, _)| r.clone()));
        match found {
            Some(rel) => {
                let d = fresh.remove(&rel).unwrap();
                move_to(db, &p, &rel, &d)?;
                out.changed.push(p.asset_id);
            }
            None => gone.push(p),
        }
    }
    // deleted: the item leaves that folder (a link in the trash: that trashed entry is gone), or is missing when no file of it is left
    for p in gone {
        let others_here = !db.in_store(p.asset_id) || db.placements_of(p.asset_id)?.into_iter().any(|o| o.id != p.id && store::abs(paths, &o.rel_path).exists());
        if others_here && is_member(db, p.asset_id) {
            // a grouped file's entry: forgotten, and made again beside its item's by the next pass
            db.tx(|db| {
                db.remove_placement(p.id)?;
                db.mark_store_dirty(&[p.asset_id])
            })?;
        } else if others_here {
            db.tx(|db| {
                db.remove_placement(p.id)?;
                match p.folder_id {
                    Some(f) if in_trash(&p.rel_path) => {
                        db.conn.execute("DELETE FROM trashed_entries WHERE asset_id = ? AND folder_id = ?", [p.asset_id, f]).map_err(|e| e.to_string())?;
                    }
                    Some(f) => db.remove_from_folder(&[p.asset_id], f)?,
                    None => {}
                }
                Ok(())
            })?;
        } else {
            db.set_status(&[p.asset_id], Availability::Missing)?;
        }
        out.changed.push(p.asset_id);
    }
    // new: a missing item come back (restored from the Recycle Bin), or a file dropped in by hand
    let present_keys: HashSet<String> = db
        .conn
        .prepare("SELECT file_key FROM assets a WHERE status = 0 AND file_key IS NOT NULL AND (root_id = ? OR EXISTS (SELECT 1 FROM placements p WHERE p.asset_id = a.id))")
        .and_then(|mut st| st.query_map([root], |r| r.get::<_, String>(0)).map(|q| q.filter_map(|r| r.ok()).collect()))
        .map_err(|e| e.to_string())?;
    // read after the placements: a purge forgets its rows and records its recycles in one transaction
    let recycling = db.recycle_paths(root)?;
    let mut fresh: Vec<(String, OnDisk)> = fresh.into_iter().collect();
    fresh.sort_by(|a, b| a.0.cmp(&b.0));
    for (rel, d) in fresh {
        if in_trash(&rel) || recycling.contains(&rel) {
            continue;
        }
        let abs = store::abs(paths, &rel);
        if d.key.as_deref().is_some_and(|k| present_keys.contains(k)) {
            // another entry of an item already here (a link made by hand): left alone, listed as a stray
            continue;
        }
        if d.mtime.is_some_and(|m| now - m < SETTLE_MS) || !crate::core::import::auto_import::free_to_read(&abs) {
            out.unsettled.push(abs);
            continue;
        }
        let qh = filesystem::quick_hash(&abs, d.size as u64).ok();
        let back = db.find_move_candidate(d.key.as_deref(), qh.as_deref(), d.size).filter(|id| db.asset(*id).ok().flatten().is_some_and(|a| a.root_id == root));
        if let Some(id) = back {
            if let Some(p) = db.placements_of(id)?.into_iter().next() {
                move_to(db, &p, &rel, &d)?;
                out.changed.push(id);
                continue;
            }
        }
        let folder = folder_of_dir(db, parent_of(&rel))?;
        match db.tx(|db| insert_managed_file(db, paths, root, &abs, None, folder)) {
            Ok(id) => out.adopted.push(id),
            Err(e) => out.problems.push(format!("{rel}: {e}")),
        }
    }
    // replaced or edited: present placements whose file changed
    let mut by_asset: HashMap<AssetId, Vec<Placement>> = HashMap::new();
    for p in ids.iter().filter_map(|id| db.placement(*id)) {
        if disk.contains_key(&p.rel_path) {
            by_asset.entry(p.asset_id).or_default().push(p);
        }
    }
    for (asset, here) in by_asset {
        match follow_content(db, paths, asset, &here, disk, links) {
            Ok(true) => out.changed.push(asset),
            Ok(false) => {}
            Err(e) => out.problems.push(e),
        }
    }
    Ok(())
}

/// An item whose linked placements no longer share one file (an editor saved one by replacing it): the newest wins and the others become links to it, their old files going to the Recycle Bin. An item edited in place gets its size, date and thumbnail renewed.
fn follow_content(db: &mut Db, paths: &LibraryPaths, asset: AssetId, here: &[Placement], disk: &HashMap<String, OnDisk>, links: bool) -> DbResult<bool> {
    let a = db.asset(asset)?.ok_or("no such item")?;
    if !db.in_store(asset) {
        let key = fsys_key_of(db, asset);
        if here.iter().any(|p| disk[&p.rel_path].size != a.size || (p.linked && disk[&p.rel_path].key != key)) {
            db.mark_store_dirty(&[asset])?;
        }
        return Ok(false);
    }
    let linked: Vec<&Placement> = here.iter().filter(|p| p.linked).collect();
    let keys: HashSet<Option<&str>> = linked.iter().map(|p| disk[&p.rel_path].key.as_deref()).collect();
    if linked.len() > 1 && keys.len() > 1 {
        let winner = linked.iter().max_by_key(|p| disk[&p.rel_path].mtime.unwrap_or(0)).unwrap();
        let src = store::abs(paths, &winner.rel_path);
        let wkey = disk[&winner.rel_path].key.clone();
        for p in linked.iter().filter(|p| disk[&p.rel_path].key != wkey) {
            let at = store::abs(paths, &p.rel_path);
            let tmp = PathBuf::from(format!("{}.relinking", at.display()));
            let _ = std::fs::remove_file(&tmp);
            store::disk::link_or_copy(&src, &tmp, links).map_err(|e| format!("{}: {e}", p.rel_path))?;
            if let Err(e) = filesystem::to_recycle_bin(&at) {
                let _ = std::fs::remove_file(&tmp);
                return Err(format!("{}: {e}", p.rel_path));
            }
            std::fs::rename(&tmp, &at).map_err(|e| format!("{}: {e}", p.rel_path))?;
        }
        logging::info(&format!("{} was replaced outside Curio; its other folders now have the new file", winner.rel_path));
        let w = &disk[&winner.rel_path];
        db.tx(|db| {
            db.file_rewritten(asset, None, w.size, w.mtime)?;
            db.conn.execute("UPDATE assets SET file_key = ?, status = 0 WHERE id = ?", rusqlite::params![w.key, asset]).map_err(|e| e.to_string())
        })?;
        return Ok(true);
    }
    let Some(primary) = disk.get(&a.rel_path) else { return Ok(false) };
    if primary.size == a.size && primary.mtime == a.mtime && a.status == Availability::Ok {
        return Ok(false);
    }
    db.tx(|db| {
        if primary.size != a.size || primary.mtime != a.mtime {
            db.file_rewritten(asset, None, primary.size, primary.mtime)?;
        }
        db.conn.execute("UPDATE assets SET file_key = COALESCE(?, file_key), status = 0 WHERE id = ?", rusqlite::params![primary.key, asset]).map_err(|e| e.to_string())
    })?;
    Ok(true)
}
