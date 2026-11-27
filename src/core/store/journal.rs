//! File work recorded in `file_ops` before it starts, so a crash midway can be finished or undone at the next open.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::db::{Db, DbResult};
use crate::core::system::filesystem;
use crate::core::system::logging;
use crate::core::model::{AssetId, FileOp};

/// Whether the file at `path` is still the one the op recorded. An op without size and time proves nothing, so it counts as changed.
pub fn unchanged(path: &Path, op: &FileOp) -> bool {
    match (filesystem::stat(path), op.size) {
        (Ok(st), Some(size)) => size == st.size as i64 && op.mtime.is_some() && op.mtime == st.mtime,
        _ => false,
    }
}

/// The new copy beside `dest`: only a plain name of Curio's own, since the database may have come from anyone.
pub fn tmp_path(dest: &Path, op: &FileOp) -> Option<PathBuf> {
    let t = op.tmp.as_deref()?;
    let mut parts = Path::new(t).components();
    let plain = matches!((parts.next(), parts.next()), (Some(std::path::Component::Normal(_)), None));
    (plain && t.ends_with(".replacing")).then(|| dest.with_file_name(t))
}

/// Send one purged copy to the Recycle Bin. `Ok` means the op is finished, including when there was nothing left to do.
fn recycle_one(db: &Db, op: &FileOp) -> Result<(), String> {
    let path = db.file_op_path(op).ok_or("its location is no longer part of the library")?;
    // added back since: the file belongs to that item now
    if db.asset_at(op.root_id, &op.rel_path).is_some() || db.placement_at(&op.rel_path).is_some() {
        return Ok(());
    }
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
        Ok(_) => {}
    }
    if !unchanged(&path, op) {
        return Err(format!("{} changed after it was deleted, so it was left in place", path.display()));
    }
    filesystem::to_recycle_bin(&path)
}

/// Work through up to `limit` pending recycle ops. Returns (finished, failed, more left).
pub fn run_recycles(db: &Db, limit: i64, stop: &AtomicBool) -> DbResult<(usize, usize, bool)> {
    let ops = db.file_ops(Some("recycle"), Some("pending"), limit)?;
    let (mut done, mut failed) = (0, 0);
    for op in &ops {
        if stop.load(Ordering::Relaxed) {
            return Ok((done, failed, true));
        }
        match recycle_one(db, op) {
            Ok(()) => {
                db.remove_file_op(op.id)?;
                done += 1;
            }
            Err(why) => {
                logging::warn(&format!("emptying the trash: {}: {why}", op.rel_path));
                db.set_file_op_state(op.id, "failed", Some(&why))?;
                failed += 1;
            }
        }
    }
    Ok((done, failed, ops.len() as i64 == limit))
}

/// Finish or undo replaces cut short by a crash. Runs at open, before any worker can start a new one. Returns the items whose file changed.
pub fn recover_replaces(db: &mut Db) -> Vec<AssetId> {
    let mut changed = vec![];
    for op in db.file_ops(Some("replace"), None, 10_000).unwrap_or_default() {
        if op.state == "failed" {
            continue;
        }
        match recover_replace(db, &op) {
            Ok(id) => changed.extend(id),
            Err(why) => {
                logging::error(&format!("replacing {}: {why}", op.rel_path));
                let _ = db.set_file_op_state(op.id, "failed", Some(&why));
            }
        }
    }
    changed
}

fn recover_replace(db: &mut Db, op: &FileOp) -> Result<Option<AssetId>, String> {
    let dest = db.file_op_path(op).ok_or("its location is no longer part of the library")?;
    let tmp = tmp_path(&dest, op).ok_or("no new copy was recorded")?;
    // the old file is only touched once the new copy is complete
    if op.state != "staged" {
        let _ = std::fs::remove_file(&tmp);
        db.remove_file_op(op.id)?;
        logging::info(&format!("replacing {} was interrupted before the new file was ready; the old file stays", dest.display()));
        return Ok(None);
    }
    if tmp.exists() {
        if dest.exists() {
            if !unchanged(&dest, op) {
                return Err(format!("{} changed since the replace began; the new copy was kept as {}", dest.display(), tmp.display()));
            }
            filesystem::to_recycle_bin(&dest)?;
        }
        std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
    } else if !dest.exists() {
        return Err("neither the old nor the new file is there".into());
    }
    logging::info(&format!("finished an interrupted replace of {}", dest.display()));
    finish_replace(db, op, &dest)
}

/// Replace an item's file with `src` on the calling thread, outside any transaction, through the same journal steps `recover_replaces` completes: a copy beside it, `staged`, the old file to the Recycle Bin, the copy renamed into place, the row updated.
pub fn replace_with(db: &mut Db, id: AssetId, src: &Path) -> DbResult<()> {
    let a = db.asset(id)?.ok_or("no such item")?;
    db.require_writable(a.root_id)?;
    let dest = PathBuf::from(&a.abs_path);
    let tmp_name = format!("{}.{}.replacing", filesystem::stem_of(&dest), fastrand::u32(..));
    let tmp = dest.with_file_name(&tmp_name);
    let old = filesystem::stat(&dest).ok();
    let mut op = FileOp {
        kind: "replace".into(),
        root_id: a.root_id,
        rel_path: a.rel_path.clone(),
        tmp: Some(tmp_name),
        asset_uid: Some(a.uid.clone()),
        size: old.as_ref().map(|s| s.size as i64),
        mtime: old.as_ref().and_then(|s| s.mtime),
        state: "copying".into(),
        ..Default::default()
    };
    op.id = db.add_file_op(&op)?;
    let abandon = |db: &Db, err: String| -> DbResult<()> {
        let _ = std::fs::remove_file(&tmp);
        db.remove_file_op(op.id)?;
        Err(err)
    };
    if let Err(e) = std::fs::copy(src, &tmp).and_then(|_| std::fs::OpenOptions::new().write(true).open(&tmp)?.sync_all()) {
        return abandon(db, e.to_string());
    }
    // from here on recovery finishes the swap rather than dropping the new copy
    db.set_file_op_state(op.id, "staged", None)?;
    if dest.exists() {
        if !unchanged(&dest, &op) {
            return abandon(db, "the file changed while it was being replaced; nothing was replaced".into());
        }
        if let Err(e) = filesystem::to_recycle_bin(&dest) {
            return abandon(db, e);
        }
    }
    std::fs::rename(&tmp, &dest).map_err(|e| format!("the new file is ready as {} but couldn't take the old one's name: {e}", tmp.display()))?;
    finish_replace(db, &op, &dest)?;
    Ok(())
}

/// Point the item at its new file and clear the op in one transaction.
pub fn finish_replace(db: &mut Db, op: &FileOp, dest: &Path) -> DbResult<Option<AssetId>> {
    let st = filesystem::stat(dest).map_err(|e| e.to_string())?;
    let id = op.asset_uid.as_deref().and_then(|u| db.asset_by_uid(u));
    db.tx(|db| {
        if let Some(id) = id {
            db.file_rewritten(id, None, st.size as i64, st.mtime)?;
        }
        db.remove_file_op(op.id)
    })?;
    Ok(id)
}
