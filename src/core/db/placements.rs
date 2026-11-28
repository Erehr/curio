//! Where items are placed: their folders' links in the managed store.

use crate::core::db::IN_STORE;
use crate::core::db::fold_path;
use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use rusqlite::{params, OptionalExtension};
use crate::core::system::filesystem;
use crate::core::model::*;

fn placement_row(r: &rusqlite::Row) -> rusqlite::Result<Placement> {
    Ok(Placement { id: r.get(0)?, asset_id: r.get(1)?, folder_id: r.get(2)?, rel_path: r.get(3)?, linked: r.get::<_, i64>(4)? != 0 })
}

impl Db {
    /// Up to `limit` managed items queued for the store to follow, and whether the folder tree changed.
    pub fn store_dirty(&self, limit: i64) -> DbResult<(bool, Vec<AssetId>)> {
        let folders = self.conn.query_row("SELECT EXISTS(SELECT 1 FROM store_dirty WHERE asset_id = 0)", [], |r| r.get::<_, bool>(0)).map_err(e)?;
        // items before the files grouped into them, whose names come from their items' entries
        let mut st = self.conn.prepare_cached("SELECT d.asset_id FROM store_dirty d LEFT JOIN assets a ON a.id = d.asset_id WHERE d.asset_id > 0 ORDER BY a.part_of IS NOT NULL, d.asset_id LIMIT ?").map_err(e)?;
        let rows = st.query_map([limit], |r| r.get(0)).map_err(e)?;
        Ok((folders, rows.filter_map(|r| r.ok()).collect()))
    }

    pub fn store_dirty_any(&self) -> bool {
        self.conn.query_row("SELECT EXISTS(SELECT 1 FROM store_dirty)", [], |r| r.get(0)).unwrap_or(false)
    }

    pub fn mark_store_dirty(&self, ids: &[AssetId]) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("INSERT OR IGNORE INTO store_dirty VALUES (?)").map_err(e)?;
        for id in ids {
            st.execute([id]).map_err(e)?;
        }
        Ok(())
    }

    pub fn clear_store_dirty(&self, id: AssetId) -> DbResult<()> {
        self.conn.execute("DELETE FROM store_dirty WHERE asset_id = ?", [id]).map_err(e)?;
        Ok(())
    }

    /// Queue every item whose placements don't match its folders, its folders' directories or the trash, and the folder tree when a folder has no directory yet. An item indexed in place has a placement per folder and none outside them; a missing one is left until its file is back.
    pub fn mark_store_inconsistent(&self) -> DbResult<usize> {
        const OFF: &str = "EXISTS (SELECT 1 FROM asset_folders x WHERE x.asset_id = a.id AND NOT EXISTS (SELECT 1 FROM placements p WHERE p.asset_id = a.id AND p.folder_id = x.folder_id))
            OR EXISTS (SELECT 1 FROM trashed_entries t WHERE t.asset_id = a.id AND NOT EXISTS (SELECT 1 FROM placements p WHERE p.asset_id = a.id AND p.folder_id = t.folder_id))
            OR EXISTS (SELECT 1 FROM placements p WHERE p.asset_id = a.id AND p.folder_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM asset_folders x WHERE x.asset_id = a.id AND x.folder_id = p.folder_id)
                AND NOT EXISTS (SELECT 1 FROM trashed_entries t WHERE t.asset_id = a.id AND t.folder_id = p.folder_id))
            OR EXISTS (SELECT 1 FROM placements p JOIN trashed_entries t ON t.asset_id = p.asset_id AND t.folder_id = p.folder_id WHERE p.asset_id = a.id
                AND (substr(p.rel_key, 1, 7) <> '.trash/' OR instr(substr(p.rel_key, 8), '/') > 0))
            OR EXISTS (SELECT 1 FROM placements p WHERE p.asset_id = a.id AND NOT name_fits(p.rel_path, a.name))
            OR EXISTS (SELECT 1 FROM placements p JOIN folders f ON f.id = p.folder_id JOIN asset_folders x ON x.asset_id = p.asset_id AND x.folder_id = p.folder_id WHERE p.asset_id = a.id AND (f.dir IS NULL
                OR substr(p.rel_key, 1, length(f.dir) + 1) <> fold_path(f.dir) || '/' OR instr(substr(p.rel_key, length(f.dir) + 2), '/') > 0))";
        let n = self
            .conn
            .execute(
                &format!(
                    "INSERT OR IGNORE INTO store_dirty
                     SELECT a.id FROM assets a JOIN roots r ON r.id = a.root_id WHERE r.kind = 'managed' AND a.part_of IS NULL AND (
                       (a.deleted = 1 AND ((SELECT count(*) FROM placements p WHERE p.asset_id = a.id) <> 1
                           OR EXISTS (SELECT 1 FROM placements p WHERE p.asset_id = a.id AND (p.folder_id IS NOT NULL OR substr(p.rel_key, 1, 7) <> '.trash/' OR instr(substr(p.rel_key, 8), '/') > 0))))
                       OR (a.deleted = 0 AND ({OFF}
                           OR EXISTS (SELECT 1 FROM placements p WHERE p.asset_id = a.id AND p.folder_id IS NULL) = EXISTS (SELECT 1 FROM asset_folders x WHERE x.asset_id = a.id)
                           OR EXISTS (SELECT 1 FROM placements p WHERE p.asset_id = a.id AND p.folder_id IS NULL AND instr(p.rel_path, '/') > 0))))"
                ),
                [],
            )
            .map_err(e)?;
        let indexed = self
            .conn
            .execute(
                &format!(
                    "INSERT OR IGNORE INTO store_dirty
                     SELECT a.id FROM assets a JOIN roots r ON r.id = a.root_id WHERE r.kind <> 'managed' AND a.part_of IS NULL AND (
                       (a.deleted = 1 AND EXISTS (SELECT 1 FROM placements p WHERE p.asset_id = a.id))
                       OR (a.deleted = 0 AND a.status = 0 AND ({OFF} OR EXISTS (SELECT 1 FROM placements p WHERE p.asset_id = a.id AND p.folder_id IS NULL))))"
                ),
                [],
            )
            .map_err(e)?;
        self.conn.execute("INSERT OR IGNORE INTO store_dirty SELECT 0 WHERE EXISTS (SELECT 1 FROM folders WHERE dir IS NULL)", []).map_err(e)?;
        let organized = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO store_dirty SELECT a.id FROM assets a JOIN roots r ON r.id = a.root_id
                 WHERE r.kind = 'linked' AND r.access = 'organize' AND a.deleted = 0 AND a.part_of IS NULL AND NOT name_fits(a.rel_path, a.name)",
                [],
            )
            .map_err(e)?;
        Ok(n + indexed + organized)
    }

    /// Rewrite the directory of every folder at or under `from` to sit at `to`.
    pub fn move_folder_dirs(&self, from: &str, to: &str) -> DbResult<()> {
        let rows: Vec<(i64, String)> = {
            let mut st = self.conn.prepare("SELECT id, dir FROM folders WHERE dir IS NOT NULL").map_err(e)?;
            let q = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
            q.filter_map(|r| r.ok()).collect()
        };
        let (key, key_slash) = (fold_path(from), fold_path(&format!("{from}/")));
        for (id, dir) in rows {
            let k = fold_path(&dir);
            let new = if k == key {
                to.to_string()
            } else if k.starts_with(&key_slash) {
                format!("{to}/{}", dir.chars().skip(from.chars().count() + 1).collect::<String>())
            } else {
                continue;
            };
            self.conn.execute("UPDATE folders SET dir = ? WHERE id = ?", params![new, id]).map_err(e)?;
        }
        Ok(())
    }

    pub fn placements_of(&self, asset: AssetId) -> DbResult<Vec<Placement>> {
        let mut st = self.conn.prepare_cached("SELECT id, asset_id, folder_id, rel_path, linked FROM placements WHERE asset_id = ? ORDER BY id").map_err(e)?;
        let rows = st.query_map([asset], placement_row).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn placements(&self) -> DbResult<Vec<Placement>> {
        let mut st = self.conn.prepare("SELECT id, asset_id, folder_id, rel_path, linked FROM placements").map_err(e)?;
        let rows = st.query_map([], placement_row).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn placement(&self, id: i64) -> Option<Placement> {
        self.conn.query_row("SELECT id, asset_id, folder_id, rel_path, linked FROM placements WHERE id = ?", [id], placement_row).optional().ok().flatten()
    }

    /// The placement whose path folds to the same key, whatever its case.
    pub fn placement_at(&self, rel_path: &str) -> Option<Placement> {
        self.conn
            .query_row("SELECT id, asset_id, folder_id, rel_path, linked FROM placements WHERE rel_key = ?", [fold_path(rel_path)], placement_row)
            .optional()
            .ok()
            .flatten()
    }

    /// The placement at `rel_path` when its item is listed in `folder` (`None` = in no folder) and not in the trash. A file still waiting for the store to move it after its item left is not one.
    pub fn member_placement_at(&self, rel_path: &str, folder: Option<i64>) -> Option<Placement> {
        let p = self.placement_at(rel_path)?;
        let listed: bool = self
            .conn
            .query_row(
                "SELECT a.deleted = 0 AND CASE WHEN ?2 IS NULL THEN NOT EXISTS (SELECT 1 FROM asset_folders x WHERE x.asset_id = a.id) ELSE EXISTS (SELECT 1 FROM asset_folders x WHERE x.asset_id = a.id AND x.folder_id = ?2) END FROM assets a WHERE a.id = ?1",
                params![p.asset_id, folder],
                |r| r.get(0),
            )
            .ok()?;
        listed.then_some(p)
    }

    pub fn add_placement(&self, asset: AssetId, folder: Option<i64>, rel_path: &str, linked: bool) -> DbResult<i64> {
        self.conn
            .execute(
                "INSERT INTO placements(asset_id, folder_id, rel_path, rel_key, linked) VALUES(?, ?, ?, ?, ?)",
                params![asset, folder, rel_path, fold_path(rel_path), linked as i64],
            )
            .map_err(e)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Whether the item's own file is in the library's store, rather than in a location.
    pub fn in_store(&self, asset: AssetId) -> bool {
        self.conn
            .query_row("SELECT EXISTS(SELECT 1 FROM assets a JOIN roots r ON r.id = a.root_id WHERE a.id = ? AND r.kind = 'managed')", [asset], |r| r.get(0))
            .unwrap_or(false)
    }

    /// Point a placement at a new path and folder. A library item's own path follows when it was this placement's.
    pub fn move_placement(&self, id: i64, folder: Option<i64>, rel_path: &str) -> DbResult<()> {
        let p = self.placement(id).ok_or("no such placement")?;
        self.conn
            .execute("UPDATE placements SET folder_id = ?, rel_path = ?, rel_key = ? WHERE id = ?", params![folder, rel_path, fold_path(rel_path), id])
            .map_err(e)?;
        self.conn.execute(&format!("UPDATE assets SET rel_path = ?2 WHERE id = ?1 AND rel_path = ?3 AND {IN_STORE}"), params![p.asset_id, rel_path, p.rel_path]).map_err(e)?;
        Ok(())
    }

    pub fn set_placement_linked(&self, id: i64, linked: bool) -> DbResult<()> {
        self.conn.execute("UPDATE placements SET linked = ? WHERE id = ?", params![linked as i64, id]).map_err(e)?;
        Ok(())
    }

    /// Forget a placement. When a library item's own path was this one, it moves to another placement; a library item's last placement can't be removed. An item indexed in place can lose them all.
    pub fn remove_placement(&self, id: i64) -> DbResult<()> {
        let p = self.placement(id).ok_or("no such placement")?;
        let other = self.placements_of(p.asset_id)?.into_iter().find(|o| o.id != id);
        if other.is_none() && self.in_store(p.asset_id) {
            return Err("an item's last file can't be removed this way".into());
        }
        self.conn.execute("DELETE FROM placements WHERE id = ?", [id]).map_err(e)?;
        if let Some(other) = other {
            self.conn.execute(&format!("UPDATE assets SET rel_path = ?2 WHERE id = ?1 AND rel_path = ?3 AND {IN_STORE}"), params![p.asset_id, other.rel_path, p.rel_path]).map_err(e)?;
        }
        Ok(())
    }

    /// Placements matching any path in `rels` or sitting underneath any of them (case-folded via `rel_key` index).
    pub fn placements_matching(&self, rels: &[String]) -> DbResult<Vec<Placement>> {
        let mut by_id: HashMap<i64, Placement> = HashMap::new();
        let mut exact = self.conn.prepare_cached("SELECT id, asset_id, folder_id, rel_path, linked FROM placements WHERE rel_key = ?").map_err(e)?;
        let mut under = self.conn.prepare_cached("SELECT id, asset_id, folder_id, rel_path, linked FROM placements WHERE rel_key >= ?1 AND rel_key < ?2").map_err(e)?;
        for r in rels {
            let k = fold_path(r);
            for p in exact.query_map([&k], placement_row).map_err(e)?.filter_map(|x| x.ok()) {
                by_id.insert(p.id, p);
            }
            let lo = format!("{k}/");
            let hi = format!("{k}0");
            for p in under.query_map(params![lo, hi], placement_row).map_err(e)?.filter_map(|x| x.ok()) {
                by_id.insert(p.id, p);
            }
        }
        Ok(by_id.into_values().collect())
    }

    /// Rewrite every placement and managed item path under the directory `from` to sit under `to`.
    pub fn move_placement_dir(&self, from: &str, to: &str) -> DbResult<usize> {
        let folded = fold_path(from);
        let lo = format!("{folded}/");
        let hi = format!("{folded}0");
        let skip = from.chars().count() + 1;
        let rows: Vec<(i64, i64, String)> = {
            let mut st = self.conn.prepare_cached("SELECT id, asset_id, rel_path FROM placements WHERE rel_key >= ?1 AND rel_key < ?2").map_err(e)?;
            let q = st.query_map(params![lo, hi], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(e)?;
            q.filter_map(|r| r.ok()).collect()
        };
        for (id, asset, old) in &rows {
            let tail: String = old.chars().skip(skip).collect();
            let new = if to.is_empty() { tail } else { format!("{to}/{tail}") };
            self.conn.execute("UPDATE placements SET rel_path = ?, rel_key = ? WHERE id = ?", params![new, fold_path(&new), id]).map_err(e)?;
            self.conn.execute(&format!("UPDATE assets SET rel_path = ?3 WHERE id = ?1 AND rel_path = ?2 AND {IN_STORE}"), params![asset, old, new]).map_err(e)?;
        }
        Ok(rows.len())
    }

    /// Where an op's file is now: its root may have moved since the op was recorded.
    pub fn file_op_path(&self, op: &FileOp) -> Option<PathBuf> {
        self.root(op.root_id).map(|r| filesystem::db_to_abs(&r.path, &op.rel_path))
    }

    /// Purged files of a root: rel_path → (size, mtime) when purged.
    pub fn forgotten(&self, root_id: i64) -> DbResult<HashMap<String, (i64, Option<i64>)>> {
        let mut st = self.conn.prepare_cached("SELECT rel_path, size, mtime FROM forgotten_files WHERE root_id = ?").map_err(e)?;
        let rows = st.query_map([root_id], |r| Ok((r.get::<_, String>(0)?, (r.get(1)?, r.get(2)?)))).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn forgotten_one(&self, root_id: i64, rel_path: &str) -> Option<(i64, Option<i64>)> {
        self.conn
            .prepare_cached("SELECT size, mtime FROM forgotten_files WHERE root_id = ? AND rel_path = ?")
            .and_then(|mut st| st.query_row(params![root_id, rel_path], |r| Ok((r.get(0)?, r.get(1)?))).optional())
            .ok()
            .flatten()
    }

    /// The item whose file is at `path`: in a location, or any of its entries in the library's store.
    pub fn asset_at_path(&self, path: &Path) -> Option<AssetId> {
        for r in self.roots().ok()? {
            let Ok(rel) = path.strip_prefix(&r.path) else { continue };
            let rel = filesystem::rel_to_db(rel);
            if r.kind == RootKind::Managed {
                if let Some(p) = self.placement_at(&rel) {
                    return Some(p.asset_id);
                }
            }
            if let Some(id) = self.asset_at(r.id, &rel) {
                return Some(id);
            }
        }
        None
    }

    pub fn asset_at(&self, root_id: i64, rel_path: &str) -> Option<AssetId> {
        self.conn.query_row("SELECT id FROM assets WHERE root_id = ? AND rel_path = ?", params![root_id, rel_path], |r| r.get(0)).optional().ok().flatten()
    }

    /// Items' ids as the API names them; rows that are gone are left out.
    pub fn uids(&self, ids: &[AssetId]) -> DbResult<Vec<(AssetId, String)>> {
        let mut st = self.conn.prepare_cached("SELECT uid_text(uid) FROM assets WHERE id = ?").map_err(e)?;
        let mut out = Vec::with_capacity(ids.len());
        for &id in ids {
            if let Some(u) = st.query_row([id], |r| r.get::<_, String>(0)).optional().map_err(e)? {
                out.push((id, u));
            }
        }
        Ok(out)
    }

    pub fn asset_by_uid(&self, uid: &str) -> Option<AssetId> {
        self.conn.query_row("SELECT id FROM assets WHERE uid = uid_bytes(?)", [uid], |r| r.get(0)).optional().ok().flatten()
    }

    pub fn invalidate_thumbs(&self, ids: &[AssetId]) -> DbResult<()> {
        let mut st = self
            .conn
            .prepare_cached("UPDATE assets SET thumb_state = 3, thumb_ver = thumb_ver + 1 WHERE id = ?")
            .map_err(e)?;
        for id in ids {
            st.execute([id]).map_err(e)?;
        }
        Ok(())
    }
}
