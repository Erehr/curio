//! Smart folders: saved queries in a tree of their own.

use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use rusqlite::types::Value;
use rusqlite::{params, OptionalExtension};
use crate::core::model::*;
use crate::core::query::{self, fields::FieldCatalog};

impl Db {
    pub fn create_smart_folder(&self, name: &str, query: &str) -> DbResult<i64> {
        self.create_smart_folder_in(name, query, None, None)
    }

    pub fn create_smart_folder_in(&self, name: &str, query: &str, parent: Option<i64>, group_by: Option<&str>) -> DbResult<i64> {
        let gb = group_by.map(str::trim).filter(|s| !s.is_empty());
        self.conn
            .execute(
                "INSERT INTO smart_folders(uid, parent_id, name, query, group_by, created_at, sort_order)
                 VALUES(uid_bytes(?1), ?2, ?3, ?4, ?5, ?6, (SELECT COALESCE(max(sort_order), 0) + 1 FROM smart_folders WHERE parent_id IS ?2))",
                params![new_uid(), parent, name, query, gb, now_ms()],
            )
            .map_err(e)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// The rules of a smart folder and of each folder above it, outermost first.
    pub fn smart_folder_rules(&self, id: i64) -> DbResult<Vec<String>> {
        let mut out = Vec::new();
        let mut next = Some(id);
        while let Some(cur) = next {
            let row: Option<(String, Option<i64>)> =
                self.conn.query_row("SELECT query, parent_id FROM smart_folders WHERE id = ?", [cur], |r| Ok((r.get(0)?, r.get(1)?))).optional().map_err(e)?;
            let Some((q, parent)) = row else { break };
            out.push(q);
            // a damaged chain that loops back ends here
            next = parent.filter(|p| *p != id && out.len() < 64);
        }
        out.reverse();
        Ok(out)
    }

    /// What a smart folder without rules finds: anything one of its subfolders finds, each by its own rules or, without them, by its own subfolders; nothing when it has none.
    pub(super) fn smart_group(&self, id: i64, cat: &FieldCatalog, depth: usize) -> DbResult<(String, Vec<Value>)> {
        let kids: Vec<(i64, String)> = {
            let mut st = self.conn.prepare_cached("SELECT id, query FROM smart_folders WHERE parent_id = ? AND id <> ? ORDER BY sort_order").map_err(e)?;
            let rows = st.query_map(params![id, id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
            rows.collect::<Result<_, _>>().map_err(e)?
        };
        let mut parts = Vec::new();
        let mut params = Vec::new();
        // a damaged tree that loops back ends here
        if depth < 64 {
            for (kid, q) in kids {
                let (sql, ps) = if q.trim().is_empty() { self.smart_group(kid, cat, depth + 1)? } else { query::compile::compile_str(&q, cat).map(|c| (c.sql, c.params))? };
                if sql != "0" {
                    parts.push(format!("({sql})"));
                    params.extend(ps);
                }
            }
        }
        Ok(if parts.is_empty() { ("0".into(), vec![]) } else { (format!("({})", parts.join(" OR ")), params) })
    }

    /// Put a smart folder under `parent`, just before `before` (or last). Siblings are renumbered.
    pub fn place_smart_folder(&self, id: i64, parent: Option<i64>, before: Option<i64>) -> DbResult<()> {
        let mut p = parent;
        while let Some(pid) = p {
            if pid == id {
                return Err("A smart folder can't be moved into itself".into());
            }
            p = self.conn.query_row("SELECT parent_id FROM smart_folders WHERE id = ?", [pid], |r| r.get(0)).optional().map_err(e)?.flatten();
        }
        self.conn.execute("UPDATE smart_folders SET parent_id = ? WHERE id = ?", params![parent, id]).map_err(e)?;
        let mut list: Vec<i64> = {
            let mut st = self.conn.prepare("SELECT id FROM smart_folders WHERE parent_id IS ? AND id <> ? ORDER BY sort_order, name COLLATE NOCASE").map_err(e)?;
            let rows = st.query_map(params![parent, id], |r| r.get(0)).map_err(e)?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let at = before.and_then(|b| list.iter().position(|x| *x == b)).unwrap_or(list.len());
        list.insert(at, id);
        for (i, f) in list.iter().enumerate() {
            self.conn.execute("UPDATE smart_folders SET sort_order = ? WHERE id = ?", params![i as i64 + 1, f]).map_err(e)?;
        }
        Ok(())
    }

    pub fn update_smart_folder(&self, id: i64, name: &str, query: &str, group_by: Option<&str>) -> DbResult<()> {
        let gb = group_by.map(str::trim).filter(|s| !s.is_empty());
        self.conn.execute("UPDATE smart_folders SET name = ?, query = ?, group_by = ? WHERE id = ?", params![name, query, gb, id]).map_err(e)?;
        Ok(())
    }

    /// Delete a smart folder and its subfolders.
    pub fn delete_smart_folder(&self, id: i64) -> DbResult<()> {
        self.conn
            .execute(
                "WITH RECURSIVE sub(id) AS (SELECT ?1 UNION SELECT s.id FROM smart_folders s JOIN sub ON s.parent_id = sub.id)
                 DELETE FROM smart_folders WHERE id IN (SELECT id FROM sub)",
                [id],
            )
            .map_err(e)?;
        Ok(())
    }

    /// The smart folders with how many items each finds, for the sidebar.
    pub fn smart_folders_counted(&self) -> DbResult<Vec<SmartFolder>> {
        let cat = self.field_catalog();
        let mut list = self.smart_folders()?;
        for f in &mut list {
            f.count = self.search_count(&Scope::SmartFolder(f.id), "", false, &cat).ok();
        }
        Ok(list)
    }

    pub fn smart_folders(&self) -> DbResult<Vec<SmartFolder>> {
        let mut st = self
            .conn
            .prepare("SELECT id, name, query, sort, icon, group_by, parent_id, sort_order FROM smart_folders ORDER BY sort_order, name COLLATE NOCASE")
            .map_err(e)?;
        let rows = st
            .query_map([], |r| {
                Ok(SmartFolder { id: r.get(0)?, name: r.get(1)?, query: r.get(2)?, sort: r.get(3)?, icon: r.get(4)?, group_by: r.get(5)?, parent_id: r.get(6)?, sort_order: r.get(7)?, count: None })
            })
            .map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn smart_group_by(&self, id: i64) -> DbResult<Option<String>> {
        self.conn.query_row("SELECT group_by FROM smart_folders WHERE id = ?", [id], |r| r.get::<_, Option<String>>(0)).optional().map_err(e).map(|o| o.flatten().filter(|s| !s.is_empty()))
    }

    /// Virtual subfolders of a smart folder with `group_by` set, computed live from its matching items.
    pub fn smart_buckets(&self, id: i64) -> DbResult<Vec<SmartBucket>> {
        const MAX_BUCKETS: i64 = 200;
        let Some(gb) = self.smart_group_by(id)? else { return Ok(vec![]) };
        let cat = self.field_catalog();
        let plan = self.search_plan(&Scope::SmartFolder(id), "", &Sort::default(), false, &cat)?;
        let from = format!("FROM assets a {} WHERE ({})", plan.joins, plan.filter);
        let date_buckets = |col: &str, fmt: &str| -> DbResult<Vec<SmartBucket>> {
            let sql = format!("SELECT strftime('{fmt}', a.{col} / 1000, 'unixepoch', 'localtime') AS k, count(*) {from} AND a.{col} IS NOT NULL GROUP BY k HAVING k IS NOT NULL ORDER BY k DESC LIMIT {MAX_BUCKETS}");
            let mut st = self.conn.prepare(&sql).map_err(e)?;
            let rows = st.query_map(rusqlite::params_from_iter(plan.params.iter()), |r| {
                let v: String = r.get(0)?;
                Ok(SmartBucket { value: v.clone(), label: v, count: r.get(1)? })
            }).map_err(e)?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        };
        match gb.as_str() {
            "year:added" => date_buckets("added_at", "%Y"),
            "month:added" => date_buckets("added_at", "%Y-%m"),
            "year:created" => date_buckets("ctime", "%Y"),
            "month:created" => date_buckets("ctime", "%Y-%m"),
            "year:modified" => date_buckets("mtime", "%Y"),
            "month:modified" => date_buckets("mtime", "%Y-%m"),
            "rating" => {
                let mut st = self.conn.prepare(&format!("SELECT a.rating, count(*) {from} GROUP BY a.rating ORDER BY a.rating DESC")).map_err(e)?;
                let rows = st.query_map(rusqlite::params_from_iter(plan.params.iter()), |r| {
                    let n: i64 = r.get(0)?;
                    Ok(SmartBucket { value: n.to_string(), label: n.to_string(), count: r.get(1)? })
                }).map_err(e)?;
                Ok(rows.filter_map(|r| r.ok()).collect())
            }
            "kind" => {
                let mut st = self.conn.prepare(&format!("SELECT a.kind, count(*) {from} GROUP BY a.kind ORDER BY 2 DESC, 1 ASC")).map_err(e)?;
                let rows = st.query_map(rusqlite::params_from_iter(plan.params.iter()), |r| {
                    let v: String = r.get(0)?;
                    Ok(SmartBucket { value: v.clone(), label: v, count: r.get(1)? })
                }).map_err(e)?;
                Ok(rows.filter_map(|r| r.ok()).collect())
            }
            "ext" => {
                let mut st = self.conn.prepare(&format!("SELECT a.ext, count(*) {from} AND a.ext <> '' GROUP BY a.ext ORDER BY 2 DESC, 1 ASC LIMIT {MAX_BUCKETS}")).map_err(e)?;
                let rows = st.query_map(rusqlite::params_from_iter(plan.params.iter()), |r| {
                    let v: String = r.get(0)?;
                    Ok(SmartBucket { value: v.clone(), label: v.to_uppercase(), count: r.get(1)? })
                }).map_err(e)?;
                Ok(rows.filter_map(|r| r.ok()).collect())
            }
            "tag" => {
                let mut st = self.conn.prepare(&format!("SELECT t.name, count(DISTINCT x.asset_id) FROM assets a {} JOIN asset_tags x ON x.asset_id = a.id JOIN tags t ON t.id = x.tag_id WHERE ({}) GROUP BY t.id ORDER BY 2 DESC, t.name COLLATE NOCASE LIMIT {MAX_BUCKETS}", plan.joins, plan.filter)).map_err(e)?;
                let rows = st.query_map(rusqlite::params_from_iter(plan.params.iter()), |r| {
                    let v: String = r.get(0)?;
                    Ok(SmartBucket { value: v.clone(), label: v, count: r.get(1)? })
                }).map_err(e)?;
                Ok(rows.filter_map(|r| r.ok()).collect())
            }
            other => {
                let Some(key) = other.strip_prefix("field:") else { return Ok(vec![]) };
                let Some(f) = cat.by_key.get(key) else { return Ok(vec![]) };
                let mut params = plan.params.clone();
                params.push(Value::Integer(f.id));
                let mut st = self.conn.prepare(&format!(
                    "SELECT CASE WHEN fv.text_value IS NOT NULL THEN fv.text_value ELSE CAST(fv.num_value AS TEXT) END AS k, count(DISTINCT fv.asset_id)
                     FROM assets a {} JOIN field_values fv ON fv.asset_id = a.id
                     WHERE ({}) AND fv.field_id = ? AND (COALESCE(fv.text_value, '') <> '' OR fv.num_value IS NOT NULL)
                     GROUP BY k ORDER BY 2 DESC, k COLLATE NOCASE LIMIT {MAX_BUCKETS}",
                    plan.joins, plan.filter
                )).map_err(e)?;
                let rows = st.query_map(rusqlite::params_from_iter(params.iter()), |r| {
                    let v: String = r.get(0)?;
                    let label = if f.ftype == FieldType::Integer || f.ftype == FieldType::Float {
                        v.strip_suffix(".0").unwrap_or(&v).to_string()
                    } else {
                        v.clone()
                    };
                    Ok(SmartBucket { value: v, label, count: r.get(1)? })
                }).map_err(e)?;
                Ok(rows.filter_map(|r| r.ok()).collect())
            }
        }
    }

    pub(super) fn smart_bucket_cond(&self, group_by: &str, value: &str, cat: &FieldCatalog) -> DbResult<(String, Vec<Value>)> {
        let date_cond = |col: &str, fmt: &str| Ok((format!("strftime('{fmt}', a.{col} / 1000, 'unixepoch', 'localtime') = ?"), vec![Value::Text(value.into())]));
        match group_by {
            "year:added" => date_cond("added_at", "%Y"),
            "month:added" => date_cond("added_at", "%Y-%m"),
            "year:created" => date_cond("ctime", "%Y"),
            "month:created" => date_cond("ctime", "%Y-%m"),
            "year:modified" => date_cond("mtime", "%Y"),
            "month:modified" => date_cond("mtime", "%Y-%m"),
            "rating" => Ok(("a.rating = ?".into(), vec![Value::Integer(value.parse::<i64>().unwrap_or(-1))])),
            "kind" => Ok(("a.kind = ?".into(), vec![Value::Text(value.into())])),
            "ext" => Ok(("a.ext = ?".into(), vec![Value::Text(value.into())])),
            "tag" => Ok(("EXISTS (SELECT 1 FROM asset_tags bt JOIN tags t ON t.id = bt.tag_id WHERE bt.asset_id = a.id AND t.name = ?)".into(), vec![Value::Text(value.into())])),
            other => {
                let Some(key) = other.strip_prefix("field:") else { return Ok(("0".into(), vec![])) };
                let Some(f) = cat.by_key.get(key) else { return Ok(("0".into(), vec![])) };
                if let Ok(n) = value.parse::<f64>() {
                    Ok(("EXISTS (SELECT 1 FROM field_values bv WHERE bv.asset_id = a.id AND bv.field_id = ? AND (bv.text_value = ? OR (bv.text_value IS NULL AND bv.num_value = ?)))".into(), vec![Value::Integer(f.id), Value::Text(value.into()), Value::Real(n)]))
                } else {
                    Ok(("EXISTS (SELECT 1 FROM field_values bv WHERE bv.asset_id = a.id AND bv.field_id = ? AND bv.text_value = ?)".into(), vec![Value::Integer(f.id), Value::Text(value.into())]))
                }
            }
        }
    }
}
