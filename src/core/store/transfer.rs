//! An item's file moving between the library's store and its locations, or within a location: the item, its metadata and its folders stay, only where its file lives changes.
//!
//! On one drive a move is a rename; across drives it is a verified copy whose source goes to the Recycle Bin once the item points at the copy. Each move is recorded in `file_ops` (`item_move`) before the file is touched, so `recover` finishes or drops it after a crash. It must run outside any transaction: on the scanner thread in the app.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::db::{Db, DbResult};
use crate::core::system::filesystem;
use crate::core::import::managed_root;
use crate::core::library::paths::LibraryPaths;
use crate::core::system::logging;
use crate::core::model::{AssetId, FileOp, RootAccess, RootKind};
use crate::core::store::{self, SuffixStyle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The source stays; a linked source is remembered so its location's scan doesn't add it back.
    Copy,
    /// The source goes: renamed on one drive, else copied, verified and sent to the Recycle Bin.
    Move,
    /// As `Copy`, with the store's file a hard link to the source where the drive allows: for sources meant to be deleted afterwards, as an Eagle library's.
    Link,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Detail {
    to_root: i64,
    /// The library folder an item coming into the store is placed in.
    folder: Option<i64>,
    copy: bool,
    /// The item's name afterwards (an item coming into the store is named by its title).
    name: String,
}

fn base_of(rel: &str) -> &str {
    rel.rsplit_once('/').map(|(_, n)| n).unwrap_or(rel)
}

fn parent_of(rel: &str) -> &str {
    rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

fn abs_in(db: &Db, root: i64, rel: &str) -> DbResult<PathBuf> {
    let r = db.root(root).ok_or_else(|| format!("no location {root}"))?;
    Ok(filesystem::db_to_abs(&r.path, rel))
}

fn rename(src: &Path, dst: &Path) -> io::Result<()> {
    store::disk::rename_no_clobber(src, dst)
}

pub(super) fn crosses_devices(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::CrossesDevices || (cfg!(windows) && e.raw_os_error() == Some(17)) || (cfg!(unix) && e.raw_os_error() == Some(18))
}

/// A free path for a file called `name` in the directory `dir` of a location: nothing on disk there and no item recorded there; numbered with `style` from 2. `own` is the file's current path, which differs only in case from a free one on Windows.
pub fn free_in_dir(db: &Db, root: i64, dir: &str, name: &str, own: Option<&str>, style: SuffixStyle) -> DbResult<String> {
    free_in_dir_with(db, root, dir, name, own, style, &[])
}

/// As `free_in_dir`, for an item whose grouped files (their suffixes and their own paths) take names beside it, which must be free too.
pub fn free_in_dir_with(db: &Db, root: i64, dir: &str, name: &str, own: Option<&str>, style: SuffixStyle, members: &[(String, String)]) -> DbResult<String> {
    let base = store::clean_name(name, false);
    let taken = |c: &str, own: Option<&str>| -> bool {
        let mine = own.is_some_and(|o| crate::core::db::fold_path(o) == crate::core::db::fold_path(c));
        !mine && (abs_in(db, root, c).map(|p| p.symlink_metadata().is_ok()).unwrap_or(true) || db.asset_at(root, c).is_some())
    };
    let mut cand = join(dir, &base);
    let mut n = 1;
    while taken(&cand, own) || members.iter().any(|(s, m)| taken(&join(dir, &format!("{}{s}", crate::core::grouping::companions::stem(base_of(&cand)))), Some(m))) {
        n += 1;
        cand = join(dir, &style.apply(&base, n));
    }
    Ok(cand)
}

/// The files grouped into `item` that follow it by name: id, suffix and path.
fn followers(db: &Db, item: AssetId) -> DbResult<Vec<(AssetId, String, String)>> {
    Ok(db.members_of(item)?.into_iter().filter_map(|p| p.suffix.map(|s| (p.id, s, p.rel_path))).collect())
}

/// After `item` moved to `to_rel` in `to_root`: each file grouped into it goes beside it, named after it (`free` gives a free path for a name in a directory). One that can't go stays where it is and is reported; it stays grouped.
#[allow(clippy::too_many_arguments)]
fn bring_members(db: &mut Db, item: AssetId, to_root: i64, to_rel: &str, mode: Mode, folder: Option<i64>, problems: &mut Vec<String>, free: &dyn Fn(&Db, &str, &str) -> DbResult<String>) -> DbResult<()> {
    for p in db.members_of(item)? {
        let want = match &p.suffix {
            Some(s) => format!("{}{s}", crate::core::grouping::companions::stem(base_of(to_rel))),
            None => p.name.clone(),
        };
        let rel = free(db, parent_of(to_rel), &want)?;
        if let Err(e) = move_item(db, p.id, to_root, &rel, mode, folder, base_of(&rel)) {
            problems.push(e);
        }
    }
    Ok(())
}

/// Move or copy `asset`'s file to `to_rel` in the root `to_root` and point the item there. `folder` places an item coming into the store; `name` is its name afterwards.
pub fn move_item(db: &mut Db, asset: AssetId, to_root: i64, to_rel: &str, mode: Mode, folder: Option<i64>, name: &str) -> DbResult<()> {
    if !db.conn.is_autocommit() {
        return Err("moving files can't run inside a transaction".into());
    }
    let a = db.asset(asset)?.ok_or("no such item")?;
    let src = PathBuf::from(&a.abs_path);
    let dst = abs_in(db, to_root, to_rel)?;
    let st = filesystem::stat(&src).map_err(|_| format!("{} is missing", a.name))?;
    let detail = Detail { to_root, folder, copy: mode != Mode::Move, name: name.to_string() };
    let mut op = FileOp {
        kind: "item_move".into(),
        root_id: a.root_id,
        rel_path: a.rel_path.clone(),
        dest: Some(to_rel.to_string()),
        asset_uid: Some(a.uid.clone()),
        size: Some(st.size as i64),
        mtime: st.mtime,
        state: "pending".into(),
        detail: Some(serde_json::to_string(&detail).unwrap_or_default()),
        ..Default::default()
    };
    op.id = db.add_file_op(&op)?;
    let copied = match mode {
        Mode::Copy => store::disk::copy_verified(&src, &dst).map(|_| true),
        Mode::Link => store::disk::link_or_copy(&src, &dst, true).map(|_| true),
        Mode::Move => match rename(&src, &dst) {
            Ok(()) => Ok(false),
            Err(e) if crosses_devices(&e) => store::disk::copy_verified(&src, &dst).map(|_| true),
            Err(e) => Err(e),
        },
    };
    let copied = match copied {
        Ok(c) => c,
        Err(e) => {
            db.remove_file_op(op.id)?;
            return Err(format!("{}: {e}", a.name));
        }
    };
    // with both files there after a crash, only this says the target is the finished copy
    if copied {
        db.set_file_op_state(op.id, "copied", None)?;
    }
    finish(db, &op, &detail, copied)
}

/// Point the item at its new file and clear the op, in one transaction. A move that had to copy queues its source for the Recycle Bin; a copy of a linked file remembers the source so its location's scan doesn't add it back.
fn finish(db: &mut Db, op: &FileOp, d: &Detail, copied: bool) -> DbResult<()> {
    let uid = op.asset_uid.clone().unwrap_or_default();
    let asset = db.asset_by_uid(&uid).ok_or("the item is gone")?;
    let to_rel = op.dest.clone().unwrap_or_default();
    let dst = abs_in(db, d.to_root, &to_rel)?;
    let st = filesystem::stat(&dst).map_err(|e| e.to_string())?;
    let from = db.root(op.root_id);
    let to = db.root(d.to_root).ok_or("no such location")?;
    db.tx(|db| {
        if from.as_ref().is_some_and(|r| r.kind == RootKind::Managed) {
            // the item's other placements stay as links in its folders
            db.conn.execute("DELETE FROM placements WHERE asset_id = ? AND rel_key = fold_path(?)", rusqlite::params![asset, op.rel_path]).map_err(|e| e.to_string())?;
            db.mark_store_dirty(&[asset])?;
        }
        if to.kind == RootKind::Managed {
            db.conn
                .execute("UPDATE assets SET root_id = ?, rel_path = ?, name = ?, display_name = '' WHERE id = ?", rusqlite::params![d.to_root, to_rel, d.name, asset])
                .map_err(|e| e.to_string())?;
            let folder = d.folder.filter(|f| db.conn.query_row("SELECT EXISTS(SELECT 1 FROM folders WHERE id = ?)", [f], |r| r.get::<_, bool>(0)).unwrap_or(false));
            db.add_placement(asset, folder, &to_rel, true)?;
            // its other folders get links through the store queue
            db.mark_store_dirty(&[asset])?;
        } else {
            db.conn.execute("UPDATE assets SET root_id = ?, rel_path = ?, name = ? WHERE id = ?", rusqlite::params![d.to_root, to_rel, d.name, asset]).map_err(|e| e.to_string())?;
        }
        db.conn.execute("UPDATE assets SET file_key = ?, size = ?, mtime = ?, status = 0 WHERE id = ?", rusqlite::params![st.file_key, st.size as i64, st.mtime, asset]).map_err(|e| e.to_string())?;
        // its name may have changed, and a name given in Curio is gone
        db.refresh_fts(&[asset])?;
        if copied && !d.copy {
            db.add_file_op(&FileOp { kind: "recycle".into(), root_id: op.root_id, rel_path: op.rel_path.clone(), asset_uid: Some(uid.clone()), size: op.size, mtime: op.mtime, state: "pending".into(), ..Default::default() })?;
        }
        if d.copy && from.as_ref().is_some_and(|r| r.kind == RootKind::Linked && !r.explicit_only) {
            db.conn
                .execute("INSERT INTO forgotten_files(root_id, rel_path, size, mtime) VALUES(?, ?, ?, ?) ON CONFLICT(root_id, rel_path) DO UPDATE SET size = excluded.size, mtime = excluded.mtime", rusqlite::params![op.root_id, op.rel_path, op.size.unwrap_or(0), op.mtime])
                .map_err(|e| e.to_string())?;
        }
        db.remove_file_op(op.id)
    })
}

/// Finish or drop item moves cut short. Returns the items that moved.
pub fn recover(db: &mut Db) -> Vec<AssetId> {
    let mut moved = vec![];
    for op in db.file_ops(Some("item_move"), None, 100_000).unwrap_or_default() {
        if op.state == "failed" {
            continue;
        }
        match recover_one(db, &op) {
            Ok(Some(id)) => moved.push(id),
            Ok(None) => {}
            Err(why) => {
                logging::error(&format!("moving {}: {why}", op.rel_path));
                let _ = db.set_file_op_state(op.id, "failed", Some(&why));
            }
        }
    }
    moved
}

fn recover_one(db: &mut Db, op: &FileOp) -> DbResult<Option<AssetId>> {
    let d: Detail = op.detail.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();
    let Some(asset) = op.asset_uid.as_deref().and_then(|u| db.asset_by_uid(u)) else {
        db.remove_file_op(op.id)?;
        return Ok(None);
    };
    let to_rel = op.dest.clone().unwrap_or_default();
    let a = db.asset(asset)?.ok_or("no such item")?;
    if a.root_id == d.to_root && a.rel_path == to_rel {
        db.remove_file_op(op.id)?;
        return Ok(None);
    }
    let src = abs_in(db, op.root_id, &op.rel_path)?;
    let dst = abs_in(db, d.to_root, &to_rel)?;
    if op.root_id == d.to_root && crate::core::db::fold_path(&op.rel_path) == crate::core::db::fold_path(&to_rel) {
        if store::disk::name_on_disk(&dst).as_deref() == Some(base_of(&to_rel)) {
            finish(db, op, &d, false)?;
            return Ok(Some(asset));
        }
        db.remove_file_op(op.id)?;
        return Ok(None);
    }
    match (src.exists(), dst.exists()) {
        (false, true) => finish(db, op, &d, false)?,
        (true, false) => {
            db.remove_file_op(op.id)?;
            return Ok(None);
        }
        (true, true) => {
            let copy = op.state == "copied" && crate::core::store::journal::unchanged(&src, op) && std::fs::metadata(&dst).is_ok_and(|m| Some(m.len() as i64) == op.size);
            if !copy {
                return Err(format!("both {} and {} exist and the copy wasn't recorded as finished, so neither was touched", src.display(), dst.display()));
            }
            finish(db, op, &d, true)?
        }
        (false, false) => return Err(format!("neither {} nor {} is there", src.display(), dst.display())),
    }
    Ok(Some(asset))
}

/// What one batch of moves did.
#[derive(Debug, Default)]
pub struct Outcome {
    pub moved: Vec<AssetId>,
    pub problems: Vec<String>,
    /// How many of `moved` share their data with the source through a hard link.
    pub linked: usize,
}

/// The title an item coming into the store takes as its file name: the name it shows, with its extension.
fn title_of(display: &str, name: &str, ext: &str) -> String {
    let t = if display.trim().is_empty() { name } else { display.trim() };
    if ext.is_empty() || t.to_lowercase().ends_with(&format!(".{ext}")) {
        t.to_string()
    } else {
        format!("{t}.{ext}")
    }
}

/// Copy or move linked items into the library's store, into the directory of `target` (a folder they join first, as a drop onto it does) or else of their first folder. Moving needs a location set to organize files. A copy out of an Eagle library is a hard link where the drive allows, as importing one is.
pub fn into_library(db: &mut Db, paths: &LibraryPaths, ids: &[AssetId], mode: Mode, target: Option<i64>, style: SuffixStyle) -> DbResult<Outcome> {
    let managed = managed_root(db, paths)?;
    let mut out = Outcome::default();
    for &id in ids {
        let Some(a) = db.asset(id)? else { continue };
        let Some(root) = db.root(a.root_id) else { continue };
        if root.kind == RootKind::Managed {
            continue;
        }
        if a.deleted {
            out.problems.push(format!("{}: it is in the trash", a.name));
            continue;
        }
        let mode = if mode == Mode::Copy && crate::core::import::eagle::eagle_images_path(&root.path) { Mode::Link } else { mode };
        if let Some(f) = target {
            db.tx(|db| db.add_to_folder(&[id], f))?;
        }
        if mode == Mode::Move && root.access != RootAccess::Organize {
            out.problems.push(format!("{}: {} isn't set to organize files, so its files stay there (copy them instead, or allow it in Settings › Storage)", a.name, if root.label.is_empty() { &root.path } else { &root.label }));
            continue;
        }
        // its links in its folders give way to the file coming in, which the store links there again; so do those of the files grouped into it
        let members: Vec<AssetId> = db.members_of(id)?.into_iter().map(|p| p.id).collect();
        if let Err(e) = std::iter::once(id).chain(members.iter().copied()).try_for_each(|m| store::placing::drop_links(db, paths, m)) {
            db.mark_store_dirty(&[id])?;
            db.mark_store_dirty(&members)?;
            out.problems.push(format!("{}: {e}", a.name));
            continue;
        }
        let folder: Option<i64> = target.or_else(|| db.conn.query_row("SELECT folder_id FROM asset_folders WHERE asset_id = ? ORDER BY folder_id LIMIT 1", [id], |r| r.get(0)).ok());
        let dir = store::folder_dir(db, paths, folder)?;
        let title = title_of(&a.display_name, &a.name, &a.ext);
        let rel = store::free_for(db, paths, &dir, &title, style, &store::own_of(db, id, None));
        match move_item(db, id, managed, &rel, mode, folder, &title) {
            Ok(()) => {
                bring_members(db, id, managed, &rel, mode, folder, &mut out.problems, &|db, dir, name| Ok(store::free_name(db, paths, dir, name, style)))?;
                if mode == Mode::Link && filesystem::file_key(Path::new(&a.abs_path)).is_some_and(|k| filesystem::file_key(&store::abs(paths, &rel)) == Some(k)) {
                    out.linked += 1;
                }
                out.moved.push(id);
            }
            Err(e) => {
                db.mark_store_dirty(&[id])?;
                out.problems.push(e);
            }
        }
    }
    Ok(out)
}

/// The location holding `dir` (the innermost when locations nest).
pub fn location_of(db: &Db, dir: &Path) -> DbResult<Option<crate::core::model::Root>> {
    let dir = filesystem::canonical(dir);
    Ok(db.roots()?.into_iter().filter(|r| r.kind == RootKind::Linked && dir.starts_with(filesystem::canonical(Path::new(&r.path)))).max_by_key(|r| r.path.len()))
}

/// Move items' files into `dir`, a folder inside one of the library's locations that isn't read-only. Copied items leave the store and keep their library folders, whose links become links to the moved file; linked items need a location set to organize files.
pub fn to_folder(db: &mut Db, ids: &[AssetId], dir: &Path, style: SuffixStyle) -> DbResult<Outcome> {
    let root = location_of(db, dir)?.ok_or("Choose a folder inside one of the library's locations.")?;
    db.require_writable(root.id)?;
    let canon_root = filesystem::canonical(Path::new(&root.path));
    let dir_rel = filesystem::rel_to_db(filesystem::canonical(dir).strip_prefix(&canon_root).unwrap_or(Path::new("")));
    let mut out = Outcome::default();
    for &id in ids {
        let Some(a) = db.asset(id)? else { continue };
        let Some(from) = db.root(a.root_id) else { continue };
        if a.deleted {
            out.problems.push(format!("{}: it is in the trash", a.name));
            continue;
        }
        if from.kind == RootKind::Linked && from.access != RootAccess::Organize {
            out.problems.push(format!("{}: {} isn't set to organize files, so its file stays there", a.name, if from.label.is_empty() { &from.path } else { &from.label }));
            continue;
        }
        if from.id == root.id && parent_of(&a.rel_path) == dir_rel {
            continue;
        }
        let suffixes: Vec<(String, String)> = followers(db, id)?.into_iter().map(|(_, s, r)| (s, r)).collect();
        let rel = free_in_dir_with(db, root.id, &dir_rel, &a.name, None, style, &suffixes)?;
        match move_item(db, id, root.id, &rel, Mode::Move, None, base_of(&rel)) {
            Ok(()) => {
                bring_members(db, id, root.id, &rel, Mode::Move, None, &mut out.problems, &|db, dir, name| free_in_dir(db, root.id, dir, name, None, style))?;
                out.moved.push(id);
            }
            Err(e) => out.problems.push(e),
        }
    }
    Ok(out)
}

/// Rename a linked item's file in place to match its name, in a location set to organize files.
pub fn follow_name(db: &mut Db, asset: AssetId, style: SuffixStyle) -> DbResult<bool> {
    let Some(a) = db.asset(asset)? else { return Ok(false) };
    let Some(root) = db.root(a.root_id) else { return Ok(false) };
    if root.kind != RootKind::Linked || root.access != RootAccess::Organize || a.deleted {
        return Ok(false);
    }
    let mut moved = false;
    if !store::is_variant(base_of(&a.rel_path), &store::clean_name(&a.name, false)) && Path::new(&a.abs_path).exists() {
        let suffixes: Vec<(String, String)> = followers(db, asset)?.into_iter().map(|(_, s, r)| (s, r)).collect();
        let to = free_in_dir_with(db, root.id, parent_of(&a.rel_path), &a.name, Some(&a.rel_path), style, &suffixes)?;
        move_item(db, asset, root.id, &to, Mode::Move, None, base_of(&to))?;
        moved = true;
    }
    // the files grouped into it take its stem; one that couldn't before tries again
    let item_rel: String = db.conn.query_row("SELECT rel_path FROM assets WHERE id = ?", [asset], |r| r.get(0)).map_err(|e| e.to_string())?;
    for (m, suffix, rel) in followers(db, asset)? {
        let want = store::clean_name(&format!("{}{suffix}", crate::core::grouping::companions::stem(base_of(&item_rel))), false);
        let at = join(parent_of(&item_rel), &want);
        let here = abs_in(db, root.id, &rel)?;
        if crate::core::db::fold_path(&at) == crate::core::db::fold_path(&rel) || !here.exists() || db.asset(m)?.is_none_or(|x| x.root_id != root.id) {
            continue;
        }
        let to = free_in_dir(db, root.id, parent_of(&item_rel), &want, Some(&rel), style)?;
        match move_item(db, m, root.id, &to, Mode::Move, None, base_of(&to)) {
            Ok(()) => moved = true,
            Err(e) => logging::warn(&format!("{rel} couldn't follow its item's name: {e}")),
        }
    }
    Ok(moved)
}

