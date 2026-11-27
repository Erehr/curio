//! Tags and tag groups.

use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use rusqlite::{params, OptionalExtension};
use crate::core::system::filesystem;
use crate::core::model::*;

impl Db {
    pub fn tag_id(&self, name: &str) -> Option<i64> {
        self.conn.query_row("SELECT id FROM tags WHERE name = ?", [name.trim()], |r| r.get(0)).optional().ok().flatten()
    }

    pub fn get_or_create_tag(&self, name: &str) -> DbResult<i64> {
        let name = name.trim();
        if name.is_empty() {
            return Err("empty tag".into());
        }
        if let Some(id) = self.tag_id(name) {
            return Ok(id);
        }
        self.conn.execute("INSERT INTO tags(uid, name) VALUES(uid_bytes(?), ?)", params![new_uid(), name]).map_err(e)?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn add_tags(&self, ids: &[AssetId], names: &[String]) -> DbResult<()> {
        let mut tag_ids = Vec::new();
        for n in names {
            if !n.trim().is_empty() {
                tag_ids.push(self.get_or_create_tag(n)?);
            }
        }
        let mut st = self.conn.prepare_cached("INSERT OR IGNORE INTO asset_tags(tag_id, asset_id) VALUES(?, ?)").map_err(e)?;
        for id in ids {
            for t in &tag_ids {
                st.execute(params![t, id]).map_err(e)?;
            }
        }
        self.touch(ids)?;
        self.refresh_fts(ids)
    }

    pub fn remove_tag(&self, ids: &[AssetId], tag_id: i64) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("DELETE FROM asset_tags WHERE tag_id = ? AND asset_id = ?").map_err(e)?;
        for id in ids {
            st.execute(params![tag_id, id]).map_err(e)?;
        }
        self.touch(ids)?;
        self.refresh_fts(ids)
    }

    pub fn clear_tags(&self, ids: &[AssetId]) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("DELETE FROM asset_tags WHERE asset_id = ?").map_err(e)?;
        for id in ids {
            st.execute([id]).map_err(e)?;
        }
        self.touch(ids)?;
        self.refresh_fts(ids)
    }

    pub fn asset_tags(&self, id: AssetId) -> DbResult<Vec<Tag>> {
        let mut st = self
            .conn
            .prepare("SELECT t.id, t.name, t.group_id, t.color FROM asset_tags x JOIN tags t ON t.id = x.tag_id WHERE x.asset_id = ? ORDER BY t.name COLLATE NOCASE")
            .map_err(e)?;
        let rows = st
            .query_map([id], |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, group_id: r.get(2)?, color: r.get(3)?, group_ids: vec![], count: 0, starred: false }))
            .map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn tags(&self) -> DbResult<Vec<Tag>> {
        let mut st = self
            .conn
            .prepare(
                "SELECT t.id, t.name, t.group_id, t.color, (SELECT count(*) FROM asset_tags x JOIN assets a ON a.id = x.asset_id
                   WHERE x.tag_id = t.id AND a.deleted = 0), t.starred FROM tags t ORDER BY t.name COLLATE NOCASE",
            )
            .map_err(e)?;
        let rows = st
            .query_map([], |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, group_id: r.get(2)?, color: r.get(3)?, group_ids: vec![], count: r.get(4)?, starred: r.get::<_, i64>(5)? != 0 }))
            .map_err(e)?;
        let mut tags: Vec<Tag> = rows.filter_map(|r| r.ok()).collect();
        let mut members: std::collections::HashMap<i64, Vec<i64>> = Default::default();
        {
            let mut st = self.conn.prepare("SELECT m.tag_id, m.group_id FROM tag_group_members m JOIN tag_groups g ON g.id = m.group_id ORDER BY g.sort_order, g.name").map_err(e)?;
            let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))).map_err(e)?;
            for (t, g) in rows.flatten() {
                members.entry(t).or_default().push(g);
            }
        }
        for t in tags.iter_mut() {
            if let Some(list) = members.remove(&t.id) {
                t.group_ids = list;
            }
        }
        Ok(tags)
    }

    pub fn rename_tag(&mut self, id: i64, new_name: &str) -> DbResult<()> {
        let new_name = new_name.trim();
        if let Some(other) = self.tag_id(new_name) {
            if other != id {
                // merge into existing tag
                let tx = self.conn.savepoint().map_err(e)?;
                tx.execute("INSERT OR IGNORE INTO asset_tags(tag_id, asset_id) SELECT ?, asset_id FROM asset_tags WHERE tag_id = ?", params![other, id])
                    .map_err(e)?;
                tx.execute("DELETE FROM tags WHERE id = ?", [id]).map_err(e)?;
                tx.commit().map_err(e)?;
                let ids = self.ids_with_tag(other)?;
                return self.refresh_fts(&ids);
            }
        }
        self.conn.execute("UPDATE tags SET name = ? WHERE id = ?", params![new_name, id]).map_err(e)?;
        let ids = self.ids_with_tag(id)?;
        self.refresh_fts(&ids)
    }

    pub fn delete_tag(&self, id: i64) -> DbResult<()> {
        let ids = self.ids_with_tag(id)?;
        self.conn.execute("DELETE FROM tags WHERE id = ?", [id]).map_err(e)?;
        self.refresh_fts(&ids)
    }

    fn ids_with_tag(&self, tag_id: i64) -> DbResult<Vec<AssetId>> {
        let mut st = self.conn.prepare("SELECT asset_id FROM asset_tags WHERE tag_id = ?").map_err(e)?;
        let rows = st.query_map([tag_id], |r| r.get(0)).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Move: the tag leaves every other group.
    pub fn set_tag_group(&self, tag_id: i64, group: Option<i64>) -> DbResult<()> {
        self.conn.execute("DELETE FROM tag_group_members WHERE tag_id = ?", [tag_id]).map_err(e)?;
        if let Some(g) = group {
            self.conn.execute("INSERT OR IGNORE INTO tag_group_members(tag_id, group_id) VALUES(?, ?)", params![tag_id, g]).map_err(e)?;
        }
        self.conn.execute("UPDATE tags SET group_id = ? WHERE id = ?", params![group, tag_id]).map_err(e)?;
        Ok(())
    }

    /// Add: it stays in the groups it is already in.
    pub fn add_tag_to_group(&self, tag_id: i64, group: i64) -> DbResult<()> {
        self.conn.execute("INSERT OR IGNORE INTO tag_group_members(tag_id, group_id) VALUES(?, ?)", params![tag_id, group]).map_err(e)?;
        self.conn.execute("UPDATE tags SET group_id = COALESCE(group_id, ?) WHERE id = ?", params![group, tag_id]).map_err(e)?;
        Ok(())
    }

    pub fn remove_tag_from_group(&self, tag_id: i64, group: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM tag_group_members WHERE tag_id = ? AND group_id = ?", params![tag_id, group]).map_err(e)?;
        self.conn
            .execute("UPDATE tags SET group_id = (SELECT m.group_id FROM tag_group_members m WHERE m.tag_id = tags.id LIMIT 1) WHERE id = ?", [tag_id])
            .map_err(e)?;
        Ok(())
    }

    pub fn set_tag_group_description(&self, id: i64, text: &str) -> DbResult<()> {
        self.conn.execute("UPDATE tag_groups SET description = ? WHERE id = ?", params![text, id]).map_err(e)?;
        Ok(())
    }

    pub fn set_tag_group_order(&self, ids: &[i64]) -> DbResult<()> {
        for (i, id) in ids.iter().enumerate() {
            self.conn.execute("UPDATE tag_groups SET sort_order = ? WHERE id = ?", params![i as i64, id]).map_err(e)?;
        }
        Ok(())
    }

    /// Items carrying any tag of a group.
    pub fn tag_group_items(&self, group: i64) -> DbResult<Vec<AssetId>> {
        let mut st = self
            .conn
            .prepare("SELECT DISTINCT x.asset_id FROM asset_tags x JOIN tag_group_members m ON m.tag_id = x.tag_id JOIN assets a ON a.id = x.asset_id WHERE m.group_id = ? AND a.deleted = 0")
            .map_err(e)?;
        let rows = st.query_map([group], |r| r.get(0)).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Take items out of every folder.
    pub fn clear_folders(&self, ids: &[AssetId]) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("DELETE FROM asset_folders WHERE asset_id = ?").map_err(e)?;
        for id in ids {
            st.execute([id]).map_err(e)?;
        }
        Ok(())
    }

    pub fn set_rotation(&self, ids: &[AssetId], degrees: i64) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("UPDATE assets SET rotation = ? WHERE id = ?").map_err(e)?;
        for id in ids {
            st.execute(params![degrees.rem_euclid(360), id]).map_err(e)?;
        }
        Ok(())
    }

    /// The file itself changed shape or content: new size, new thumbnail and previews.
    pub fn file_rewritten(&self, id: AssetId, dims: Option<(i64, i64)>, size: i64, mtime: Option<i64>) -> DbResult<()> {
        if let Some((w, h)) = dims {
            self.conn.execute("UPDATE assets SET width = ?, height = ?, aspect = ? * 1.0 / ? WHERE id = ?", params![w, h, w, h.max(1), id]).map_err(e)?;
        } else {
            self.conn.execute("UPDATE assets SET width = NULL, height = NULL WHERE id = ?", [id]).map_err(e)?;
        }
        self.conn.execute("UPDATE assets SET size = ?, mtime = ?, rotation = 0, content_hash = NULL WHERE id = ?", params![size, mtime, id]).map_err(e)?;
        // a rewritten file is usually a new file under the old name; moves are matched by this
        if let Some(key) = self.abs_path(id).and_then(|p| filesystem::file_key(&p)) {
            self.conn.execute("UPDATE assets SET file_key = ? WHERE id = ?", params![key, id]).map_err(e)?;
        }
        self.invalidate_thumbs(&[id])
    }

    /// A new version number for a thumbnail about to be written by hand.
    pub fn next_thumb_version(&self, id: AssetId) -> DbResult<(String, i64, String)> {
        self.conn.execute("UPDATE assets SET thumb_ver = thumb_ver + 1 WHERE id = ?", [id]).map_err(e)?;
        self.conn.query_row("SELECT uid_text(uid), thumb_ver, ext FROM assets WHERE id = ?", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(e)
    }

    pub fn tag_groups(&self) -> DbResult<Vec<TagGroup>> {
        let mut st = self.conn.prepare("SELECT id, name, color, icon, description FROM tag_groups ORDER BY sort_order, name").map_err(e)?;
        let rows = st.query_map([], |r| Ok(TagGroup { id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, icon: r.get(3)?, description: r.get(4)? })).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn rename_tag_group(&self, id: i64, name: &str) -> DbResult<()> {
        self.conn.execute("UPDATE tag_groups SET name = ? WHERE id = ?", params![name, id]).map_err(e)?;
        Ok(())
    }

    pub fn set_tag_group_look(&self, id: i64, color: Option<&str>, icon: Option<&str>) -> DbResult<()> {
        self.conn.execute("UPDATE tag_groups SET color = ?, icon = ? WHERE id = ?", params![color, icon, id]).map_err(e)?;
        Ok(())
    }

    /// Its tags stay and become ungrouped.
    pub fn delete_tag_group(&self, id: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM tag_group_members WHERE group_id = ?", [id]).map_err(e)?;
        self.conn
            .execute("UPDATE tags SET group_id = (SELECT m.group_id FROM tag_group_members m WHERE m.tag_id = tags.id LIMIT 1) WHERE group_id = ?", [id])
            .map_err(e)?;
        self.conn.execute("DELETE FROM tag_groups WHERE id = ?", [id]).map_err(e)?;
        Ok(())
    }

    pub fn create_tag_group(&self, name: &str, color: Option<&str>) -> DbResult<i64> {
        self.conn.execute("INSERT INTO tag_groups(uid, name, color) VALUES(uid_bytes(?), ?, ?)", params![new_uid(), name, color]).map_err(e)?;
        Ok(self.conn.last_insert_rowid())
    }
}
