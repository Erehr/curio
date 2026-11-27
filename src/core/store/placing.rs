//! Files placed, moved and removed in the store, each through the journal.

use std::io;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::core::db::{fold_path, Db, DbResult};
use crate::core::system::filesystem;
use crate::core::import::managed_root;
use crate::core::library::paths::LibraryPaths;
use crate::core::model::{AssetId, FileOp, Placement};
use crate::core::store::disk::rename_no_clobber;
use crate::core::store::clean_name;
use crate::core::store::join;
use crate::core::store::base_of;
use crate::core::store::disk::link_or_copy;
use crate::core::store::disk::exists;
use crate::core::store::own_of;
use crate::core::store::free_for;
use crate::core::store::folder_dir;
use crate::core::store::SuffixStyle;
use crate::core::store::is_folder_dir;
use crate::core::store::parent_of;
use crate::core::store::abs;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Detail {
    pub(super) placement: Option<i64>,
    pub(super) folder: Option<i64>,
}

pub(super) fn detail_of(op: &FileOp) -> Detail {
    op.detail.as_deref().and_then(|d| serde_json::from_str(d).ok()).unwrap_or_default()
}

fn begin(db: &Db, paths: &LibraryPaths, kind: &str, rel_path: &str, dest: Option<&str>, asset_uid: Option<String>, detail: &Detail) -> DbResult<FileOp> {
    begin_at(db, managed_root(db, paths)?, &abs(paths, rel_path), kind, rel_path, dest, asset_uid, detail)
}

/// `begin` for an op whose source `rel_path` (at `src`) is in the root `root`, such as a location file being linked into the store.
#[allow(clippy::too_many_arguments)]
fn begin_at(db: &Db, root: i64, src: &Path, kind: &str, rel_path: &str, dest: Option<&str>, asset_uid: Option<String>, detail: &Detail) -> DbResult<FileOp> {
    if !db.conn.is_autocommit() {
        return Err("store operations can't run inside a transaction".into());
    }
    let st = filesystem::stat(src).ok().filter(|s| !s.is_dir);
    let mut op = FileOp {
        kind: kind.into(),
        root_id: root,
        rel_path: rel_path.into(),
        dest: dest.map(|d| d.to_string()),
        asset_uid,
        size: st.as_ref().map(|s| s.size as i64),
        mtime: st.as_ref().and_then(|s| s.mtime),
        state: "pending".into(),
        detail: Some(serde_json::to_string(detail).unwrap_or_default()),
        ..Default::default()
    };
    op.id = db.add_file_op(&op)?;
    Ok(op)
}

/// Drop the op of a file step that failed before changing anything, and pass the error on.
fn abandon<T>(db: &Db, op: &FileOp, err: io::Error, what: &str) -> DbResult<T> {
    db.remove_file_op(op.id)?;
    Err(format!("{what}: {err}"))
}

fn uid_of(db: &Db, asset: AssetId) -> DbResult<String> {
    db.conn.query_row("SELECT uid_text(uid) FROM assets WHERE id = ?", [asset], |r| r.get(0)).map_err(|_| format!("no item {asset}"))
}

pub(super) fn name_of_asset(db: &Db, asset: AssetId) -> DbResult<String> {
    db.conn.query_row("SELECT name FROM assets WHERE id = ?", [asset], |r| r.get(0)).map_err(|_| format!("no item {asset}"))
}

/// A folder deleted while its file was being placed: the placement is recorded at the root and the next pass moves it.
pub(super) fn live_folder(db: &Db, folder: Option<i64>) -> Option<i64> {
    folder.filter(|f| db.conn.query_row("SELECT EXISTS(SELECT 1 FROM folders WHERE id = ?)", [f], |r| r.get::<_, bool>(0)).unwrap_or(false))
}

/// Remove a store file no placement row points at any more, and the directories it leaves empty.
pub fn remove_unplaced(db: &Db, paths: &LibraryPaths, rel: &str) -> io::Result<()> {
    match std::fs::remove_file(abs(paths, rel)) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    prune_dirs(db, paths, parent_of(rel));
    Ok(())
}

/// Remove empty directories from `dir` up towards the store root, stopping at a folder's own, which stays while the folder does.
fn prune_dirs(db: &Db, paths: &LibraryPaths, mut dir: &str) {
    while !dir.is_empty() && !is_folder_dir(db, dir) && std::fs::remove_dir(abs(paths, dir)).is_ok() {
        dir = parent_of(dir);
    }
}

/// Give `asset` a file in `folder` (`None` = store root): a hard link to its existing file. Returns the existing placement when it already has one there.
pub fn place(db: &mut Db, paths: &LibraryPaths, asset: AssetId, folder: Option<i64>, links: bool, style: SuffixStyle) -> DbResult<Placement> {
    if let Some(p) = db.placements_of(asset)?.into_iter().find(|p| p.folder_id == folder) {
        return Ok(p);
    }
    let dir = folder_dir(db, paths, folder)?;
    place_in(db, paths, asset, folder, &dir, links, style)
}

/// A new file entry of the item in the directory `dir`, recorded as being in `folder`: a link to one of its files in the store, or to its location file for an item indexed in place.
pub(super) fn place_in(db: &mut Db, paths: &LibraryPaths, asset: AssetId, folder: Option<i64>, dir: &str, links: bool, style: SuffixStyle) -> DbResult<Placement> {
    let name = name_of_asset(db, asset)?;
    let rel = free_for(db, paths, dir, &name, style, &own_of(db, asset, None));
    link_at(db, paths, asset, folder, &rel, links)
}

/// A new file entry of the item at `rel`, which must be free.
pub(super) fn link_at(db: &mut Db, paths: &LibraryPaths, asset: AssetId, folder: Option<i64>, rel: &str, links: bool) -> DbResult<Placement> {
    let a = db.asset(asset)?.ok_or("no such item")?;
    let (root, src_rel, src) = if db.in_store(asset) {
        let have = db.placements_of(asset)?;
        let p = have.iter().find(|p| exists(&abs(paths, &p.rel_path))).ok_or("the item has no file in the library to link")?;
        (managed_root(db, paths)?, p.rel_path.clone(), abs(paths, &p.rel_path))
    } else {
        (a.root_id, a.rel_path.clone(), PathBuf::from(&a.abs_path))
    };
    let rel = rel.to_string();
    let op = begin_at(db, root, &src, "place", &src_rel, Some(&rel), Some(a.uid.clone()), &Detail { folder, ..Default::default() })?;
    let linked = match link_or_copy(&src, &abs(paths, &rel), links) {
        Ok(l) => l,
        Err(e) => return abandon(db, &op, e, &format!("adding {rel}")),
    };
    let id = db.tx(|db| {
        let id = db.add_placement(asset, live_folder(db, folder), &rel, linked)?;
        db.remove_file_op(op.id)?;
        Ok(id)
    })?;
    db.placement(id).ok_or_else(|| "placement vanished".into())
}

/// Remove one of an item's files. A library item keeps its other placements and its last one can't go this way; an item indexed in place keeps its location file.
pub fn unplace(db: &mut Db, paths: &LibraryPaths, placement: i64) -> DbResult<()> {
    let p = db.placement(placement).ok_or("no such placement")?;
    if db.in_store(p.asset_id) {
        let others: Vec<Placement> = db.placements_of(p.asset_id)?.into_iter().filter(|o| o.id != p.id).collect();
        if others.is_empty() {
            return Err("an item's last file can't be removed this way".into());
        }
        if !others.iter().any(|o| exists(&abs(paths, &o.rel_path))) {
            return Err(format!("{} is the item's only file left on disk, so it was kept", p.rel_path));
        }
    }
    let op = begin(db, paths, "unplace", &p.rel_path, None, Some(uid_of(db, p.asset_id)?), &Detail { placement: Some(p.id), ..Default::default() })?;
    match std::fs::remove_file(abs(paths, &p.rel_path)) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return abandon(db, &op, e, &format!("removing {}", p.rel_path)),
    }
    db.tx(|db| {
        db.remove_placement(p.id)?;
        db.remove_file_op(op.id)
    })?;
    prune_dirs(db, paths, parent_of(&p.rel_path));
    Ok(())
}

/// Remove every link of an item indexed in place, before its file comes into the store and gets placements of its own.
pub fn drop_links(db: &mut Db, paths: &LibraryPaths, asset: AssetId) -> DbResult<()> {
    for p in db.placements_of(asset)? {
        unplace(db, paths, p.id)?;
    }
    Ok(())
}

/// Move a placement to the directory `dir`, recorded as being in `folder`.
pub(super) fn relocate_to(db: &mut Db, paths: &LibraryPaths, placement: i64, folder: Option<i64>, dir: &str, name: Option<&str>, style: SuffixStyle) -> DbResult<Placement> {
    let p = db.placement(placement).ok_or("no such placement")?;
    if folder.is_some() && folder != p.folder_id && db.placements_of(p.asset_id)?.iter().any(|o| o.folder_id == folder) {
        return Err("the item already has a file in that folder".into());
    }
    let name = match name {
        Some(n) => n.to_string(),
        None => base_of(&p.rel_path).to_string(),
    };
    let wanted = join(dir, &clean_name(&name, false));
    // a change of case only keeps the placement's own path free
    let rel = if fold_path(&wanted) == fold_path(&p.rel_path) { wanted } else { free_for(db, paths, dir, &name, style, &own_of(db, p.asset_id, Some(p.id))) };
    move_entry(db, paths, &p, folder, &rel)
}

/// Move a placement to `rel`, which must be free, recorded as being in `folder`.
pub(super) fn move_entry(db: &mut Db, paths: &LibraryPaths, p: &Placement, folder: Option<i64>, rel: &str) -> DbResult<Placement> {
    let rel = rel.to_string();
    if rel == p.rel_path {
        if p.folder_id != folder {
            db.move_placement(p.id, folder, &rel)?;
        }
        return db.placement(p.id).ok_or_else(|| "placement vanished".into());
    }
    let op = begin(db, paths, "move", &p.rel_path, Some(&rel), Some(uid_of(db, p.asset_id)?), &Detail { placement: Some(p.id), folder })?;
    if let Err(e) = rename_no_clobber(&abs(paths, &p.rel_path), &abs(paths, &rel)) {
        return abandon(db, &op, e, &format!("moving {}", p.rel_path));
    }
    db.tx(|db| {
        db.move_placement(p.id, live_folder(db, folder), &rel)?;
        db.remove_file_op(op.id)
    })?;
    prune_dirs(db, paths, parent_of(&p.rel_path));
    db.placement(p.id).ok_or_else(|| "placement vanished".into())
}

/// Move the store directory `from` to `to` with everything below it, and every placement and folder directory recorded there.
pub fn move_dir(db: &mut Db, paths: &LibraryPaths, from: &str, to: &str) -> DbResult<()> {
    let op = begin(db, paths, "move_dir", from, Some(to), None, &Detail::default())?;
    let (a, b) = (abs(paths, from), abs(paths, to));
    if from != to && exists(&a) {
        if let Err(e) = rename_no_clobber(&a, &b) {
            return abandon(db, &op, e, &format!("moving the folder {from}"));
        }
    }
    finish_dir(db, &op, from, to)?;
    prune_dirs(db, paths, parent_of(from));
    Ok(())
}

pub(super) fn finish_dir(db: &mut Db, op: &FileOp, from: &str, to: &str) -> DbResult<()> {
    db.tx(|db| {
        db.move_placement_dir(from, to)?;
        db.move_folder_dirs(from, to)?;
        db.remove_file_op(op.id)
    })
}
