//! Folders: their tree, their items and their covers.

use crate::core::db::SUBTREE;
use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use std::collections::HashMap;
use rusqlite::{params, OptionalExtension};
use crate::core::model::*;

impl Db {
    /// A sibling of another name, ignoring case, since each folder is a directory in the store.
    fn folder_name_taken(&self, parent: Option<i64>, name: &str, except: Option<i64>) -> DbResult<bool> {
        self.conn
            .query_row("SELECT EXISTS (SELECT 1 FROM folders WHERE parent_id IS ?1 AND name = ?2 COLLATE NOCASE AND id IS NOT ?3)", params![parent, name, except], |r| r.get(0))
            .map_err(e)
    }

    /// `name`, numbered from 2 in the library's style when a sibling has it.
    pub fn free_folder_name(&self, parent: Option<i64>, name: &str, except: Option<i64>) -> DbResult<String> {
        let base = name.trim();
        let base = if base.is_empty() { "New folder" } else { base };
        let style = self.name_suffix();
        let mut cand = base.to_string();
        let mut n = 1;
        while self.folder_name_taken(parent, &cand, except)? {
            n += 1;
            cand = format!("{base}{}", style.tag(n));
        }
        Ok(cand)
    }

    /// How the library numbers a second name in one folder (Settings › General › Duplicate names), for folders and files alike.
    pub fn name_suffix(&self) -> crate::core::store::SuffixStyle {
        self.meta_get("settings").and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()).and_then(|v| serde_json::from_value(v["name_suffix"].clone()).ok()).unwrap_or_default()
    }

    pub fn create_folder(&self, name: &str, parent: Option<i64>) -> DbResult<i64> {
        let name = self.free_folder_name(parent, name, None)?;
        let order: i64 = self
            .conn
            .query_row("SELECT COALESCE(max(sort_order), 0) + 1 FROM folders WHERE parent_id IS ?", [parent], |r| r.get(0))
            .map_err(e)?;
        self.conn
            .execute(
                "INSERT INTO folders(uid, parent_id, name, sort_order, created_at) VALUES(uid_bytes(?), ?, ?, ?, ?)",
                params![new_uid(), parent, name, order, now_ms()],
            )
            .map_err(e)?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn rename_folder(&self, id: i64, name: &str) -> DbResult<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err("A folder needs a name".into());
        }
        let parent: Option<i64> = self.conn.query_row("SELECT parent_id FROM folders WHERE id = ?", [id], |r| r.get(0)).map_err(|_| format!("no folder {id}"))?;
        if self.folder_name_taken(parent, name, Some(id))? {
            return Err(format!("There is already a folder called “{name}” here"));
        }
        self.conn.execute("UPDATE folders SET name = ? WHERE id = ?", params![name, id]).map_err(e)?;
        Ok(())
    }

    pub fn set_folder_color(&self, id: i64, color: Option<&str>) -> DbResult<()> {
        self.conn.execute("UPDATE folders SET color = ? WHERE id = ?", params![color, id]).map_err(e)?;
        Ok(())
    }

    pub fn set_folder_icon(&self, id: i64, icon: Option<&str>) -> DbResult<()> {
        self.conn.execute("UPDATE folders SET icon = ? WHERE id = ?", params![icon, id]).map_err(e)?;
        Ok(())
    }

    pub fn move_folder(&self, id: i64, new_parent: Option<i64>) -> DbResult<()> {
        // refuse cycles
        let mut p = new_parent;
        while let Some(pid) = p {
            if pid == id {
                return Err("A folder can't be moved into itself".into());
            }
            p = self.conn.query_row("SELECT parent_id FROM folders WHERE id = ?", [pid], |r| r.get(0)).optional().map_err(e)?.flatten();
        }
        let (name, parent): (String, Option<i64>) = self.conn.query_row("SELECT name, parent_id FROM folders WHERE id = ?", [id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(|_| format!("no folder {id}"))?;
        if parent == new_parent {
            return Ok(());
        }
        let name = self.free_folder_name(new_parent, &name, Some(id))?;
        self.conn.execute("UPDATE folders SET parent_id = ?, name = ? WHERE id = ?", params![new_parent, name, id]).map_err(e)?;
        Ok(())
    }

    /// Put everything in `src` into `dst` and remove `src`: its items, trashed entries and files join `dst`, and its subfolders join it too, merged with the ones of the same name.
    pub fn merge_folder(&self, src: i64, dst: i64) -> DbResult<()> {
        if src == dst {
            return Ok(());
        }
        let mut p = Some(dst);
        while let Some(pid) = p {
            if pid == src {
                return Err("A folder can't be merged into one inside it".into());
            }
            p = self.conn.query_row("SELECT parent_id FROM folders WHERE id = ?", [pid], |r| r.get(0)).optional().map_err(e)?.flatten();
        }
        self.conn
            .execute(
                "INSERT OR IGNORE INTO asset_folders(folder_id, asset_id, position) SELECT ?2, asset_id, position + (SELECT COALESCE(max(position), 0) FROM asset_folders WHERE folder_id = ?2) FROM asset_folders WHERE folder_id = ?1",
                params![src, dst],
            )
            .map_err(e)?;
        self.conn
            .execute("INSERT OR IGNORE INTO trashed_entries(asset_id, folder_id, position, trashed_at) SELECT asset_id, ?2, position, trashed_at FROM trashed_entries WHERE folder_id = ?1", params![src, dst])
            .map_err(e)?;
        self.conn
            .execute("UPDATE placements SET folder_id = ?2 WHERE folder_id = ?1 AND NOT EXISTS (SELECT 1 FROM placements o WHERE o.asset_id = placements.asset_id AND o.folder_id = ?2)", params![src, dst])
            .map_err(e)?;
        let kids: Vec<(i64, String)> = {
            let mut st = self.conn.prepare("SELECT id, name FROM folders WHERE parent_id = ? ORDER BY sort_order, id").map_err(e)?;
            let rows = st.query_map([src], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
            rows.filter_map(|r| r.ok()).collect()
        };
        for (kid, name) in kids {
            let same: Option<i64> = self.conn.query_row("SELECT id FROM folders WHERE parent_id = ? AND name = ? COLLATE NOCASE", params![dst, name], |r| r.get(0)).optional().map_err(e)?;
            match same {
                Some(other) => self.merge_folder(kid, other)?,
                None => {
                    self.conn.execute("UPDATE folders SET parent_id = ? WHERE id = ?", params![dst, kid]).map_err(e)?;
                }
            }
        }
        self.conn.execute("DELETE FROM folders WHERE id = ?", [src]).map_err(e)?;
        Ok(())
    }

    /// Delete a virtual folder and its subfolders. Assets stay in the library.
    /// Put a folder under `parent`, just before `before` (or last). Siblings are renumbered.
    pub fn place_folder(&self, id: i64, parent: Option<i64>, before: Option<i64>) -> DbResult<()> {
        self.move_folder(id, parent)?;
        let mut list: Vec<i64> = {
            let mut st = self
                .conn
                .prepare("SELECT id FROM folders WHERE parent_id IS ? AND id <> ? ORDER BY sort_order, name COLLATE NOCASE")
                .map_err(e)?;
            let rows = st.query_map(rusqlite::params![parent, id], |r| r.get(0)).map_err(e)?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let at = before.and_then(|b| list.iter().position(|x| *x == b)).unwrap_or(list.len());
        list.insert(at, id);
        for (i, f) in list.iter().enumerate() {
            self.conn.execute("UPDATE folders SET sort_order = ? WHERE id = ?", rusqlite::params![i as i64 + 1, f]).map_err(e)?;
        }
        Ok(())
    }

    pub fn set_tag_starred(&self, id: i64, starred: bool) -> DbResult<()> {
        self.conn.execute("UPDATE tags SET starred = ? WHERE id = ?", rusqlite::params![starred as i64, id]).map_err(e)?;
        Ok(())
    }

    /// Look at images whose animated flag isn't known yet. Returns how many were checked and the ones found animated.
    pub fn backfill_animated(&self, limit: i64) -> DbResult<(usize, Vec<AssetId>)> {
        let rows: Vec<AssetId> = {
            let mut st = self
                .conn
                .prepare("SELECT id FROM assets WHERE animated IS NULL AND status = 0 AND part_of IS NULL AND ext IN ('gif', 'webp', 'png', 'apng') LIMIT ?")
                .map_err(e)?;
            let q = st.query_map([limit], |r| r.get(0)).map_err(e)?;
            q.filter_map(|r| r.ok()).collect()
        };
        let checked = rows.len();
        let mut found = Vec::new();
        for id in rows {
            let Some(path) = self.abs_path(id) else { continue };
            let ext = path.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
            let yes = crate::core::metadata::extract::is_animated(&path, &ext);
            self.conn.execute("UPDATE assets SET animated = ? WHERE id = ?", rusqlite::params![yes as i64, id]).map_err(e)?;
            if yes {
                found.push(id);
            }
        }
        Ok((checked, found))
    }

    /// Clear `animated = 0` on WebP items once so the chunk-aware detector re-checks existing libraries.
    pub fn reset_webp_animated(&self) -> DbResult<()> {
        self.conn.execute("UPDATE assets SET animated = NULL WHERE ext = 'webp' AND animated = 0", []).map_err(e)?;
        Ok(())
    }

    /// Delete a folder and its subfolders. Items that belong to no folder outside them go to the trash; the others only lose these memberships. Returns the trashed items.
    pub fn delete_folder(&self, id: i64) -> DbResult<Vec<AssetId>> {
        const SUB: &str = "WITH RECURSIVE sub(id) AS (SELECT ?1 UNION ALL SELECT f.id FROM folders f JOIN sub ON f.parent_id = sub.id)";
        let trashed: Vec<AssetId> = {
            let mut st = self
                .conn
                .prepare(&format!(
                    "{SUB} SELECT DISTINCT x.asset_id FROM asset_folders x JOIN assets a ON a.id = x.asset_id
                     WHERE x.folder_id IN (SELECT id FROM sub) AND a.deleted = 0
                       AND NOT EXISTS (SELECT 1 FROM asset_folders o WHERE o.asset_id = x.asset_id AND o.folder_id NOT IN (SELECT id FROM sub))"
                ))
                .map_err(e)?;
            let rows = st.query_map([id], |r| r.get(0)).map_err(e)?;
            rows.filter_map(|r| r.ok()).collect()
        };
        self.set_deleted(&trashed, true)?;
        self.remember_folders(id)?;
        self.conn.execute(&format!("{SUB} DELETE FROM folders WHERE id IN (SELECT id FROM sub)"), [id]).map_err(e)?;
        Ok(trashed)
    }

    /// Names from the top down to `folder`.
    fn folder_path(&self, folder: i64) -> DbResult<Vec<String>> {
        let mut out = vec![];
        let mut at = Some(folder);
        while let Some(f) = at {
            let (name, parent): (String, Option<i64>) = self.conn.query_row("SELECT name, parent_id FROM folders WHERE id = ?", [f], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
            out.push(name);
            at = parent;
        }
        out.reverse();
        Ok(out)
    }

    /// Before `folder` and its subfolders are deleted: where their trashed items and trashed entries were, so a restore can bring the folders back.
    fn remember_folders(&self, folder: i64) -> DbResult<()> {
        let rows: Vec<(AssetId, i64)> = {
            let mut st = self
                .conn
                .prepare(&format!(
                    "{SUBTREE} SELECT x.asset_id, x.folder_id FROM asset_folders x JOIN assets a ON a.id = x.asset_id WHERE a.deleted = 1 AND x.folder_id IN (SELECT id FROM sub)
                     UNION SELECT asset_id, folder_id FROM trashed_entries WHERE folder_id IN (SELECT id FROM sub)"
                ))
                .map_err(e)?;
            let q = st.query_map([folder], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
            q.filter_map(|r| r.ok()).collect()
        };
        let mut paths: HashMap<i64, String> = HashMap::new();
        let mut st = self.conn.prepare_cached("INSERT OR IGNORE INTO trashed_folders(asset_id, path) VALUES (?, ?)").map_err(e)?;
        for (asset, f) in rows {
            if !paths.contains_key(&f) {
                paths.insert(f, serde_json::to_string(&self.folder_path(f)?).map_err(e)?);
            }
            st.execute(params![asset, paths[&f]]).map_err(e)?;
        }
        Ok(())
    }

    /// Bring trashed items, and items' trashed folder entries, back into `folder` alone.
    pub fn restore_into(&self, ids: &[AssetId], folder: i64) -> DbResult<()> {
        let mut whole = self.conn.prepare_cached("SELECT deleted FROM assets WHERE id = ?").map_err(e)?;
        for id in ids {
            if whole.query_row([id], |r| r.get::<_, bool>(0)).map_err(e)? {
                self.conn.execute("DELETE FROM asset_folders WHERE asset_id = ?", [id]).map_err(e)?;
            }
            self.conn.execute("DELETE FROM trashed_entries WHERE asset_id = ?", [id]).map_err(e)?;
            self.conn.execute("DELETE FROM trashed_folders WHERE asset_id = ?", [id]).map_err(e)?;
        }
        self.set_deleted(ids, false)?;
        self.add_to_folder(ids, folder)
    }

    /// Put restored items back in the deleted folders they were in, making the folders again by name.
    pub fn restore_folders(&self, ids: &[AssetId]) -> DbResult<()> {
        for id in ids {
            let paths: Vec<String> = {
                let mut st = self.conn.prepare_cached("SELECT path FROM trashed_folders WHERE asset_id = ?").map_err(e)?;
                let q = st.query_map([id], |r| r.get(0)).map_err(e)?;
                q.filter_map(|r| r.ok()).collect()
            };
            for path in paths {
                let names: Vec<String> = serde_json::from_str(&path).unwrap_or_default();
                let mut parent: Option<i64> = None;
                for name in &names {
                    let found: Option<i64> = self.conn.query_row("SELECT id FROM folders WHERE parent_id IS ? AND name = ? COLLATE NOCASE", params![parent, name], |r| r.get(0)).optional().map_err(e)?;
                    parent = Some(match found {
                        Some(f) => f,
                        None => self.create_folder(name, parent)?,
                    });
                }
                if let Some(f) = parent {
                    self.conn
                        .execute("INSERT OR IGNORE INTO asset_folders(folder_id, asset_id, position) VALUES (?1, ?2, (SELECT COALESCE(max(position), 0) + 1 FROM asset_folders WHERE folder_id = ?1))", params![f, id])
                        .map_err(e)?;
                }
            }
            self.conn.execute("DELETE FROM trashed_folders WHERE asset_id = ?", [id]).map_err(e)?;
        }
        Ok(())
    }

    /// Folders with `count` = distinct items in the folder and all its subfolders.
    pub fn folders(&self) -> DbResult<Vec<Folder>> {
        let mut st = self
            .conn
            .prepare(
                "WITH RECURSIVE up(folder_id, top) AS (
                     SELECT id, id FROM folders
                     UNION ALL SELECT up.folder_id, f.parent_id FROM up JOIN folders f ON f.id = up.top WHERE f.parent_id IS NOT NULL),
                 deep(top, n) AS (
                     SELECT up.top, count(DISTINCT x.asset_id) FROM up JOIN asset_folders x ON x.folder_id = up.folder_id JOIN assets a ON a.id = x.asset_id
                     WHERE a.deleted = 0 GROUP BY up.top)
                 SELECT f.id, f.parent_id, f.name, f.color, f.sort_order, COALESCE(deep.n, 0), f.icon, f.auto_tags, f.created_at
                 FROM folders f LEFT JOIN deep ON deep.top = f.id ORDER BY f.sort_order, f.name COLLATE NOCASE",
            )
            .map_err(e)?;
        let rows = st
            .query_map([], |r| {
                Ok(Folder { id: r.get(0)?, parent_id: r.get(1)?, name: r.get(2)?, color: r.get(3)?, sort_order: r.get(4)?, count: r.get(5)?, icon: r.get(6)?, auto_tags: r.get::<_, Option<String>>(7)?.and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default(), created_at: r.get(8)? })
            })
            .map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Ids and names of the folders directly under `parent` (top level for `None`).
    pub fn subfolders(&self, parent: Option<i64>) -> DbResult<Vec<(i64, String)>> {
        let mut st = self.conn.prepare_cached("SELECT id, name FROM folders WHERE parent_id IS ?").map_err(e)?;
        let rows = st.query_map([parent], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// The items of a location, or of one of its folders (`dir`, relative to it), with their files there: not in the trash, missing or grouped into another.
    pub fn items_under(&self, root: i64, dir: Option<&str>) -> DbResult<Vec<AssetId>> {
        let mut st = self
            .conn
            .prepare("SELECT id FROM assets WHERE root_id = ?1 AND part_of IS NULL AND deleted = 0 AND status = 0 AND (?2 IS NULL OR substr(rel_path, 1, length(?2) + 1) = ?2 || '/') ORDER BY id")
            .map_err(e)?;
        let rows = st.query_map(params![root, dir], |r| r.get(0)).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Tags added to whatever lands in this folder from now on.
    pub fn set_folder_auto_tags(&self, id: i64, tags: &[String]) -> DbResult<()> {
        let json = if tags.is_empty() { None } else { serde_json::to_string(tags).ok() };
        self.conn.execute("UPDATE folders SET auto_tags = ? WHERE id = ?", params![json, id]).map_err(e)?;
        if !tags.is_empty() {
            let mut st = self.conn.prepare("SELECT asset_id FROM asset_folders WHERE folder_id = ?").map_err(e)?;
            let inside: Vec<AssetId> = st.query_map([id], |r| r.get(0)).map_err(e)?.filter_map(|r| r.ok()).collect();
            if !inside.is_empty() {
                self.add_tags(&inside, tags)?;
            }
        }
        Ok(())
    }

    /// Put folders in title order: the children of `parent`, or every level.
    /// Orders the subfolders of `parent` (or of every folder) by name, or by the items they hold with their subfolders' as the sidebar counts them, ties by name.
    pub fn sort_folders(&self, parent: Option<i64>, descending: bool, all_levels: bool, by_count: bool) -> DbResult<()> {
        let all = self.folders()?;
        let mut parents: Vec<Option<i64>> = if all_levels { all.iter().map(|f| f.parent_id).collect() } else { vec![parent] };
        parents.sort();
        parents.dedup();
        for p in parents {
            let mut kids: Vec<&Folder> = all.iter().filter(|f| f.parent_id == p).collect();
            kids.sort_by(|a, b| {
                let by_name = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                let order = if by_count { a.count.cmp(&b.count) } else { by_name };
                let order = if descending { order.reverse() } else { order };
                // equal counts stay A to Z either way
                order.then(by_name)
            });
            for (i, f) in kids.iter().enumerate() {
                self.conn.execute("UPDATE folders SET sort_order = ? WHERE id = ?", params![i as i64 + 1, f.id]).map_err(e)?;
            }
        }
        Ok(())
    }

    /// A subfolder by name, ignoring case.
    pub fn child_folder(&self, parent: Option<i64>, name: &str) -> DbResult<Option<i64>> {
        self.conn
            .query_row("SELECT id FROM folders WHERE parent_id IS ? AND name = ? COLLATE NOCASE", params![parent, name], |r| r.get(0))
            .optional()
            .map_err(e)
    }

    /// A copy of a folder beside it, with the same items, subfolders and settings.
    pub fn clone_folder(&self, id: i64) -> DbResult<i64> {
        let (name, parent): (String, Option<i64>) = self.conn.query_row("SELECT name, parent_id FROM folders WHERE id = ?", [id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
        self.copy_folder_into(id, parent, &format!("{name} copy"))
    }

    fn copy_folder_into(&self, id: i64, parent: Option<i64>, name: &str) -> DbResult<i64> {
        let new_id = self.create_folder(name, parent)?;
        self.conn
            .execute(
                "UPDATE folders SET color = o.color, icon = o.icon, auto_tags = o.auto_tags FROM (SELECT color, icon, auto_tags FROM folders WHERE id = ?1) o WHERE folders.id = ?2",
                params![id, new_id],
            )
            .map_err(e)?;
        self.conn
            .execute("INSERT OR IGNORE INTO asset_folders(folder_id, asset_id, position) SELECT ?1, asset_id, position FROM asset_folders WHERE folder_id = ?2", params![new_id, id])
            .map_err(e)?;
        let kids: Vec<(i64, String)> = {
            let mut st = self.conn.prepare("SELECT id, name FROM folders WHERE parent_id = ? ORDER BY sort_order").map_err(e)?;
            let rows = st.query_map([id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
            rows.filter_map(|r| r.ok()).collect()
        };
        for (kid, kid_name) in kids {
            self.copy_folder_into(kid, Some(new_id), &kid_name)?;
        }
        Ok(new_id)
    }

    /// Saved favorites: folders, smart folders and tags, in the person's order.
    pub fn favorites(&self) -> Vec<serde_json::Value> {
        self.meta_get("favorites").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn set_favorites(&self, list: &[serde_json::Value]) -> DbResult<()> {
        self.meta_set("favorites", &serde_json::to_string(list).unwrap_or_default())
    }

    pub fn add_to_folder(&self, ids: &[AssetId], folder: i64) -> DbResult<()> {
        let auto: Option<String> = self.conn.query_row("SELECT auto_tags FROM folders WHERE id = ?", [folder], |r| r.get(0)).optional().map_err(e)?.flatten();
        if let Some(names) = auto.and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok()) {
            if !names.is_empty() {
                self.add_tags(ids, &names)?;
            }
        }
        let mut pos: f64 = self
            .conn
            .query_row("SELECT COALESCE(max(position), 0) FROM asset_folders WHERE folder_id = ?", [folder], |r| r.get(0))
            .map_err(e)?;
        let mut st = self
            .conn
            .prepare_cached("INSERT OR IGNORE INTO asset_folders(folder_id, asset_id, position) VALUES(?, ?, ?)")
            .map_err(e)?;
        for id in ids {
            pos += 1.0;
            st.execute(params![folder, id, pos]).map_err(e)?;
        }
        self.touch(ids)
    }

    /// Delete in a folder view: each item's entries in `folder` (and its subfolders with `deep`) go to the trash while it stays in its other folders; an item in no other folder goes to the trash whole, keeping its folders for a restore. Returns those.
    pub fn trash_entries(&self, ids: &[AssetId], folder: i64, deep: bool) -> DbResult<Vec<AssetId>> {
        let within = if deep { SUBTREE } else { "WITH sub(id) AS (SELECT ?1)" };
        let mut elsewhere = self.conn.prepare_cached(&format!("{within} SELECT EXISTS (SELECT 1 FROM asset_folders WHERE asset_id = ?2 AND folder_id NOT IN (SELECT id FROM sub))")).map_err(e)?;
        let mut keep = self
            .conn
            .prepare_cached(&format!("{within} INSERT INTO trashed_entries(asset_id, folder_id, position, trashed_at) SELECT asset_id, folder_id, position, ?3 FROM asset_folders WHERE asset_id = ?2 AND folder_id IN (SELECT id FROM sub) ON CONFLICT(asset_id, folder_id) DO UPDATE SET position = excluded.position, trashed_at = excluded.trashed_at"))
            .map_err(e)?;
        let mut drop = self.conn.prepare_cached(&format!("{within} DELETE FROM asset_folders WHERE asset_id = ?2 AND folder_id IN (SELECT id FROM sub)")).map_err(e)?;
        let mut whole = Vec::new();
        let now = now_ms();
        for id in ids {
            if elsewhere.query_row(params![folder, id], |r| r.get::<_, bool>(0)).map_err(e)? {
                keep.execute(params![folder, id, now]).map_err(e)?;
                drop.execute(params![folder, id]).map_err(e)?;
            } else {
                whole.push(*id);
            }
        }
        self.set_deleted(&whole, true)?;
        self.touch(ids)?;
        Ok(whole)
    }

    /// Those of `ids` in the trash.
    pub fn trashed_among(&self, ids: &[AssetId]) -> DbResult<Vec<AssetId>> {
        let mut st = self.conn.prepare_cached("SELECT deleted FROM assets WHERE id = ?").map_err(e)?;
        let mut out = Vec::new();
        for id in ids {
            if st.query_row([id], |r| r.get::<_, bool>(0)).optional().map_err(e)?.unwrap_or(false) {
                out.push(*id);
            }
        }
        Ok(out)
    }

    /// How many tags and folders there are, to tell whether an edit made new ones.
    pub fn list_sizes(&self) -> DbResult<(i64, i64)> {
        self.conn.query_row("SELECT (SELECT COUNT(*) FROM tags), (SELECT COUNT(*) FROM folders)", [], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)
    }

    /// Bring items' trashed folder entries back: those in `folder` and its subfolders, or all of them.
    pub fn restore_entries(&self, ids: &[AssetId], folder: Option<i64>) -> DbResult<()> {
        let within = match folder {
            Some(_) => SUBTREE,
            None => "WITH sub(id) AS (SELECT folder_id FROM trashed_entries WHERE ?1 IS NULL)",
        };
        let mut back = self
            .conn
            .prepare_cached(&format!("{within} INSERT OR IGNORE INTO asset_folders(folder_id, asset_id, position) SELECT folder_id, asset_id, position FROM trashed_entries WHERE asset_id = ?2 AND folder_id IN (SELECT id FROM sub)"))
            .map_err(e)?;
        let mut gone = self.conn.prepare_cached(&format!("{within} DELETE FROM trashed_entries WHERE asset_id = ?2 AND folder_id IN (SELECT id FROM sub)")).map_err(e)?;
        for id in ids {
            back.execute(params![folder, id]).map_err(e)?;
            gone.execute(params![folder, id]).map_err(e)?;
        }
        self.touch(ids)
    }

    /// Take items out of `folder` and its subfolders.
    pub fn remove_from_folder_tree(&self, ids: &[AssetId], folder: i64) -> DbResult<()> {
        let mut st = self
            .conn
            .prepare_cached(&format!("{SUBTREE} DELETE FROM asset_folders WHERE asset_id = ?2 AND folder_id IN (SELECT id FROM sub)"))
            .map_err(e)?;
        for id in ids {
            st.execute(params![folder, id]).map_err(e)?;
        }
        self.touch(ids)
    }

    pub fn remove_from_folder(&self, ids: &[AssetId], folder: i64) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("DELETE FROM asset_folders WHERE folder_id = ? AND asset_id = ?").map_err(e)?;
        for id in ids {
            st.execute(params![folder, id]).map_err(e)?;
        }
        self.touch(ids)
    }

    /// Manual ordering: place `ids` before `before` (or at the end) inside a folder.
    pub fn reorder_in_folder(&self, folder: i64, ids: &[AssetId], before: Option<AssetId>) -> DbResult<()> {
        let target: f64 = match before {
            Some(b) => self
                .conn
                .query_row("SELECT position FROM asset_folders WHERE folder_id = ? AND asset_id = ?", params![folder, b], |r| r.get(0))
                .unwrap_or(0.0),
            None => self
                .conn
                .query_row("SELECT COALESCE(max(position), 0) + 1 FROM asset_folders WHERE folder_id = ?", [folder], |r| r.get(0))
                .map_err(e)?,
        };
        let prev: f64 = self
            .conn
            .query_row(
                "SELECT COALESCE(max(position), ? - 1) FROM asset_folders WHERE folder_id = ? AND position < ?",
                params![target, folder, target],
                |r| r.get(0),
            )
            .map_err(e)?;
        let step = (target - prev) / (ids.len() as f64 + 1.0);
        for (i, id) in ids.iter().enumerate() {
            self.conn
                .execute(
                    "UPDATE asset_folders SET position = ? WHERE folder_id = ? AND asset_id = ?",
                    params![prev + step * (i as f64 + 1.0), folder, id],
                )
                .map_err(e)?;
        }
        Ok(())
    }

    pub fn asset_folders(&self, id: AssetId) -> DbResult<Vec<Folder>> {
        let mut st = self
            .conn
            .prepare("SELECT f.id, f.parent_id, f.name, f.color, f.sort_order FROM asset_folders x JOIN folders f ON f.id = x.folder_id WHERE x.asset_id = ?")
            .map_err(e)?;
        let rows = st
            .query_map([id], |r| {
                Ok(Folder { id: r.get(0)?, parent_id: r.get(1)?, name: r.get(2)?, color: r.get(3)?, sort_order: r.get(4)?, count: 0, icon: None, auto_tags: vec![], created_at: 0 })
            })
            .map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
