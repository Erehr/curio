//! Custom fields: their definitions and their typed values.

use crate::core::db::e;
use crate::core::db::Db;
use crate::core::db::DbResult;
use std::collections::HashSet;
use rusqlite::{params, params_from_iter, OptionalExtension};
use crate::core::model::*;
use crate::core::query::{self, fields::FieldCatalog};

/// Convert a raw value into stored (text, num) pairs according to the field type and validation.
pub fn typed_values(f: &FieldDef, raw: &RawValue) -> DbResult<Vec<(Option<String>, Option<f64>)>> {
    if matches!(f.ftype, FieldType::Unknown(_)) {
        return Err(format!("{}: read-only in this version of Curio", f.label));
    }
    let items: Vec<RawValue> = match raw {
        RawValue::List(v) if f.ftype == FieldType::MultiSelect => v.clone(),
        RawValue::Text(s) if f.ftype == FieldType::MultiSelect => {
            s.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()).map(|x| RawValue::Text(x.to_string())).collect()
        }
        RawValue::List(v) => v.clone(),
        other => vec![other.clone()],
    };
    let v = &f.validation;
    let mut out = Vec::new();
    for item in items {
        let pair: (Option<String>, Option<f64>) = match &f.ftype {
            FieldType::Integer | FieldType::Float => {
                let n = match &item {
                    RawValue::Num(n) => *n,
                    RawValue::Bool(b) => *b as i64 as f64,
                    RawValue::Text(s) => {
                        let t = s.trim();
                        if t.is_empty() {
                            continue;
                        }
                        t.parse::<f64>().map_err(|_| format!("{}: '{t}' is not a number", f.label))?
                    }
                    RawValue::List(_) => continue,
                };
                if !n.is_finite() {
                    return Err(format!("{}: not a valid number", f.label));
                }
                let n = if f.ftype == FieldType::Integer { n.round() } else { n };
                if let Some(min) = v.min {
                    if n < min {
                        return Err(format!("{}: {n} is below the minimum {min}", f.label));
                    }
                }
                if let Some(max) = v.max {
                    if n > max {
                        return Err(format!("{}: {n} is above the maximum {max}", f.label));
                    }
                }
                (None, Some(n))
            }
            FieldType::Boolean => {
                let b = match &item {
                    RawValue::Bool(b) => *b,
                    RawValue::Num(n) => *n != 0.0,
                    RawValue::Text(s) => query::compile::parse_bool(s).map_err(|_| format!("{}: '{s}' is not true/false", f.label))?,
                    RawValue::List(_) => continue,
                };
                (None, Some(if b { 1.0 } else { 0.0 }))
            }
            FieldType::Date | FieldType::DateTime => {
                let (t, n) = match &item {
                    RawValue::Num(n) => (None, *n),
                    RawValue::Text(s) => {
                        if s.trim().is_empty() {
                            continue;
                        }
                        let n = query::compile::parse_date_ms(s).ok_or(format!("{}: '{s}' is not a date (YYYY-MM-DD)", f.label))?;
                        (Some(s.trim().to_string()), n)
                    }
                    _ => continue,
                };
                (t, Some(n))
            }
            _ => {
                let s = match &item {
                    RawValue::Text(s) => s.clone(),
                    RawValue::Num(n) => format!("{n}"),
                    RawValue::Bool(b) => b.to_string(),
                    RawValue::List(_) => continue,
                };
                if f.ftype == FieldType::Json && !s.trim().is_empty() && serde_json::from_str::<serde_json::Value>(&s).is_err() {
                    return Err(format!("{}: not valid JSON", f.label));
                }
                if matches!(f.ftype, FieldType::SingleSelect | FieldType::MultiSelect)
                    && !f.options.is_empty()
                    && !f.options.iter().any(|o| o.eq_ignore_ascii_case(&s))
                {
                    return Err(format!("{}: '{s}' is not one of {}", f.label, f.options.join(", ")));
                }
                if let Some(ml) = v.max_length {
                    if s.chars().count() > ml {
                        return Err(format!("{}: longer than {ml} characters", f.label));
                    }
                }
                if let Some(p) = &v.pattern {
                    if let Ok(re) = regex::Regex::new(p) {
                        if !re.is_match(&s) {
                            return Err(format!("{}: doesn't match the required pattern", f.label));
                        }
                    }
                }
                // numeric shadow for text that looks like a number (allows sorting text fields numerically later)
                let num = s.trim().parse::<f64>().ok();
                (Some(s), num)
            }
        };
        out.push(pair);
    }
    if v.required && out.is_empty() {
        return Err(format!("{} is required", f.label));
    }
    Ok(out)
}

impl Db {
    pub fn fields(&self) -> DbResult<Vec<FieldDef>> {
        let mut st = self.conn.prepare("SELECT id, def FROM fields").map_err(e)?;
        let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))).map_err(e)?;
        let mut out: Vec<FieldDef> = rows
            .filter_map(|r| r.ok())
            .filter_map(|(id, def)| {
                let mut f: FieldDef = serde_json::from_str(&def).ok()?;
                f.id = id;
                Some(f)
            })
            .collect();
        out.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.label.to_lowercase().cmp(&b.label.to_lowercase())));
        Ok(out)
    }

    pub fn field_catalog(&self) -> FieldCatalog {
        FieldCatalog::new(&self.fields().unwrap_or_default())
    }

    /// Register or update a field definition. Non-destructive rules:
    /// * a key owned by another plugin is never taken over: a plugin declaring it with the same type shares it, with another type is refused;
    /// * changing the value type is refused when values exist (the old type stays, data kept);
    /// * presentation attributes (label, visibility, widget…) may change freely.
    /// The program's or a plugin's field, defined again each time it loads: what the person chose for it (shown, in search, sort, its own filter, order, indexed) stays.
    pub fn register_field(&self, def: &FieldDef) -> DbResult<i64> {
        let old: Option<String> = self.conn.query_row("SELECT def FROM fields WHERE key = ?", [def.key.trim()], |r| r.get(0)).optional().map_err(e)?;
        let mut d = def.clone();
        if let Some(old) = old.and_then(|j| serde_json::from_str::<FieldDef>(&j).ok()) {
            d.visible = old.visible;
            d.own_filter = old.own_filter;
            d.filter_kind = old.filter_kind;
            d.sort_order = old.sort_order;
            d.indexed = old.indexed;
            d.pair = d.pair.or(old.pair);
            d.needs_scan = old.needs_scan;
            for (k, v) in old.extra {
                d.extra.entry(k).or_insert(v);
            }
        }
        self.define_field(&d)
    }

    pub fn define_field(&self, def: &FieldDef) -> DbResult<i64> {
        let key = def.key.trim();
        if key.is_empty() || !key.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            return Err(format!("Field key '{key}' must use letters, digits, '_' or '.'"));
        }
        if query::fields::builtin_for(key).is_some() {
            return Err(format!("'{key}' is a built-in property name"));
        }
        let existing: Option<(i64, String, String)> = self
            .conn
            .query_row("SELECT id, def, owner FROM fields WHERE key = ?", [key], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .optional()
            .map_err(e)?;
        let now = now_ms();
        match existing {
            None => {
                let mut d = def.clone();
                d.key = key.to_string();
                if d.sort_order == 0 {
                    d.sort_order = self.conn.query_row("SELECT count(*) + 1 FROM fields", [], |r| r.get(0)).unwrap_or(1);
                }
                let json = serde_json::to_string(&d).map_err(e)?;
                self.conn
                    .execute(
                        "INSERT INTO fields(uid, key, def, type, owner, created_at, updated_at) VALUES(uid_bytes(?), ?, ?, ?, ?, ?, ?)",
                        params![new_uid(), key, json, d.ftype.as_str(), d.owner, now, now],
                    )
                    .map_err(e)?;
                Ok(self.conn.last_insert_rowid())
            }
            Some((id, old_json, owner)) => {
                let old: FieldDef = serde_json::from_str(&old_json).map_err(e)?;
                if owner != def.owner && owner != "user" && def.owner != "user" {
                    // the first plugin's definition stays; one of the same type joins it, so each fills it for its own files
                    if old.ftype != def.ftype {
                        return Err(format!("Field '{key}' belongs to '{owner}' as {}; a plugin sharing it declares the same type", old.ftype.as_str()));
                    }
                    if !old.shared_with.contains(&def.owner) {
                        let mut d = old.clone();
                        d.shared_with.push(def.owner.clone());
                        let json = serde_json::to_string(&d).map_err(e)?;
                        self.conn.execute("UPDATE fields SET def = ?, updated_at = ? WHERE id = ?", params![json, now, id]).map_err(e)?;
                    }
                    return Ok(id);
                }
                let mut d = def.clone();
                d.key = key.to_string();
                if old.ftype != d.ftype {
                    let n: i64 =
                        self.conn.query_row("SELECT count(*) FROM field_values WHERE field_id = ?", [id], |r| r.get(0)).map_err(e)?;
                    if n > 0 {
                        // keep values readable: never reinterpret stored data
                        d.ftype = old.ftype;
                    }
                }
                // keep the original owner so plugin removal doesn't orphan user-owned data and vice versa
                d.owner = owner.clone();
                d.shared_with = old.shared_with.clone();
                let json = serde_json::to_string(&d).map_err(e)?;
                self.conn
                    .execute(
                        "UPDATE fields SET def = ?, type = ?, updated_at = ?, schema_version = schema_version + 1 WHERE id = ?",
                        params![json, d.ftype.as_str(), now, id],
                    )
                    .map_err(e)?;
                Ok(id)
            }
        }
    }

    /// Delete a user field and its values. Plugin fields can only be hidden.
    pub fn delete_field(&self, id: i64) -> DbResult<()> {
        let owner: String = self.conn.query_row("SELECT owner FROM fields WHERE id = ?", [id], |r| r.get(0)).map_err(e)?;
        if owner != "user" {
            return Err(format!("This field belongs to plugin '{owner}'; hide it instead."));
        }
        let holders: Vec<AssetId> = {
            let mut st = self.conn.prepare("SELECT DISTINCT asset_id FROM field_values WHERE field_id = ?").map_err(e)?;
            let rows = st.query_map([id], |r| r.get(0)).map_err(e)?;
            rows.filter_map(|r| r.ok()).collect()
        };
        self.conn.execute("DELETE FROM fields WHERE id = ?", [id]).map_err(e)?;
        self.refresh_fts(&holders)
    }

    pub fn field_values(&self, asset: AssetId) -> DbResult<Vec<FieldValue>> {
        let mut st = self
            .conn
            .prepare_cached("SELECT field_id, idx, text_value, num_value FROM field_values WHERE asset_id = ? ORDER BY field_id, idx")
            .map_err(e)?;
        let rows = st
            .query_map([asset], |r| Ok(FieldValue { field_id: r.get(0)?, idx: r.get(1)?, text: r.get(2)?, num: r.get(3)? }))
            .map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Validate and store values for one field on several assets (replaces previous values).
    pub fn set_field(&self, ids: &[AssetId], field: &FieldDef, raw: &RawValue) -> DbResult<()> {
        let values = typed_values(field, raw)?;
        let mut del = self.conn.prepare_cached("DELETE FROM field_values WHERE asset_id = ? AND field_id = ?").map_err(e)?;
        let mut ins = self
            .conn
            .prepare_cached("INSERT INTO field_values(asset_id, field_id, idx, text_value, num_value) VALUES(?, ?, ?, ?, ?)")
            .map_err(e)?;
        for id in ids {
            del.execute(params![id, field.id]).map_err(e)?;
            for (i, (t, n)) in values.iter().enumerate() {
                ins.execute(params![id, field.id, i as i64, t, n]).map_err(e)?;
            }
        }
        if field.in_text_search() {
            self.refresh_fts(ids)?;
        }
        Ok(())
    }

    pub fn clear_field(&self, ids: &[AssetId], field_id: i64) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("DELETE FROM field_values WHERE asset_id = ? AND field_id = ?").map_err(e)?;
        for id in ids {
            st.execute(params![id, field_id]).map_err(e)?;
        }
        self.refresh_fts(ids)
    }

    /// Values of user fields `from` has and `to` has none of, copied as they are.
    pub fn fill_user_fields(&self, to: AssetId, from: AssetId) -> DbResult<()> {
        self.conn
            .prepare_cached(
                "INSERT INTO field_values(asset_id, field_id, idx, text_value, num_value)
                 SELECT ?1, v.field_id, v.idx, v.text_value, v.num_value FROM field_values v JOIN fields f ON f.id = v.field_id
                 WHERE v.asset_id = ?2 AND f.owner = 'user' AND NOT EXISTS (SELECT 1 FROM field_values w WHERE w.asset_id = ?1 AND w.field_id = v.field_id)",
            )
            .map_err(e)?
            .execute(params![to, from])
            .map_err(e)?;
        Ok(())
    }

    /// The fields this item has a value of.
    pub fn fields_with_values(&self, id: AssetId) -> HashSet<i64> {
        let Ok(mut st) = self.conn.prepare_cached("SELECT DISTINCT field_id FROM field_values WHERE asset_id = ?") else { return HashSet::new() };
        st.query_map([id], |r| r.get(0)).map(|rows| rows.flatten().collect()).unwrap_or_default()
    }

    /// The fields whose value on this item was set by hand.
    pub fn overridden_ids(&self, id: AssetId) -> HashSet<i64> {
        let Ok(mut st) = self.conn.prepare_cached("SELECT field_id FROM field_overrides WHERE asset_id = ?") else { return HashSet::new() };
        st.query_map([id], |r| r.get(0)).map(|rows| rows.flatten().collect()).unwrap_or_default()
    }

    /// The items holding values of these fields.
    pub fn holders(&self, field_ids: &[i64]) -> DbResult<Vec<AssetId>> {
        if field_ids.is_empty() {
            return Ok(vec![]);
        }
        let mut st = self.conn.prepare(&format!("SELECT DISTINCT asset_id FROM field_values WHERE field_id IN ({}) ORDER BY asset_id", vec!["?"; field_ids.len()].join(", "))).map_err(e)?;
        let rows = st.query_map(params_from_iter(field_ids.iter()), |r| r.get(0)).map_err(e)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(e)
    }

}
