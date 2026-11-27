//! Locations: the folders a library reads, their state and what they hold.

use crate::core::db::queue_link_recycles;
use crate::core::db::fold_path;
use crate::core::db::read_only_message;
use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use std::path::Path;
use rusqlite::{params, OptionalExtension};
use crate::core::system::filesystem;
use crate::core::model::*;

fn root_row(r: &rusqlite::Row) -> rusqlite::Result<Root> {
    let path: String = r.get(1)?;
    let kind = RootKind::parse(&r.get::<_, String>(2)?);
    let explicit_only = r.get::<_, i64>(6)? != 0;
    Ok(Root {
        id: r.get(0)?,
        listed: kind == RootKind::Linked && (!explicit_only || crate::core::import::eagle::eagle_images_path(&path)),
        path,
        kind,
        label: r.get(3)?,
        enabled: r.get::<_, i64>(4)? != 0,
        watch: r.get::<_, i64>(5)? != 0,
        explicit_only,
        online: kind != RootKind::Unknown && r.get::<_, i64>(7)? != 0,
        last_scan: r.get(8)?,
        asset_count: r.get(9)?,
        access: if kind == RootKind::Unknown { RootAccess::ReadOnly } else { RootAccess::parse(&r.get::<_, String>(10)?) },
        hint: None,
        icon: r.get(11)?,
        color: r.get(12)?,
    })
}

impl Db {
    pub fn add_root(&self, path: &str, kind: RootKind, explicit_only: bool) -> DbResult<i64> {
        if let Some(id) = self.root_id_for_path(path) {
            // adding a folder as a full location upgrades one that only held individually linked files
            if !explicit_only {
                self.conn.execute("UPDATE roots SET explicit_only = 0 WHERE id = ? AND explicit_only <> 0", [id]).map_err(e)?;
            }
            return Ok(id);
        }
        let label = Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string());
        let volume = filesystem::volume_of(Path::new(path));
        self.conn
            .execute(
                "INSERT INTO roots(uid, path, kind, label, volume, explicit_only, sort_order) VALUES(uid_bytes(?), ?, ?, ?, ?, ?, (SELECT COALESCE(max(sort_order), 0) + 1 FROM roots))",
                params![new_uid(), path, kind.as_str(), label, volume, explicit_only as i64],
            )
            .map_err(e)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Every folder inside a location that holds items somewhere below it, with the number it holds; sorted by path.
    pub fn root_dirs(&self, root: i64) -> DbResult<Vec<(String, i64)>> {
        // rtrim with every character but `/` cuts the file name off, leaving "a/b/" (or "" for the top)
        let mut st = self
            .conn
            .prepare("SELECT rtrim(rel_path, replace(rel_path, '/', '')), count(*) FROM assets WHERE root_id = ? AND deleted = 0 AND part_of IS NULL GROUP BY 1")
            .map_err(e)?;
        let rows: Vec<(String, i64)> = st.query_map([root], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?.filter_map(|r| r.ok()).collect();
        let mut dirs: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
        for (dir, n) in rows {
            let mut at = 0;
            while let Some(i) = dir[at..].find('/') {
                *dirs.entry(dir[..at + i].to_string()).or_default() += n;
                at += i + 1;
            }
        }
        Ok(dirs.into_iter().collect())
    }

    /// Put a location just before `before` (or last). All locations are renumbered.
    pub fn place_root(&self, id: i64, before: Option<i64>) -> DbResult<()> {
        let mut list: Vec<i64> = self.roots()?.into_iter().map(|r| r.id).filter(|r| *r != id).collect();
        let at = before.and_then(|b| list.iter().position(|x| *x == b)).unwrap_or(list.len());
        list.insert(at, id);
        for (i, r) in list.iter().enumerate() {
            self.conn.execute("UPDATE roots SET sort_order = ? WHERE id = ?", params![i as i64 + 1, r]).map_err(e)?;
        }
        Ok(())
    }

    /// The managed store moves with its library folder, so its stored path follows. Rows left by opens under an earlier path are merged in.
    pub fn repoint_managed_root(&mut self, path: &str) -> DbResult<()> {
        let rows: Vec<(i64, String)> = {
            let mut st = self.conn.prepare("SELECT id, path FROM roots WHERE kind = ? ORDER BY id").map_err(e)?;
            let rows = st.query_map([RootKind::Managed.as_str()], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
            rows.filter_map(|r| r.ok()).collect()
        };
        if rows.iter().all(|(_, p)| p == path) {
            return Ok(());
        }
        self.tx(|db| {
            let keep = match rows.iter().find(|(_, p)| p == path) {
                Some((id, _)) => *id,
                None => {
                    let id = rows[0].0;
                    db.conn.execute("UPDATE roots SET path = ?, volume = ? WHERE id = ?", params![path, filesystem::volume_of(Path::new(path)), id]).map_err(e)?;
                    id
                }
            };
            for (id, _) in rows.iter().filter(|(id, _)| *id != keep) {
                db.conn.execute("UPDATE OR IGNORE assets SET root_id = ? WHERE root_id = ?", params![keep, id]).map_err(e)?;
                db.conn.execute("UPDATE file_ops SET root_id = ? WHERE root_id = ?", params![keep, id]).map_err(e)?;
                db.conn.execute("UPDATE OR IGNORE forgotten_files SET root_id = ? WHERE root_id = ?", params![keep, id]).map_err(e)?;
                let left: i64 = db.conn.query_row("SELECT count(*) FROM assets WHERE root_id = ?", [id], |r| r.get(0)).map_err(e)?;
                if left == 0 {
                    db.conn.execute("DELETE FROM roots WHERE id = ?", [id]).map_err(e)?;
                }
            }
            Ok(())
        })
    }

    /// Placement keys made by an earlier folding rule are made again.
    pub fn refold_placement_keys(&self) -> DbResult<usize> {
        self.conn.execute("UPDATE OR IGNORE placements SET rel_key = fold_path(rel_path) WHERE rel_key <> fold_path(rel_path)", []).map_err(e)
    }

    pub fn root_id_for_path(&self, path: &str) -> Option<i64> {
        self.conn.query_row("SELECT id FROM roots WHERE path = ?", [path], |r| r.get(0)).optional().ok().flatten()
    }

    pub fn roots(&self) -> DbResult<Vec<Root>> {
        let mut st = self
            .conn
            .prepare(
                "SELECT r.id, r.path, r.kind, r.label, r.enabled, r.watch, r.explicit_only, r.online, r.last_scan,
                        (SELECT count(*) FROM assets a WHERE a.root_id = r.id AND a.deleted = 0 AND a.part_of IS NULL), r.access, r.icon, r.color
                 FROM roots r ORDER BY r.sort_order, r.kind DESC, r.label COLLATE NOCASE",
            )
            .map_err(e)?;
        let rows = st.query_map([], root_row).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// One location, without the item count the sidebar shows.
    pub fn root(&self, id: i64) -> Option<Root> {
        self.conn
            .query_row("SELECT r.id, r.path, r.kind, r.label, r.enabled, r.watch, r.explicit_only, r.online, r.last_scan, 0, r.access, r.icon, r.color FROM roots r WHERE r.id = ?", [id], root_row)
            .optional()
            .ok()
            .flatten()
    }

    /// Remove the locations made for files linked one by one that no item uses any more (the triggers miss those whose file work was still pending).
    pub fn drop_empty_loose_roots(&self) -> DbResult<usize> {
        self.conn
            .execute("DELETE FROM roots WHERE id IN (SELECT r.id FROM roots r WHERE r.kind = 'linked' AND r.explicit_only = 1 AND NOT (r.path LIKE '%.library/images' OR r.path LIKE '%.library\\images') AND NOT EXISTS (SELECT 1 FROM assets a WHERE a.root_id = r.id) AND NOT EXISTS (SELECT 1 FROM file_ops o WHERE o.root_id = r.id))", [])
            .map_err(e)
    }

    pub fn set_root_online(&self, id: i64, online: bool) -> DbResult<()> {
        self.conn.execute("UPDATE roots SET online = ? WHERE id = ?", params![online as i64, id]).map_err(e)?;
        if !online {
            self.conn.execute("UPDATE assets SET status = 1 WHERE root_id = ? AND status = 0", [id]).map_err(e)?;
        }
        Ok(())
    }

    /// The managed store is the library's own, so it always stays writable.
    pub fn set_root_access(&self, id: i64, access: RootAccess) -> DbResult<()> {
        let n = self
            .conn
            .execute("UPDATE roots SET access = ? WHERE id = ? AND kind != 'managed'", params![access.as_str(), id])
            .map_err(e)?;
        if n == 0 {
            return Err("The library's own storage can't be made read-only".into());
        }
        Ok(())
    }

    /// The one check before Curio writes a file inside a root.
    pub fn require_writable(&self, root_id: i64) -> DbResult<()> {
        let row: Option<(String, String)> = self
            .conn
            .query_row("SELECT label, access FROM roots WHERE id = ?", [root_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()
            .map_err(e)?;
        match row {
            Some((label, access)) if RootAccess::parse(&access) == RootAccess::ReadOnly => Err(read_only_message(&label)),
            _ => Ok(()),
        }
    }

    /// `require_writable` for a path that needn't belong to an asset: refused inside any read-only root.
    pub fn require_writable_path(&self, path: &Path) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("SELECT path, label FROM roots WHERE access = 'read_only'").map_err(e)?;
        let rows: Vec<(String, String)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?.filter_map(|r| r.ok()).collect();
        // compared resolved and, where the filesystem ignores case, folded, so another spelling of the same folder is still caught
        let key = |p: &Path| {
            let c = filesystem::canonical_existing_part(p).to_string_lossy().replace('\\', "/");
            let c = if cfg!(any(windows, target_os = "macos")) { fold_path(&c) } else { c };
            c.trim_end_matches('/').to_string()
        };
        let at = key(path);
        match rows.iter().find(|(root, _)| {
            let r = key(Path::new(root));
            at == r || at.starts_with(&format!("{r}/"))
        }) {
            Some((_, label)) => Err(read_only_message(label)),
            None => Ok(()),
        }
    }

    /// A location's icon or colour; empty clears it.
    pub fn set_root_label(&self, id: i64, label: &str) -> DbResult<()> {
        self.conn.execute("UPDATE roots SET label = ? WHERE id = ?", params![label, id]).map_err(e)?;
        Ok(())
    }

    pub fn set_root_look(&self, id: i64, icon: bool, value: Option<&str>) -> DbResult<()> {
        let col = if icon { "icon" } else { "color" };
        let v = value.filter(|v| !v.is_empty());
        self.conn.execute(&format!("UPDATE roots SET {col} = ? WHERE id = ?"), params![v, id]).map_err(e)?;
        Ok(())
    }

    pub fn set_root_flags(&self, id: i64, enabled: bool, watch: bool) -> DbResult<()> {
        self.conn
            .execute("UPDATE roots SET enabled = ?, watch = ? WHERE id = ?", params![enabled as i64, watch as i64, id])
            .map_err(e)?;
        Ok(())
    }

    pub fn touch_root_scan(&self, id: i64) -> DbResult<()> {
        self.conn.execute("UPDATE roots SET last_scan = ? WHERE id = ?", params![now_ms(), id]).map_err(e)?;
        Ok(())
    }

    /// Point a root at a new location (drive letter changed, share remounted, tree moved).
    /// Asset ids, tags and metadata are untouched; status is re-checked by the next scan.
    pub fn relocate_root(&self, id: i64, new_path: &str) -> DbResult<()> {
        let volume = filesystem::volume_of(Path::new(new_path));
        self.conn
            .execute("UPDATE roots SET path = ?, volume = ?, online = 1 WHERE id = ?", params![new_path, volume, id])
            .map_err(e)?;
        Ok(())
    }

    /// Remove a root from the library. Its index rows go to the trash flag first; files are
    /// never touched. With `purge` the rows (and their tags/fields) are deleted from the index.
    pub fn remove_root(&mut self, id: i64, purge: bool) -> DbResult<()> {
        let tx = self.conn.savepoint().map_err(e)?;
        tx.execute("DELETE FROM forgotten_files WHERE root_id = ?", [id]).map_err(e)?;
        if purge {
            queue_link_recycles(&tx, &format!("a.root_id = {id}"))?;
            tx.execute("DELETE FROM asset_fts WHERE rowid IN (SELECT id FROM assets WHERE root_id = ?)", [id]).map_err(e)?;
            tx.execute("DELETE FROM assets WHERE root_id = ?", [id]).map_err(e)?;
            tx.execute("DELETE FROM roots WHERE id = ?", [id]).map_err(e)?;
        } else {
            tx.execute("UPDATE roots SET enabled = 0, watch = 0 WHERE id = ?", [id]).map_err(e)?;
            tx.execute("UPDATE assets SET deleted = 1 WHERE root_id = ?", [id]).map_err(e)?;
        }
        tx.commit().map_err(e)
    }
}
