//! The store kept in step with the index: what changed in the library made on disk.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use crate::core::db::{fold_path, Db, DbResult};
use crate::core::system::filesystem;
use crate::core::library::paths::LibraryPaths;
use crate::core::system::logging;
use crate::core::model::{AssetId, Availability, Placement};
use crate::core::store::sync_folders;
use crate::core::store::placing::link_at;
use crate::core::store::placing::move_entry;
use crate::core::store::own_of;
use crate::core::store::free_for;
use crate::core::store::placing::name_of_asset;
use crate::core::store::is_free_for;
use crate::core::store::Own;
use crate::core::store::placing::live_folder;
use crate::core::store::join;
use crate::core::store::member_name;
use crate::core::store::placing::place_in;
use crate::core::store::placing::place;
use crate::core::store::base_of;
use crate::core::store::is_variant;
use crate::core::store::folder_dir;
use crate::core::store::clean_name;
use crate::core::store::disk::exists;
use crate::core::store::placing::relocate_to;
use crate::core::store::TRASH_DIR;
use crate::core::store::parent_of;
use crate::core::store::placing::unplace;
use crate::core::store::disk::link_or_copy;
use crate::core::store::disk::same_file;
use crate::core::store::abs;
use crate::core::store::SuffixStyle;

/// Bring one item's files in the store in line with its folders, or with the trash. An item indexed in place first has its location file follow its name, where its location organizes files. Returns whether anything moved.
pub fn sync_item(db: &mut Db, paths: &LibraryPaths, asset: AssetId, links: bool, style: SuffixStyle) -> DbResult<bool> {
    let row: Option<(bool, String, String, bool)> = db
        .conn
        .query_row("SELECT a.deleted, a.name, a.rel_path, r.kind = 'managed' FROM assets a JOIN roots r ON r.id = a.root_id WHERE a.id = ?", [asset], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .ok();
    let Some((deleted, name, rel, managed)) = row else { return Ok(false) };
    let part: Option<(AssetId, Option<String>)> = db.conn.query_row("SELECT part_of, part_suffix FROM assets WHERE id = ? AND part_of IS NOT NULL", [asset], |r| Ok((r.get(0)?, r.get(1)?))).ok();
    if let Some((item, suffix)) = part {
        let same_root: bool = db.conn.query_row("SELECT (SELECT root_id FROM assets WHERE id = ?1) = (SELECT root_id FROM assets WHERE id = ?2)", [asset, item], |r| r.get(0)).unwrap_or(false);
        // a file left behind when its item moved elsewhere stays as it is until it joins it
        if !same_root {
            return Ok(false);
        }
        return follow_item(db, paths, asset, item, suffix.as_deref(), managed.then_some(rel.as_str()), links, style);
    }
    if managed {
        return follow_folders(db, paths, asset, deleted, name, Some(&rel), links, style);
    }
    let followed = crate::core::store::transfer::follow_name(db, asset, style);
    let synced = follow_folders(db, paths, asset, deleted, name, None, links, style)?;
    Ok(followed? || synced)
}

/// Whether a link of an item indexed in place no longer has its location file's data: a hard link whose location file was replaced, or a copy of another size or older than it.
fn stale(paths: &LibraryPaths, src: &Path, p: &Placement) -> bool {
    let at = abs(paths, &p.rel_path);
    if same_file(src, &at) {
        return false;
    }
    let (Ok(s), Ok(d)) = (std::fs::metadata(src), std::fs::metadata(&at)) else { return false };
    p.linked || s.len() != d.len() || d.modified().ok() < s.modified().ok()
}

/// Make a stale link an entry of the location file again. A link newer than the location file was saved through the store, so its content first replaces the location file (whose old version goes to the Recycle Bin); the link follows on the next pass. A location that can't be written keeps its file, and the edited link goes to the Recycle Bin instead.
fn renew(db: &mut Db, paths: &LibraryPaths, asset: AssetId, src: &Path, p: &Placement, links: bool) -> DbResult<()> {
    let at = abs(paths, &p.rel_path);
    let edited = match (std::fs::metadata(src).and_then(|m| m.modified()), std::fs::metadata(&at).and_then(|m| m.modified())) {
        (Ok(s), Ok(d)) => d > s,
        _ => false,
    };
    if edited {
        match crate::core::store::journal::replace_with(db, asset, &at) {
            Ok(()) => {
                logging::info(&format!("{} was saved in the library's store; its location file now has the change", p.rel_path));
                return Ok(());
            }
            Err(e) => logging::warn(&format!("{} was saved in the library's store, but its location file can't take the change ({e}); the edited file goes to the Recycle Bin", p.rel_path)),
        }
    }
    let tmp = PathBuf::from(format!("{}.relinking", at.display()));
    let _ = std::fs::remove_file(&tmp);
    let linked = link_or_copy(src, &tmp, links).map_err(|e| format!("renewing {}: {e}", p.rel_path))?;
    let swap = if edited { filesystem::to_recycle_bin(&at) } else { Ok(()) }.and_then(|_| std::fs::rename(&tmp, &at).map_err(|e| e.to_string()));
    if let Err(e) = swap {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("renewing {}: {e}", p.rel_path));
    }
    if linked != p.linked {
        db.set_placement_linked(p.id, linked)?;
    }
    Ok(())
}

/// Placements follow the item's folders and the trash. `own` is a library item's own path; such an item keeps one file in the trash when trashed whole, and an item with no file on disk is left as it is. An item indexed in place has a link per folder and per folder entry in the trash, none otherwise, and waits while its location file is missing. Placements whose file is gone are forgotten.
#[allow(clippy::too_many_arguments)]
fn follow_folders(db: &mut Db, paths: &LibraryPaths, asset: AssetId, deleted: bool, name: String, own: Option<&str>, links: bool, style: SuffixStyle) -> DbResult<bool> {
    if let Some(rel) = own {
        if db.placements_of(asset)?.is_empty() {
            db.add_placement(asset, None, rel, true)?;
        }
    }
    let (mut here, mut gone) = (vec![], vec![]);
    for p in db.placements_of(asset)? {
        // only a file that is certainly gone loses its row; an unreadable one (a drive or permission error) stays for the next sync
        match std::fs::symlink_metadata(abs(paths, &p.rel_path)) {
            Ok(_) => here.push(p),
            Err(e) if e.kind() == io::ErrorKind::NotFound => gone.push(p),
            Err(e) => return Err(format!("{} can't be checked: {e}", p.rel_path)),
        }
    }
    if own.is_some() && here.is_empty() {
        return Ok(false);
    }
    for p in &gone {
        db.remove_placement(p.id)?;
    }
    let mut moved = !gone.is_empty();
    if deleted && own.is_none() {
        for p in &here {
            unplace(db, paths, p.id)?;
        }
        return Ok(moved || !here.is_empty());
    }
    if deleted {
        let keep = here.iter().find(|p| p.folder_id.is_none() && parent_of(&p.rel_path) == TRASH_DIR).unwrap_or(&here[0]).clone();
        if keep.folder_id.is_some() || parent_of(&keep.rel_path) != TRASH_DIR {
            relocate_to(db, paths, keep.id, None, TRASH_DIR, Some(&name), style)?;
            moved = true;
        }
        for p in here.iter().filter(|p| p.id != keep.id) {
            unplace(db, paths, p.id)?;
            moved = true;
        }
        return Ok(moved);
    }
    let mut want: Vec<Option<i64>> = {
        let mut st = db.conn.prepare_cached("SELECT folder_id FROM asset_folders WHERE asset_id = ? ORDER BY folder_id").map_err(|e| e.to_string())?;
        let q = st.query_map([asset], |r| r.get::<_, i64>(0)).map_err(|e| e.to_string())?;
        q.filter_map(|r| r.ok()).map(Some).collect()
    };
    if want.is_empty() && own.is_some() {
        want.push(None);
    }
    if own.is_none() {
        let a = db.asset(asset)?.ok_or("no such item")?;
        let src = PathBuf::from(&a.abs_path);
        if a.status != Availability::Ok || !exists(&src) {
            return Ok(moved);
        }
        for p in &here {
            if stale(paths, &src, p) {
                renew(db, paths, asset, &src, p, links)?;
                moved = true;
            }
        }
    }
    // folder entries in the trash keep a link in the trash
    let binned: Vec<i64> = {
        let mut st = db.conn.prepare_cached("SELECT folder_id FROM trashed_entries WHERE asset_id = ? ORDER BY folder_id").map_err(|e| e.to_string())?;
        let q = st.query_map([asset], |r| r.get::<_, i64>(0)).map_err(|e| e.to_string())?;
        q.filter_map(|r| r.ok()).collect()
    };
    let clean = clean_name(&name, false);
    let mut done: HashSet<Option<i64>> = HashSet::new();
    let mut binned_done: HashSet<i64> = HashSet::new();
    let mut spare: std::collections::VecDeque<Placement> = Default::default();
    for p in here {
        if let Some(f) = p.folder_id.filter(|f| binned.contains(f) && !binned_done.contains(f) && parent_of(&p.rel_path) == TRASH_DIR) {
            binned_done.insert(f);
            continue;
        }
        if want.contains(&p.folder_id) && !done.contains(&p.folder_id) {
            let dir = folder_dir(db, paths, p.folder_id)?;
            if parent_of(&p.rel_path) != dir || !is_variant(base_of(&p.rel_path), &clean) {
                relocate_to(db, paths, p.id, p.folder_id, &dir, Some(&name), style)?;
                moved = true;
            }
            done.insert(p.folder_id);
        } else {
            spare.push_back(p);
        }
    }
    for f in want.iter().filter(|f| !done.contains(*f)) {
        // an entry the item no longer needs moves to the new folder rather than being copied
        match spare.pop_front() {
            Some(p) => {
                let dir = folder_dir(db, paths, *f)?;
                relocate_to(db, paths, p.id, *f, &dir, Some(&name), style)?;
            }
            None => {
                place(db, paths, asset, *f, links, style)?;
            }
        }
        moved = true;
    }
    for f in binned.iter().filter(|f| !binned_done.contains(f)) {
        let own = spare.iter().position(|p| p.folder_id == Some(*f));
        match own.or(if spare.is_empty() { None } else { Some(0) }).and_then(|i| spare.remove(i)) {
            Some(p) => {
                relocate_to(db, paths, p.id, Some(*f), TRASH_DIR, Some(&name), style)?;
            }
            None => {
                place_in(db, paths, asset, Some(*f), TRASH_DIR, links, style)?;
            }
        }
        moved = true;
    }
    for p in spare {
        unplace(db, paths, p.id)?;
        moved = true;
    }
    Ok(moved)
}

/// Where a grouped file's entries belong: one beside each entry of its item, named after it (its item's stem and the file's suffix, or its own name without one).
fn member_entries(db: &Db, member_name_now: &str, item: AssetId, suffix: Option<&str>) -> DbResult<Vec<(Option<i64>, String)>> {
    Ok(db
        .placements_of(item)?
        .into_iter()
        .map(|ip| {
            let name = match suffix {
                Some(s) => member_name(base_of(&ip.rel_path), s),
                None => clean_name(member_name_now, false),
            };
            (ip.folder_id, join(parent_of(&ip.rel_path), &name))
        })
        .collect())
}

/// A grouped file's entries follow its item's: one beside each, named after it. A name taken by a file outside the group gives the item's entry there a free number, and this file follows on the next pass. A copied file keeps its own file until its item has an entry; one indexed in place waits while its location file is missing.
#[allow(clippy::too_many_arguments)]
fn follow_item(db: &mut Db, paths: &LibraryPaths, member: AssetId, item: AssetId, suffix: Option<&str>, own: Option<&str>, links: bool, style: SuffixStyle) -> DbResult<bool> {
    let m = db.asset(member)?.ok_or("no such item")?;
    if let Some(rel) = own {
        if db.placements_of(member)?.is_empty() {
            db.add_placement(member, None, rel, true)?;
        }
    }
    let (mut here, mut gone) = (vec![], vec![]);
    for p in db.placements_of(member)? {
        match std::fs::symlink_metadata(abs(paths, &p.rel_path)) {
            Ok(_) => here.push(p),
            Err(e) if e.kind() == io::ErrorKind::NotFound => gone.push(p),
            Err(e) => return Err(format!("{} can't be checked: {e}", p.rel_path)),
        }
    }
    if own.is_some() && here.is_empty() {
        return Ok(false);
    }
    for p in &gone {
        db.remove_placement(p.id)?;
    }
    let mut moved = !gone.is_empty();
    if own.is_none() {
        let src = PathBuf::from(&m.abs_path);
        if m.status != Availability::Ok || !exists(&src) {
            return Ok(moved);
        }
        for p in &here {
            if stale(paths, &src, p) {
                renew(db, paths, member, &src, p, links)?;
                moved = true;
            }
        }
    }
    let want = member_entries(db, &m.name, item, suffix)?;
    if want.is_empty() && own.is_some() {
        return Ok(moved);
    }
    let mut done = vec![false; want.len()];
    let mut spare: std::collections::VecDeque<Placement> = Default::default();
    for p in here {
        match want.iter().enumerate().position(|(i, (_, r))| !done[i] && fold_path(r) == fold_path(&p.rel_path)) {
            Some(i) => {
                done[i] = true;
                if p.folder_id != want[i].0 {
                    db.move_placement(p.id, live_folder(db, want[i].0), &p.rel_path)?;
                    moved = true;
                }
            }
            None => spare.push_back(p),
        }
    }
    let mine = Own { members: vec![member], ..Default::default() };
    for (i, (folder, rel)) in want.iter().enumerate() {
        if done[i] {
            continue;
        }
        if !is_free_for(db, paths, rel, &mine) {
            let Some(ip) = db.placements_of(item)?.into_iter().find(|ip| ip.folder_id == *folder && parent_of(&ip.rel_path) == parent_of(rel)) else { continue };
            let item_name = name_of_asset(db, item)?;
            let to = free_for(db, paths, parent_of(&ip.rel_path), &item_name, style, &own_of(db, item, Some(ip.id)));
            if to != ip.rel_path {
                move_entry(db, paths, &ip, ip.folder_id, &to)?;
                db.mark_store_dirty(&[member])?;
                moved = true;
            }
            continue;
        }
        // the entry of that folder moves first, so no two entries share a folder on the way
        let pick = spare.iter().position(|p| p.folder_id == *folder).or_else(|| spare.iter().position(|p| !want.iter().enumerate().any(|(j, (f, _))| !done[j] && j != i && *f == p.folder_id)));
        match pick.and_then(|k| spare.remove(k)) {
            Some(p) => {
                move_entry(db, paths, &p, *folder, rel)?;
            }
            None => {
                link_at(db, paths, member, *folder, rel, links)?;
            }
        }
        done[i] = true;
        moved = true;
    }
    let mut kept = done.iter().filter(|d| **d).count();
    for p in spare {
        // a deferred target leaves the member's last file in place until the next pass moves it
        if own.is_some() && kept == 0 {
            kept = 1;
            continue;
        }
        unplace(db, paths, p.id)?;
        moved = true;
    }
    if own.is_some() {
        let rel: String = db.conn.query_row("SELECT rel_path FROM assets WHERE id = ?", [member], |r| r.get(0)).map_err(|e| e.to_string())?;
        if base_of(&rel) != m.name {
            db.set_file_name(member, base_of(&rel))?;
        }
    }
    Ok(moved)
}

/// Queue the grouped files whose entries don't match their items': the check at open, which also finishes a pass cut short between an item's move and its files'.
pub fn mark_members_inconsistent(db: &Db) -> DbResult<usize> {
    let rows: Vec<(AssetId, AssetId, Option<String>, String)> = {
        let mut st = db
            .conn
            .prepare("SELECT m.id, m.part_of, m.part_suffix, m.name FROM assets m JOIN assets i ON i.id = m.part_of WHERE m.root_id = i.root_id AND m.status = 0")
            .map_err(|e| e.to_string())?;
        let q = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).map_err(|e| e.to_string())?;
        q.filter_map(|r| r.ok()).collect()
    };
    let mut off = vec![];
    for (member, item, suffix, name) in rows {
        let mut want: Vec<(Option<i64>, String)> = member_entries(db, &name, item, suffix.as_deref())?.into_iter().map(|(f, r)| (f, fold_path(&r))).collect();
        let mut have: Vec<(Option<i64>, String)> = db.placements_of(member)?.into_iter().map(|p| (p.folder_id, fold_path(&p.rel_path))).collect();
        want.sort();
        have.sort();
        if want != have && !(want.is_empty() && db.in_store(member)) {
            off.push(member);
        }
    }
    db.mark_store_dirty(&off)?;
    Ok(off.len())
}

#[derive(Debug, Default)]
pub struct SyncResult {
    pub moved: Vec<AssetId>,
    /// What failed, by the queued entry it stopped (`0` for the folder tree). That entry is off the queue until queued again.
    pub problems: Vec<(AssetId, String)>,
    /// More items are queued than `limit` allowed.
    pub more: bool,
}

/// Work through up to `limit` queued items (and the folder tree first, when it changed). Each is taken off the queue before its files move, so a change made meanwhile queues it again.
pub fn sync(db: &mut Db, paths: &LibraryPaths, links: bool, style: SuffixStyle, limit: i64) -> DbResult<SyncResult> {
    let mut out = SyncResult::default();
    let unknown = crate::core::store::recovery::unknown_pending(db);
    if !unknown.is_empty() {
        logging::warn(&format!("store sync paused: pending file operations from a newer Curio ({})", unknown.join(", ")));
        return Ok(out);
    }
    let (folders, ids) = db.store_dirty(limit)?;
    if folders {
        db.clear_store_dirty(0)?;
        out.problems.extend(sync_folders(db, paths)?.into_iter().map(|p| (0, p)));
    }
    for id in &ids {
        db.clear_store_dirty(*id)?;
        match sync_item(db, paths, *id, links, style) {
            Ok(true) => {
                // the files grouped into it follow its entries
                let members: Vec<AssetId> = db.members_of(*id)?.into_iter().map(|p| p.id).collect();
                db.mark_store_dirty(&members)?;
                out.moved.push(*id);
            }
            Ok(false) => {}
            // purged by the writer while this pass moved its files
            Err(_) if db.asset(*id).ok().flatten().is_none() => {}
            Err(e) => {
                let name = name_of_asset(db, *id).unwrap_or_else(|_| format!("item {id}"));
                out.problems.push((*id, format!("{name}: {e}")));
            }
        }
    }
    out.more = ids.len() as i64 == limit;
    Ok(out)
}
