//! The readers a library knows and the queue of files waiting for them.

use crate::core::db::not_indexed;
use crate::core::db::ITEMS;
use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use std::collections::HashMap;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter};
use crate::core::model::*;
use crate::core::db::search::not_in_roots;

impl Db {
    /// Every reader known when plugins last loaded, on or off.
    pub fn known_readers(&self) -> Vec<crate::core::metadata::readers::ReaderInfo> {
        self.meta_get("readers").and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default()
    }

    pub fn set_known_readers(&self, readers: &[crate::core::metadata::readers::ReaderInfo]) -> DbResult<()> {
        let json = serde_json::to_string(readers).map_err(e)?;
        if self.meta_get("readers").as_deref() != Some(json.as_str()) {
            self.meta_set("readers", &json)?;
        }
        Ok(())
    }

    /// The known readers that fill this field, on or off; none for a field no reader fills.
    pub fn readers_of(&self, key: &str) -> Vec<crate::core::metadata::readers::ReaderInfo> {
        self.known_readers().into_iter().filter(|r| r.fills(key)).collect()
    }

    /// Queues the files each reader applies to (`scope`: only these items), to be read again in `mode`; `Missing` takes only files with no value of the reader's indexed fields. Answers how many files the readers have queued.
    pub fn queue_readers(&self, readers: &[crate::core::metadata::readers::ReaderInfo], scope: Option<&[AssetId]>, mode: crate::core::metadata::readers::Mode) -> DbResult<usize> {
        use crate::core::metadata::readers::Mode;
        let indexed: HashMap<String, (i64, bool)> = self.fields()?.into_iter().map(|f| (f.key.clone(), (f.id, f.indexed))).collect();
        let list = |n: usize| vec!["?"; n].join(", ");
        for r in readers {
            let ids: Vec<i64> = r.keys.iter().filter_map(|k| indexed.get(k)).filter(|(_, on)| *on).map(|(id, _)| *id).collect();
            if !r.keys.is_empty() && ids.is_empty() {
                continue;
            }
            let mut sql = format!("INSERT INTO meta_queue(asset_id, reader, mode) SELECT a.id, ?, ? FROM assets a WHERE a.deleted = 0 AND {ITEMS}");
            let mut params: Vec<Value> = vec![Value::Text(r.id.clone()), Value::Integer(mode as i64)];
            if !r.exts.is_empty() {
                sql += &format!(" AND a.ext IN ({})", list(r.exts.len()));
                params.extend(r.exts.iter().map(|x| Value::Text(x.clone())));
            }
            if mode == Mode::Missing && !ids.is_empty() {
                sql += &format!(" AND NOT EXISTS (SELECT 1 FROM field_values v WHERE v.asset_id = a.id AND v.field_id IN ({}))", list(ids.len()));
                params.extend(ids.iter().map(|i| Value::Integer(*i)));
            }
            let upsert = " ON CONFLICT(asset_id, reader) DO UPDATE SET mode = min(mode, excluded.mode)";
            match scope {
                None => {
                    self.conn.execute(&format!("{sql}{upsert}"), params_from_iter(params.iter())).map_err(e)?;
                }
                Some(scope) => {
                    for chunk in scope.chunks(500) {
                        let mut p = params.clone();
                        p.extend(chunk.iter().map(|i| Value::Integer(*i)));
                        self.conn.execute(&format!("{sql} AND a.id IN ({}){upsert}", list(chunk.len())), params_from_iter(p.iter())).map_err(e)?;
                    }
                }
            }
        }
        Ok(self.queue_left(&readers.iter().map(|r| r.id.clone()).collect::<Vec<_>>(), scope, false) as usize)
    }

    /// Files still queued for these readers, among `scope` when given; `here` counts only those that can be read now, not offline or missing ones.
    pub fn queue_left(&self, readers: &[String], scope: Option<&[AssetId]>, here: bool) -> i64 {
        if readers.is_empty() {
            return 0;
        }
        let list = |n: usize| vec!["?"; n].join(", ");
        let status = if here { " AND a.status = 0" } else { "" };
        let count = |ids: &[AssetId]| -> i64 {
            let mut p: Vec<Value> = readers.iter().map(|r| Value::Text(r.clone())).collect();
            let within = if ids.is_empty() { String::new() } else { format!(" AND q.asset_id IN ({})", list(ids.len())) };
            p.extend(ids.iter().map(|i| Value::Integer(*i)));
            self.conn.query_row(&format!("SELECT count(DISTINCT q.asset_id) FROM meta_queue q JOIN assets a ON a.id = q.asset_id WHERE q.reader IN ({}){within}{status}", list(readers.len())), params_from_iter(p.iter()), |r| r.get(0)).unwrap_or(0)
        };
        match scope {
            None => count(&[]),
            Some(s) => s.chunks(500).map(count).sum(),
        }
    }

    /// Takes these readers' files off the queue, among `scope` when given.
    pub fn unqueue(&self, readers: &[String], scope: Option<&[AssetId]>) -> DbResult<usize> {
        let list = |n: usize| vec!["?"; n].join(", ");
        let mut n = 0;
        for r in readers {
            n += match scope {
                None => self.conn.execute("DELETE FROM meta_queue WHERE reader = ?", [r]).map_err(e)?,
                Some(s) => {
                    let mut total = 0;
                    for chunk in s.chunks(500) {
                        let mut p: Vec<Value> = vec![Value::Text(r.clone())];
                        p.extend(chunk.iter().map(|i| Value::Integer(*i)));
                        total += self.conn.execute(&format!("DELETE FROM meta_queue WHERE reader = ? AND asset_id IN ({})", list(chunk.len())), params_from_iter(p.iter())).map_err(e)?;
                    }
                    total
                }
            };
        }
        Ok(n)
    }

    /// Queued files that can be read now, for readers that are on: (item, reader, mode), in item order, excluding `skip`. Files not read since they came or changed are read whole first.
    pub fn pending_queue(&self, readers: &[String], limit: usize, skip: &[(AssetId, String)]) -> DbResult<Vec<(AssetId, String, crate::core::metadata::readers::Mode)>> {
        if readers.is_empty() {
            return Ok(vec![]);
        }
        let skip = &skip[..skip.len().min(400)];
        let mut sql = format!("SELECT q.asset_id, q.reader, q.mode FROM meta_queue q JOIN assets a ON a.id = q.asset_id WHERE a.status = 0 AND a.meta_state = 1 AND {ITEMS} AND q.reader IN ({})", vec!["?"; readers.len()].join(", "));
        let mut p: Vec<Value> = readers.iter().map(|r| Value::Text(r.clone())).collect();
        if !skip.is_empty() {
            sql += &format!(" AND (q.asset_id, q.reader) NOT IN (VALUES {})", vec!["(?, ?)"; skip.len()].join(", "));
            for (id, r) in skip {
                p.push(Value::Integer(*id));
                p.push(Value::Text(r.clone()));
            }
        }
        sql += " ORDER BY q.asset_id LIMIT ?";
        p.push(Value::Integer(limit as i64));
        let mut st = self.conn.prepare(&sql).map_err(e)?;
        let rows = st.query_map(params_from_iter(p.iter()), |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, crate::core::metadata::readers::Mode::from_db(r.get(2)?)))).map_err(e)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(e)
    }

    /// What reading a file left: its queue rows for `done` (all of them after a whole read), the errors of the readers that ran, and whether it has been read whole.
    pub fn meta_read(&self, id: AssetId, done: &[String], ran: &[String], failed: &[(String, String)], whole: bool) -> DbResult<()> {
        if whole {
            self.conn.execute("DELETE FROM meta_queue WHERE asset_id = ?", [id]).map_err(e)?;
            self.set_meta_done(id)?;
        } else {
            let mut st = self.conn.prepare_cached("DELETE FROM meta_queue WHERE asset_id = ? AND reader = ?").map_err(e)?;
            for r in done {
                st.execute(params![id, r]).map_err(e)?;
            }
        }
        let mut del = self.conn.prepare_cached("DELETE FROM meta_errors WHERE asset_id = ? AND reader = ?").map_err(e)?;
        for r in ran {
            del.execute(params![id, r]).map_err(e)?;
        }
        let mut ins = self.conn.prepare_cached("INSERT INTO meta_errors(asset_id, reader, error, at) VALUES(?, ?, ?, ?) ON CONFLICT(asset_id, reader) DO UPDATE SET error = excluded.error, at = excluded.at").map_err(e)?;
        for (r, msg) in failed {
            ins.execute(params![id, r, msg.chars().take(1000).collect::<String>(), now_ms()]).map_err(e)?;
        }
        Ok(())
    }

    /// Queues one file for these readers, to be read in full.
    pub fn queue_item(&self, id: AssetId, readers: &[String]) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("INSERT INTO meta_queue(asset_id, reader, mode) VALUES(?, ?, 0) ON CONFLICT(asset_id, reader) DO UPDATE SET mode = 0").map_err(e)?;
        for r in readers {
            st.execute(params![id, r]).map_err(e)?;
        }
        Ok(())
    }

    pub fn is_queued(&self, id: AssetId, reader: &str) -> bool {
        self.conn.query_row("SELECT 1 FROM meta_queue WHERE asset_id = ? AND reader = ?", params![id, reader], |_| Ok(())).is_ok()
    }

    /// Curio's own readers whose version went up since this library was last read queue their files; the first time, the versions are only recorded. Answers how many files.
    pub fn requeue_changed_readers(&self, readers: &[crate::core::metadata::readers::Reader]) -> DbResult<usize> {
        let stored = self.meta_get("reader_versions");
        let seen: std::collections::BTreeMap<String, u32> = stored.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();
        // a plugin switched off keeps its readers' versions, and an older build never lowers a newer build's recorded version
        let mut current = seen.clone();
        for r in readers {
            let entry = current.entry(r.id.clone()).or_insert(0);
            if r.version > *entry {
                *entry = r.version;
            }
        }
        let json = serde_json::to_string(&current).map_err(e)?;
        if stored.is_none() {
            self.meta_set("reader_versions", &json)?;
            return Ok(0);
        }
        if current == seen {
            return Ok(0);
        }
        // a plugin's reader new to the library is read by switching its plugin on, not here
        let changed: Vec<crate::core::metadata::readers::ReaderInfo> = readers.iter().filter(|r| seen.get(&r.id).map_or(matches!(r.source, crate::core::metadata::readers::Source::Builtin(_)), |v| r.version > *v)).map(|r| r.info(true)).collect();
        let n = if changed.is_empty() { 0 } else { self.queue_readers(&changed, None, crate::core::metadata::readers::Mode::All)? };
        self.meta_set("reader_versions", &json)?;
        Ok(n)
    }

    /// The items holding a value of each field, in one pass: each holder has exactly one first value, found in the numbers' index without reading the texts.
    pub fn value_counts(&self) -> DbResult<HashMap<i64, i64>> {
        let mut st = self.conn.prepare("SELECT field_id, count(*) FROM field_values WHERE idx = 0 GROUP BY field_id").map_err(e)?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
        rows.collect::<Result<_, _>>().map_err(e)
    }

    /// Items not in the trash by extension; `unread`: only those not read since they came or changed.
    pub fn ext_counts(&self, unread: bool) -> DbResult<HashMap<String, i64>> {
        let sql = if unread { "SELECT ext, count(*) FROM assets WHERE deleted = 0 AND part_of IS NULL AND meta_state = 0 GROUP BY ext" } else { "SELECT ext, count(*) FROM assets WHERE deleted = 0 AND part_of IS NULL GROUP BY ext" };
        let mut st = self.conn.prepare(sql).map_err(e)?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
        rows.collect::<Result<_, _>>().map_err(e)
    }

    /// Files queued for each reader (`errors`: that each reader failed on).
    pub fn reader_counts(&self, errors: bool) -> DbResult<HashMap<String, i64>> {
        let sql = if errors { "SELECT reader, count(*) FROM meta_errors GROUP BY reader" } else { "SELECT reader, count(*) FROM meta_queue GROUP BY reader" };
        let mut st = self.conn.prepare(sql).map_err(e)?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
        rows.collect::<Result<_, _>>().map_err(e)
    }

    /// What readers failed on, newest first: (item, reader, message, when).
    pub fn meta_errors(&self, readers: &[String], limit: usize) -> DbResult<Vec<(AssetId, String, String, i64)>> {
        if readers.is_empty() {
            return Ok(vec![]);
        }
        let sql = format!("SELECT asset_id, reader, error, at FROM meta_errors WHERE reader IN ({}) ORDER BY at DESC LIMIT ?", vec!["?"; readers.len()].join(", "));
        let mut p: Vec<Value> = readers.iter().map(|r| Value::Text(r.clone())).collect();
        p.push(Value::Integer(limit as i64));
        let mut st = self.conn.prepare(&sql).map_err(e)?;
        let rows = st.query_map(params_from_iter(p.iter()), |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).map_err(e)?;
        rows.collect::<Result<_, _>>().map_err(e)
    }

    /// Stored values of a field that Clean up would remove: all but those set by hand.
    pub fn removable_values(&self, field_id: i64) -> i64 {
        self.conn.query_row("SELECT count(*) FROM field_values v WHERE v.field_id = ? AND NOT EXISTS (SELECT 1 FROM field_overrides o WHERE o.asset_id = v.asset_id AND o.field_id = v.field_id)", [field_id], |r| r.get(0)).unwrap_or(0)
    }

    /// Removes up to `limit` of those values; answers how many went.
    pub fn remove_values(&self, field_id: i64, limit: usize) -> DbResult<usize> {
        self.conn
            .execute(
                "DELETE FROM field_values WHERE (asset_id, field_id, idx) IN (SELECT v.asset_id, v.field_id, v.idx FROM field_values v WHERE v.field_id = ?1 AND NOT EXISTS (SELECT 1 FROM field_overrides o WHERE o.asset_id = v.asset_id AND o.field_id = v.field_id) LIMIT ?2)",
                params![field_id, limit as i64],
            )
            .map_err(e)
    }

    pub fn set_overridden(&self, ids: &[AssetId], field_id: i64, on: bool) -> DbResult<()> {
        let sql = if on { "INSERT OR IGNORE INTO field_overrides(asset_id, field_id) VALUES(?, ?)" } else { "DELETE FROM field_overrides WHERE asset_id = ? AND field_id = ?" };
        let mut st = self.conn.prepare_cached(sql).map_err(e)?;
        for id in ids {
            st.execute(params![id, field_id]).map_err(e)?;
        }
        Ok(())
    }

    /// A value set by anything but a reader (`None` clears it): refused for a field that isn't indexed, and marked on a field readers fill, so they leave it. Clearing one takes the file's value again.
    pub fn set_field_by_hand(&self, ids: &[AssetId], field: &FieldDef, raw: Option<&RawValue>) -> DbResult<()> {
        if !field.indexed {
            return Err(not_indexed(&field.key));
        }
        let readers = self.readers_of(&field.key);
        match raw {
            Some(v) => {
                self.set_field(ids, field, v)?;
                if !readers.is_empty() {
                    self.set_overridden(ids, field.id, true)?;
                }
            }
            None => {
                self.clear_field(ids, field.id)?;
                if !readers.is_empty() {
                    self.set_overridden(ids, field.id, false)?;
                    self.queue_readers(&readers, Some(ids), crate::core::metadata::readers::Mode::All)?;
                }
            }
        }
        Ok(())
    }

    /// Items no reader has read yet, but those in `skip_roots`.
    pub fn pending_meta(&self, limit: usize, skip_roots: &[i64]) -> DbResult<Vec<AssetId>> {
        let sql = format!("SELECT id FROM assets WHERE meta_state = 0 AND status = 0 AND part_of IS NULL{} ORDER BY id LIMIT ?", not_in_roots(skip_roots));
        let mut st = self.conn.prepare_cached(&sql).map_err(e)?;
        let rows = st.query_map([limit as i64], |r| r.get::<_, i64>(0)).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
