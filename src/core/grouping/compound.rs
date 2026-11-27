//! Files grouped into one item: grouping takes what a file had as an item into its new item, splitting gives it back.

use std::collections::HashSet;

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::core::db::{Db, DbResult};
use crate::core::model::{AssetId, Availability};

fn e<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

/// All or nothing, inside a transaction or not.
fn atomic<T>(db: &Db, f: impl FnOnce(&Db) -> DbResult<T>) -> DbResult<T> {
    db.conn.execute_batch("SAVEPOINT compound").map_err(e)?;
    let res = f(db).and_then(|v| db.conn.execute_batch("RELEASE compound").map(|_| v).map_err(e));
    if res.is_err() {
        let _ = db.conn.execute_batch("ROLLBACK TO compound; RELEASE compound");
    }
    res
}

/// One file grouped into an item.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub id: AssetId,
    pub uid: String,
    pub role: String,
    pub rule: String,
    pub name: String,
    pub ext: String,
    pub size: i64,
    pub status: Availability,
    pub suffix: Option<String>,
    pub rel_path: String,
    /// its item's thumbnail is drawn from it
    pub cover: bool,
}

/// What a file had as an item of its own, put back when it is split off.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Saved {
    tags: Vec<String>,
    folders: Vec<(i64, f64)>,
    trashed: Vec<(i64, f64, i64)>,
    rating: i64,
    annotation: String,
    url: String,
    display_name: String,
    /// field, idx, text, number, set by hand
    values: Vec<(i64, i64, Option<String>, Option<f64>, bool)>,
}

/// Why two files can't be grouped, or `None` when they can.
fn refusal(db: &Db, item: AssetId, member: AssetId, rule: &str) -> DbResult<Option<String>> {
    if item == member {
        return Ok(Some("a file can't be grouped into itself".into()));
    }
    let row = |id: AssetId| -> DbResult<Option<(i64, Option<i64>, bool, bool, i64, bool)>> {
        db.conn
            .query_row(
                "SELECT a.root_id, a.part_of, a.solo, a.deleted = 1 OR EXISTS (SELECT 1 FROM trashed_entries t WHERE t.asset_id = a.id), a.status, EXISTS (SELECT 1 FROM assets m WHERE m.part_of = a.id) FROM assets a WHERE a.id = ?",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .optional()
            .map_err(e)
    };
    let (Some(i), Some(m)) = (row(item)?, row(member)?) else { return Ok(Some("no such item".into())) };
    Ok(if (i.2 || m.2) && rule != "user" {
        Some("it was split off by hand".into())
    } else if m.5 {
        Some("it has files of its own grouped into it".into())
    } else if m.1.is_some() {
        Some("it already belongs to an item".into())
    } else if i.1.is_some() {
        Some("the item is itself grouped into another".into())
    } else if i.0 != m.0 {
        Some("they are in different locations".into())
    } else if i.3 || m.3 {
        Some("one of them is in the trash".into())
    } else if i.4 != 0 || m.4 != 0 {
        Some("one of them is missing or offline".into())
    } else {
        None
    })
}

impl Db {
    /// Group `member` into `item`, in one transaction: the member's tags, folders, rating, notes, address and the values no reader fills go to the item where it has none, and are kept on the member for a split.
    pub fn group(&self, item: AssetId, member: AssetId, role: &str, rule: &str, suffix: Option<&str>, cover: bool) -> DbResult<()> {
        if let Some(why) = refusal(self, item, member, rule)? {
            return Err(format!("Can't group these files: {why}"));
        }
        atomic(self, |db| {
            let read: HashSet<String> = db.known_readers().into_iter().flat_map(|r| r.keys).collect();
            let saved = db.saved_of(member, &read)?;
            // the item takes what it lacks
            db.add_tags(&[item], &saved.tags)?;
            for (f, pos) in &saved.folders {
                db.conn.execute("INSERT OR IGNORE INTO asset_folders(folder_id, asset_id, position) VALUES(?, ?, ?)", params![f, item, pos]).map_err(e)?;
            }
            db.conn
                .execute(
                    "UPDATE assets SET rating = CASE WHEN rating = 0 THEN ?2 ELSE rating END, annotation = CASE WHEN annotation = '' THEN ?3 ELSE annotation END, source_url = CASE WHEN source_url = '' THEN ?4 ELSE source_url END WHERE id = ?1",
                    params![item, saved.rating, saved.annotation, saved.url],
                )
                .map_err(e)?;
            let held = db.fields_with_values(item);
            for (f, idx, text, num, by_hand) in saved.values.iter().filter(|v| !held.contains(&v.0)) {
                db.conn.execute("INSERT OR IGNORE INTO field_values(asset_id, field_id, idx, text_value, num_value) VALUES(?, ?, ?, ?, ?)", params![item, f, idx, text, num]).map_err(e)?;
                if *by_hand {
                    db.conn.execute("INSERT OR IGNORE INTO field_overrides(asset_id, field_id) VALUES(?, ?)", params![item, f]).map_err(e)?;
                }
            }
            let json = serde_json::to_string(&saved).map_err(e)?;
            db.conn
                .execute_batch(&format!(
                    "DELETE FROM asset_tags WHERE asset_id = {member};
                     DELETE FROM asset_folders WHERE asset_id = {member};
                     DELETE FROM trashed_entries WHERE asset_id = {member};
                     DELETE FROM trashed_folders WHERE asset_id = {member};
                     DELETE FROM field_overrides WHERE asset_id = {member};
                     DELETE FROM field_values WHERE asset_id = {member};
                     DELETE FROM meta_queue WHERE asset_id = {member};
                     DELETE FROM meta_errors WHERE asset_id = {member};
                     DELETE FROM asset_fts WHERE rowid = {member};"
                ))
                .map_err(e)?;
            db.conn
                .execute(
                    "UPDATE assets SET part_of = ?2, part_role = ?3, part_rule = ?4, part_suffix = ?5, part_saved = ?6, part_cover = ?7, meta_state = 1, rating = 0, annotation = '', source_url = '', display_name = '', deleted = (SELECT deleted FROM assets WHERE id = ?2) WHERE id = ?1",
                    params![member, item, role, rule, suffix, json, cover],
                )
                .map_err(e)?;
            let thumb = if cover { ", thumb_state = CASE WHEN thumb_state = 0 THEN 0 ELSE 3 END, thumb_ver = thumb_ver + 1" } else { "" };
            db.conn.execute(&format!("UPDATE assets SET meta_state = 0{thumb} WHERE id = ?"), [item]).map_err(e)?;
            db.refresh_fts(&[item])?;
            db.mark_store_dirty(&[item, member])
        })
    }

    fn saved_of(&self, id: AssetId, read: &HashSet<String>) -> DbResult<Saved> {
        let list = |sql: &str| -> DbResult<Vec<String>> {
            let mut st = self.conn.prepare_cached(sql).map_err(e)?;
            let rows = st.query_map([id], |r| r.get(0)).map_err(e)?;
            rows.collect::<Result<_, _>>().map_err(e)
        };
        let tags = list("SELECT t.name FROM asset_tags x JOIN tags t ON t.id = x.tag_id WHERE x.asset_id = ? ORDER BY t.name")?;
        let folders = {
            let mut st = self.conn.prepare_cached("SELECT folder_id, position FROM asset_folders WHERE asset_id = ? ORDER BY folder_id").map_err(e)?;
            let rows = st.query_map([id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
            rows.collect::<Result<_, _>>().map_err(e)?
        };
        let trashed = {
            let mut st = self.conn.prepare_cached("SELECT folder_id, position, trashed_at FROM trashed_entries WHERE asset_id = ? ORDER BY folder_id").map_err(e)?;
            let rows = st.query_map([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(e)?;
            rows.collect::<Result<_, _>>().map_err(e)?
        };
        let (rating, annotation, url, display_name) = self
            .conn
            .query_row("SELECT rating, annotation, source_url, display_name FROM assets WHERE id = ?", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map_err(e)?;
        let by_hand = self.overridden_ids(id);
        let values = {
            let mut st = self.conn.prepare_cached("SELECT v.field_id, v.idx, v.text_value, v.num_value, f.key FROM field_values v JOIN fields f ON f.id = v.field_id WHERE v.asset_id = ? ORDER BY v.field_id, v.idx").map_err(e)?;
            let rows = st.query_map([id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, Option<f64>>(3)?, r.get::<_, String>(4)?))).map_err(e)?;
            rows.filter_map(|r| r.ok()).filter(|v| !read.contains(&v.4) || by_hand.contains(&v.0)).map(|v| (v.0, v.1, v.2, v.3, by_hand.contains(&v.0))).collect()
        };
        Ok(Saved { tags, folders, trashed, rating, annotation, url, display_name, values })
    }

    /// Give a member back what it had as an item, and the folders its item is in now, so its files stay where they are. `solo`: split off by hand, so rules never group it again.
    fn release(&self, member: AssetId, solo: bool) -> DbResult<()> {
        let (item, json): (Option<AssetId>, Option<String>) = self.conn.query_row("SELECT part_of, part_saved FROM assets WHERE id = ?", [member], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
        let saved: Saved = json.and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default();
        let exists = |table: &str, id: i64| self.conn.query_row(&format!("SELECT EXISTS (SELECT 1 FROM {table} WHERE id = ?)"), [id], |r| r.get::<_, bool>(0)).unwrap_or(false);
        self.add_tags(&[member], &saved.tags)?;
        for (f, pos) in saved.folders.iter().filter(|(f, _)| exists("folders", *f)) {
            self.conn.execute("INSERT OR IGNORE INTO asset_folders(folder_id, asset_id, position) VALUES(?, ?, ?)", params![f, member, pos]).map_err(e)?;
        }
        if let Some(item) = item {
            self.conn.execute("INSERT OR IGNORE INTO asset_folders(folder_id, asset_id, position) SELECT folder_id, ?2, position FROM asset_folders WHERE asset_id = ?1", params![item, member]).map_err(e)?;
            self.conn.execute("INSERT OR IGNORE INTO trashed_entries(asset_id, folder_id, position, trashed_at) SELECT ?2, folder_id, position, trashed_at FROM trashed_entries WHERE asset_id = ?1", params![item, member]).map_err(e)?;
        }
        for (f, pos, at) in saved.trashed.iter().filter(|(f, ..)| exists("folders", *f)) {
            self.conn.execute("INSERT OR IGNORE INTO trashed_entries(asset_id, folder_id, position, trashed_at) VALUES(?, ?, ?, ?)", params![member, f, pos, at]).map_err(e)?;
        }
        for (f, idx, text, num, by_hand) in saved.values.iter().filter(|v| exists("fields", v.0)) {
            self.conn.execute("INSERT INTO field_values(asset_id, field_id, idx, text_value, num_value) VALUES(?, ?, ?, ?, ?) ON CONFLICT(asset_id, field_id, idx) DO UPDATE SET text_value = excluded.text_value, num_value = excluded.num_value", params![member, f, idx, text, num]).map_err(e)?;
            if *by_hand {
                self.conn.execute("INSERT OR IGNORE INTO field_overrides(asset_id, field_id) VALUES(?, ?)", params![member, f]).map_err(e)?;
            }
        }
        // the item was drawn from this file
        if let Some(item) = item {
            self.conn.execute("UPDATE assets SET thumb_state = CASE WHEN thumb_state = 0 THEN 0 ELSE 3 END, thumb_ver = thumb_ver + 1 WHERE id = ?1 AND EXISTS (SELECT 1 FROM assets WHERE id = ?2 AND part_cover = 1)", params![item, member]).map_err(e)?;
        }
        self.conn
            .execute(
                "UPDATE assets SET part_of = NULL, part_role = NULL, part_rule = NULL, part_suffix = NULL, part_saved = NULL, part_cover = 0, solo = MAX(solo, ?6), meta_state = 0, rating = ?2, annotation = ?3, source_url = ?4, display_name = ?5 WHERE id = ?1",
                params![member, saved.rating, saved.annotation, saved.url, saved.display_name, solo],
            )
            .map_err(e)?;
        self.refresh_fts(&[member])?;
        self.mark_store_dirty(&[member])
    }

    /// Split an item into its files, each an item again. Rules never group them again. Returns the files split off.
    pub fn split(&self, item: AssetId) -> DbResult<Vec<AssetId>> {
        atomic(self, |db| {
            let members: Vec<AssetId> = db.members_of(item)?.into_iter().map(|p| p.id).collect();
            for m in &members {
                db.release(*m, true)?;
            }
            db.conn.execute("UPDATE assets SET solo = 1, meta_state = 0 WHERE id = ?", [item]).map_err(e)?;
            db.mark_store_dirty(&[item])?;
            Ok(members)
        })
    }

    /// Take one file out of its item, an item again. Returns its former item.
    pub fn detach(&self, member: AssetId) -> DbResult<AssetId> {
        atomic(self, |db| {
            let item: AssetId = db.conn.query_row("SELECT part_of FROM assets WHERE id = ?", [member], |r| r.get::<_, Option<AssetId>>(0)).map_err(e)?.ok_or("That file isn't grouped into an item")?;
            db.release(member, true)?;
            db.conn.execute("UPDATE assets SET meta_state = 0 WHERE id = ?", [item]).map_err(e)?;
            db.mark_store_dirty(&[item])?;
            Ok(item)
        })
    }

    /// Split off every file a rule grouped, each an item again; what other rules grouped into the same items stays. Nothing here was split by hand, so rules may group these files again. Returns the items and the files split off.
    pub fn split_rule(&self, rule: &str) -> DbResult<(Vec<AssetId>, Vec<AssetId>)> {
        atomic(self, |db| {
            let rows: Vec<(AssetId, AssetId)> = {
                let mut st = db.conn.prepare("SELECT part_of, id FROM assets WHERE part_rule = ? AND part_of IS NOT NULL ORDER BY part_of, id").map_err(e)?;
                let q = st.query_map([rule], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
                q.collect::<Result<_, _>>().map_err(e)?
            };
            let mut items: Vec<AssetId> = rows.iter().map(|r| r.0).collect();
            items.dedup();
            for (_, m) in &rows {
                db.release(*m, false)?;
            }
            for item in &items {
                db.conn.execute("UPDATE assets SET meta_state = 0 WHERE id = ?", [item]).map_err(e)?;
            }
            db.mark_store_dirty(&items)?;
            Ok((items, rows.into_iter().map(|r| r.1).collect()))
        })
    }

    /// Rules may group a file split off by hand again.
    pub fn allow_grouping(&self, ids: &[AssetId]) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("UPDATE assets SET solo = 0 WHERE id = ?").map_err(e)?;
        for id in ids {
            st.execute([id]).map_err(e)?;
        }
        Ok(())
    }

    /// The files grouped into `item`, in the order they were grouped.
    pub fn members_of(&self, item: AssetId) -> DbResult<Vec<Part>> {
        let mut st = self
            .conn
            .prepare_cached("SELECT id, uid_text(uid), COALESCE(part_role, ''), COALESCE(part_rule, ''), name, ext, size, status, part_suffix, rel_path, part_cover FROM assets WHERE part_of = ? ORDER BY id")
            .map_err(e)?;
        let rows = st
            .query_map([item], |r| {
                Ok(Part { id: r.get(0)?, uid: r.get(1)?, role: r.get(2)?, rule: r.get(3)?, name: r.get(4)?, ext: r.get(5)?, size: r.get(6)?, status: Availability::from_i64(r.get(7)?), suffix: r.get(8)?, rel_path: r.get(9)?, cover: r.get(10)? })
            })
            .map_err(e)?;
        rows.collect::<Result<_, _>>().map_err(e)
    }

    /// `ids` and the files grouped into them.
    pub fn with_members(&self, ids: &[AssetId]) -> DbResult<Vec<AssetId>> {
        let mut out: Vec<AssetId> = ids.to_vec();
        let mut seen: HashSet<AssetId> = ids.iter().copied().collect();
        let mut st = self.conn.prepare_cached("SELECT id FROM assets WHERE part_of = ? ORDER BY id").map_err(e)?;
        for id in ids {
            let rows = st.query_map([id], |r| r.get::<_, AssetId>(0)).map_err(e)?;
            for m in rows.filter_map(|r| r.ok()) {
                if seen.insert(m) {
                    out.push(m);
                }
            }
        }
        Ok(out)
    }

    /// The items of `ids`: a grouped file stands for its item. Order kept, each once.
    pub fn items_of(&self, ids: &[AssetId]) -> DbResult<Vec<AssetId>> {
        let mut st = self.conn.prepare_cached("SELECT COALESCE(part_of, id) FROM assets WHERE id = ?").map_err(e)?;
        let mut seen = HashSet::new();
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let item = st.query_row([id], |r| r.get::<_, AssetId>(0)).optional().map_err(e)?.unwrap_or(*id);
            if seen.insert(item) {
                out.push(item);
            }
        }
        Ok(out)
    }
}
