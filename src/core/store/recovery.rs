//! Journaled operations an interruption left, finished or undone at start.

use crate::core::db::{fold_path, Db, DbResult};
use crate::core::library::paths::LibraryPaths;
use crate::core::system::logging;
use crate::core::model::{AssetId, FileOp, Placement};
use crate::core::store::placing::finish_dir;
use crate::core::store::base_of;
use crate::core::store::disk::name_on_disk;
use crate::core::store::placing::live_folder;
use crate::core::store::placing::remove_unplaced;
use crate::core::store::disk::same_file;
use crate::core::store::disk::exists;
use crate::core::store::abs;
use crate::core::store::placing::detail_of;

const KINDS: &[&str] = &["place", "unplace", "move", "move_dir"];
const KNOWN_KINDS: &[&str] = &["place", "unplace", "move", "move_dir", "recycle", "replace", "item_move"];

/// Pending file operations of a kind this build doesn't know (written by a newer version).
pub fn unknown_pending(db: &Db) -> Vec<String> {
    let mut kinds: Vec<String> = db.file_ops(None, Some("pending"), 1000).unwrap_or_default().into_iter().map(|o| o.kind).filter(|k| !KNOWN_KINDS.contains(&k.as_str())).collect();
    kinds.sort();
    kinds.dedup();
    kinds
}

/// Finish or drop store operations cut short, and retried ones. Runs at open and with the file-op queue, on the thread that runs store operations. Returns the items whose files changed.
pub fn recover(db: &mut Db, paths: &LibraryPaths) -> Vec<AssetId> {
    let mut changed = crate::core::store::transfer::recover(db);
    let ops: Vec<FileOp> = db.file_ops(None, Some("pending"), 100_000).unwrap_or_default().into_iter().filter(|o| KINDS.contains(&o.kind.as_str())).collect();
    for op in ops {
        let asset = op.asset_uid.as_deref().and_then(|u| db.asset_by_uid(u));
        match recover_one(db, paths, &op) {
            Ok(()) => changed.extend(asset),
            Err(why) => {
                logging::error(&format!("store operation {} on {}: {why}", op.kind, op.rel_path));
                let _ = db.set_file_op_state(op.id, "failed", Some(&why));
            }
        }
    }
    changed
}

fn recover_one(db: &mut Db, paths: &LibraryPaths, op: &FileOp) -> DbResult<()> {
    let d = detail_of(op);
    let src = abs(paths, &op.rel_path);
    let dest_rel = op.dest.clone().unwrap_or_default();
    let dest = abs(paths, &dest_rel);
    let asset = op.asset_uid.as_deref().and_then(|u| db.asset_by_uid(u));
    match op.kind.as_str() {
        "place" => {
            let src = db.file_op_path(op).unwrap_or(src);
            let Some(asset) = asset else {
                // purged while the link was made: the link goes too, the item's own file goes to the Recycle Bin with the purge
                if exists(&dest) && db.placement_at(&dest_rel).is_none() && (!exists(&src) || same_file(&src, &dest)) {
                    remove_unplaced(db, paths, &dest_rel).map_err(|e| format!("removing {dest_rel}: {e}"))?;
                }
                return db.remove_file_op(op.id);
            };
            if !exists(&dest) || db.placement_at(&dest_rel).is_some() {
                return db.remove_file_op(op.id);
            }
            let linked = same_file(&src, &dest);
            if !linked && std::fs::metadata(&src).map(|m| m.len()).ok() != std::fs::metadata(&dest).map(|m| m.len()).ok() {
                return Err(format!("{dest_rel} isn't a copy of {}, so it was left alone", op.rel_path));
            }
            db.tx(|db| {
                db.add_placement(asset, live_folder(db, d.folder), &dest_rel, linked)?;
                db.remove_file_op(op.id)
            })
        }
        "unplace" => {
            let Some(p) = d.placement.and_then(|id| db.placement(id)) else { return db.remove_file_op(op.id) };
            if exists(&src) {
                let others: Vec<Placement> = db.placements_of(p.asset_id)?.into_iter().filter(|o| o.id != p.id).collect();
                if db.in_store(p.asset_id) && !others.iter().any(|o| exists(&abs(paths, &o.rel_path))) {
                    return Err(format!("{} is the item's only file left on disk, so it was kept", op.rel_path));
                }
                std::fs::remove_file(&src).map_err(|e| e.to_string())?;
            }
            db.tx(|db| {
                db.remove_placement(p.id)?;
                db.remove_file_op(op.id)
            })
        }
        "move" | "move_dir" => {
            let case_only = fold_path(&op.rel_path) == fold_path(&dest_rel);
            let done = if case_only {
                name_on_disk(&dest).as_deref() == Some(base_of(&dest_rel))
            } else {
                match (exists(&src), exists(&dest)) {
                    (false, true) => true,
                    (true, false) => false,
                    (false, false) if op.kind == "move_dir" => true,
                    (false, false) => return Err(format!("neither {} nor {dest_rel} is there", op.rel_path)),
                    (true, true) => return Err(format!("both {} and {dest_rel} exist, so neither was touched", op.rel_path)),
                }
            };
            if !done {
                return db.remove_file_op(op.id);
            }
            if op.kind == "move_dir" {
                return finish_dir(db, op, &op.rel_path, &dest_rel);
            }
            let Some(p) = d.placement.and_then(|id| db.placement(id)) else {
                // purged while the file moved: the purge's trip to the Recycle Bin follows the file
                return db.tx(|db| {
                    db.conn.execute("UPDATE file_ops SET rel_path = ?1 WHERE kind = 'recycle' AND state = 'pending' AND root_id = ?2 AND rel_path = ?3", rusqlite::params![dest_rel, op.root_id, op.rel_path]).map_err(|e| e.to_string())?;
                    db.remove_file_op(op.id)
                });
            };
            db.tx(|db| {
                db.move_placement(p.id, live_folder(db, d.folder), &dest_rel)?;
                db.remove_file_op(op.id)
            })
        }
        other => Err(format!("unknown store operation {other}")),
    }
}
