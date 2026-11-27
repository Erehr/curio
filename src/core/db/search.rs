//! The main query: a search, its scope and sort, as SQL.

use crate::core::db::IN_TRASH;
use crate::core::db::ITEMS;
use crate::core::db::UNCATEGORIZED;
use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, OptionalExtension};
use crate::core::model::*;
use crate::core::query::{self, fields::FieldCatalog};

struct SearchPlan {
    joins: String,
    filter: String,
    params: Vec<Value>,
    forced_sort: Option<String>,
}

struct GroupSql {
    extra_join: String,
    select_key: String,
    select_null: String,
    select_rank: String,
    nocase: bool,
    select_params: Vec<Value>,
    join_params: Vec<Value>,
}

fn date_group_sql(col: &str, d: &DateBounds, select_params: &mut Vec<Value>) -> (String, String, String) {
    let push_bounds = |out: &mut Vec<Value>| {
        out.extend([
            Value::Integer(d.tomorrow),
            Value::Integer(d.today),
            Value::Integer(d.yesterday),
            Value::Integer(d.this_week),
            Value::Integer(d.last_week),
            Value::Integer(d.this_month),
            Value::Integer(d.last_month),
            Value::Integer(d.this_year),
            Value::Integer(d.tz_offset_ms),
        ]);
    };
    push_bounds(select_params);
    push_bounds(select_params);
    let key = format!(
        "CASE WHEN {col} IS NULL OR {col} <= 0 THEN 'date:unknown' \
         WHEN {col} >= ? THEN 'date:later' \
         WHEN {col} >= ? THEN 'date:today' \
         WHEN {col} >= ? THEN 'date:yesterday' \
         WHEN {col} >= ? THEN 'date:thisWeek' \
         WHEN {col} >= ? THEN 'date:lastWeek' \
         WHEN {col} >= ? THEN 'date:thisMonth' \
         WHEN {col} >= ? THEN 'date:lastMonth' \
         WHEN {col} >= ? THEN 'date:thisYear' \
         ELSE COALESCE('year:' || strftime('%Y', ({col} + ?) / 1000, 'unixepoch'), 'date:unknown') END"
    );
    let null_expr = format!("({col} IS NULL OR {col} <= 0)");
    let rank = format!(
        "CASE WHEN {col} IS NULL OR {col} <= 0 THEN 0 \
         WHEN {col} >= ? THEN 100001 \
         WHEN {col} >= ? THEN 100000 \
         WHEN {col} >= ? THEN 99999 \
         WHEN {col} >= ? THEN 99998 \
         WHEN {col} >= ? THEN 99997 \
         WHEN {col} >= ? THEN 99996 \
         WHEN {col} >= ? THEN 99995 \
         WHEN {col} >= ? THEN 99994 \
         ELSE CAST(COALESCE(strftime('%Y', ({col} + ?) / 1000, 'unixepoch'), '0') AS INTEGER) END"
    );
    (key, null_expr, rank)
}

fn group_sql(group: &GroupBy, cat: &FieldCatalog) -> Option<GroupSql> {
    let mut select_params = Vec::new();
    let mut join_params = Vec::new();
    let mut extra_join = String::new();
    let mut nocase = false;
    let (select_key, select_null, select_rank) = match &group.key {
        GroupKey::None => return None,
        GroupKey::Kind => {
            let ptypes = crate::core::formats::plugin_types();
            let base_rank = "CASE a.kind WHEN 'image' THEN '01' WHEN 'video' THEN '02' WHEN 'audio' THEN '03' WHEN 'document' THEN '04' WHEN 'code' THEN '05' WHEN 'link' THEN '06' WHEN 'archive' THEN '07' WHEN 'font' THEN '08' WHEN 'model' THEN '09' ELSE '99' END";
            if ptypes.is_empty() {
                ("'kind:' || a.kind".into(), "(a.kind = 'other')".into(), base_rank.into())
            } else {
                let mut key_cases = String::from("CASE");
                for (label, exts) in &ptypes {
                    if exts.is_empty() {
                        continue;
                    }
                    let ph = vec!["?"; exts.len()].join(",");
                    key_cases.push_str(&format!(" WHEN a.ext IN ({ph}) THEN ?"));
                    select_params.extend(exts.iter().map(|e| Value::Text(e.clone())));
                    select_params.push(Value::Text(format!("ptype:{label}")));
                }
                key_cases.push_str(" ELSE 'kind:' || a.kind END");

                let all_exts: Vec<&String> = ptypes.iter().flat_map(|(_, es)| es.iter()).collect();
                let null_expr = if all_exts.is_empty() {
                    "(a.kind = 'other')".into()
                } else {
                    let ph = vec!["?"; all_exts.len()].join(",");
                    select_params.extend(all_exts.iter().map(|e| Value::Text((*e).clone())));
                    format!("(a.kind = 'other' AND a.ext NOT IN ({ph}))")
                };

                let mut rank_cases = String::from("CASE");
                for (label, exts) in &ptypes {
                    if exts.is_empty() {
                        continue;
                    }
                    let ph = vec!["?"; exts.len()].join(",");
                    rank_cases.push_str(&format!(" WHEN a.ext IN ({ph}) THEN ?"));
                    select_params.extend(exts.iter().map(|e| Value::Text(e.clone())));
                    select_params.push(Value::Text(format!("10:{}", label.to_lowercase())));
                }
                rank_cases.push_str(&format!(" ELSE {base_rank} END"));

                (key_cases, null_expr, rank_cases)
            }
        }
        GroupKey::Ext => {
            nocase = true;
            ("CASE WHEN a.ext = '' THEN 'empty:' ELSE 'ext:' || upper(a.ext) END".into(), "(a.ext = '')".into(), "upper(a.ext)".into())
        }
        GroupKey::Imported => date_group_sql("a.added_at", &group.dates, &mut select_params),
        GroupKey::Modified => date_group_sql("a.mtime", &group.dates, &mut select_params),
        GroupKey::Created => date_group_sql("a.ctime", &group.dates, &mut select_params),
        GroupKey::Rating => ("CASE WHEN a.rating > 0 THEN 'rating:' || a.rating ELSE 'rating:0' END".into(), "(a.rating <= 0)".into(), "a.rating".into()),
        GroupKey::Size => (
            "CASE WHEN a.size <= 0 THEN 'size:empty' WHEN a.size < 102400 THEN 'size:tiny' WHEN a.size < 1048576 THEN 'size:small' WHEN a.size < 16777216 THEN 'size:medium' WHEN a.size < 134217728 THEN 'size:large' ELSE 'size:huge' END".into(),
            "0".into(),
            "CASE WHEN a.size <= 0 THEN 0 WHEN a.size < 102400 THEN 1 WHEN a.size < 1048576 THEN 2 WHEN a.size < 16777216 THEN 3 WHEN a.size < 134217728 THEN 4 ELSE 5 END".into(),
        ),
        GroupKey::Name => {
            let ch = "upper(substr(trim(COALESCE(NULLIF(a.display_name,''), a.name)), 1, 1))";
            (
                format!("CASE WHEN {ch} BETWEEN 'A' AND 'Z' THEN 'name:' || {ch} WHEN {ch} BETWEEN '0' AND '9' THEN 'name:0-9' ELSE 'name:#' END"),
                format!("CASE WHEN ({ch} BETWEEN 'A' AND 'Z') OR ({ch} BETWEEN '0' AND '9') THEN 0 ELSE 1 END"),
                format!("CASE WHEN {ch} BETWEEN '0' AND '9' THEN '0' ELSE {ch} END"),
            )
        }
        GroupKey::Location => {
            extra_join = " LEFT JOIN roots _gr_root ON _gr_root.id = a.root_id".into();
            nocase = true;
            let eagle = "(_gr_root.path LIKE '%.library/images' OR _gr_root.path LIKE '%.library\\images')";
            (
                format!("CASE WHEN _gr_root.kind = 'managed' THEN 'location:library' WHEN _gr_root.explicit_only = 0 OR {eagle} THEN 'location:' || a.root_id ELSE 'location:linked' END"),
                format!("CASE WHEN _gr_root.kind = 'managed' THEN 0 WHEN _gr_root.explicit_only = 0 OR {eagle} THEN 1 ELSE 2 END"),
                "COALESCE(_gr_root.label, '')".into(),
            )
        }
        GroupKey::Field(k) => {
            let f = cat.custom(k).filter(|f| f.sorts())?;
            extra_join = " LEFT JOIN field_values _gfv ON _gfv.asset_id = a.id AND _gfv.field_id = ? AND _gfv.idx = 0".into();
            join_params.push(Value::Integer(f.id));
            match &f.ftype {
                FieldType::Date | FieldType::DateTime => date_group_sql("_gfv.num_value", &group.dates, &mut select_params),
                FieldType::Boolean => (
                    "CASE WHEN _gfv.num_value IS NULL THEN 'empty:' WHEN _gfv.num_value <> 0 THEN 'bool:1' ELSE 'bool:0' END".into(),
                    "(_gfv.num_value IS NULL)".into(),
                    "COALESCE(_gfv.num_value, 0)".into(),
                ),
                FieldType::Integer => (
                    "CASE WHEN _gfv.num_value IS NULL THEN 'empty:' ELSE 'text:' || CAST(CAST(_gfv.num_value AS INTEGER) AS TEXT) END".into(),
                    "(_gfv.num_value IS NULL)".into(),
                    "_gfv.num_value".into(),
                ),
                FieldType::Float => (
                    "CASE WHEN _gfv.num_value IS NULL THEN 'empty:' ELSE 'text:' || CAST(_gfv.num_value AS TEXT) END".into(),
                    "(_gfv.num_value IS NULL)".into(),
                    "_gfv.num_value".into(),
                ),
                _ => {
                    nocase = true;
                    (
                        "CASE WHEN COALESCE(_gfv.text_value, '') = '' THEN 'empty:' ELSE 'text:' || _gfv.text_value END".into(),
                        "(COALESCE(_gfv.text_value, '') = '')".into(),
                        "COALESCE(_gfv.text_value, '')".into(),
                    )
                }
            }
        }
    };
    Some(GroupSql { extra_join, select_key, select_null, select_rank, nocase, select_params, join_params })
}

/// ` AND root_id NOT IN (…)`, or nothing for no roots.
pub(super) fn not_in_roots(roots: &[i64]) -> String {
    if roots.is_empty() {
        return String::new();
    }
    format!(" AND root_id NOT IN ({})", roots.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(","))
}

fn sort_sql(sort: &Sort, cat: &FieldCatalog, params: &mut Vec<Value>) -> String {
    let dir = if sort.descending { "DESC" } else { "ASC" };
    match &sort.key {
        SortKey::Manual | SortKey::Imported => format!("a.added_at {dir}"),
        SortKey::Modified => format!("a.mtime {dir}"),
        SortKey::Created => format!("a.ctime {dir}"),
        SortKey::Name => format!("COALESCE(NULLIF(a.display_name,''), a.name) COLLATE NOCASE {dir}"),
        SortKey::Ext => format!("a.ext {dir}, a.name COLLATE NOCASE"),
        SortKey::Size => format!("a.size {dir}"),
        SortKey::Resolution => format!("(COALESCE(a.width,0) * COALESCE(a.height,0)) {dir}"),
        SortKey::Rating => format!("a.rating {dir}"),
        SortKey::Duration => format!("a.duration {dir}"),
        SortKey::Random => "random()".into(),
        SortKey::Field(k) => match cat.custom(k) {
            Some(f) if f.sorts() => {
                let col = if f.ftype.is_numeric() { "num_value" } else { "text_value" };
                let nocase = if f.ftype.is_numeric() { "" } else { " COLLATE NOCASE" };
                params.extend([Value::Integer(f.id), Value::Integer(f.id)]);
                // NULLs last regardless of direction
                let mut out = format!(
                    "(SELECT fv.{col} IS NULL FROM field_values fv WHERE fv.asset_id = a.id AND fv.field_id = ? AND fv.idx = 0) IS NOT 0, \
                     (SELECT fv.{col} FROM field_values fv WHERE fv.asset_id = a.id AND fv.field_id = ? AND fv.idx = 0){nocase} {dir}"
                );
                // a field paired with this one (such as series_index with series) orders items within ties
                if let Some(p) = cat.by_key.values().find(|p| p.pair.as_deref() == Some(f.key.as_str()) && p.sorts()) {
                    let pcol = if p.ftype.is_numeric() { "num_value" } else { "text_value" };
                    let pnocase = if p.ftype.is_numeric() { "" } else { " COLLATE NOCASE" };
                    params.extend([Value::Integer(p.id), Value::Integer(p.id)]);
                    out.push_str(&format!(
                        ", (SELECT fv.{pcol} IS NULL FROM field_values fv WHERE fv.asset_id = a.id AND fv.field_id = ? AND fv.idx = 0) IS NOT 0, \
                         (SELECT fv.{pcol} FROM field_values fv WHERE fv.asset_id = a.id AND fv.field_id = ? AND fv.idx = 0){pnocase} ASC"
                    ));
                }
                out
            }
            _ => format!("a.added_at {dir}"),
        },
    }
}

impl Db {
    /// Run a scope + query + sort against the index. Returns ids and aspect ratios in order.
    /// Only two numbers per match leave the database; rows are fetched later per visible page.
    pub fn search(&self, scope: &Scope, query_text: &str, sort: &Sort, include_subfolders: bool, cat: &FieldCatalog) -> DbResult<Vec<(AssetId, f32)>> {
        self.search_first(scope, query_text, sort, include_subfolders, cat, None)
    }

    /// The first `limit` matches of a search, in its order.
    pub fn search_first(&self, scope: &Scope, query_text: &str, sort: &Sort, include_subfolders: bool, cat: &FieldCatalog, limit: Option<usize>) -> DbResult<Vec<(AssetId, f32)>> {
        let plan = self.search_plan(scope, query_text, sort, include_subfolders, cat)?;
        let mut params = plan.params;
        let order = match plan.forced_sort {
            Some(s) => s,
            None => sort_sql(sort, cat, &mut params),
        };
        let limit = limit.map(|n| format!(" LIMIT {n}")).unwrap_or_default();
        // items that tie, as those without a value of the field sorted by, stay newest first as the default view has them
        let sql = format!("SELECT a.id, a.aspect FROM assets a{} WHERE {} ORDER BY {order}, a.added_at DESC, a.id{limit}", plan.joins, plan.filter);
        let mut st = self.conn.prepare(&sql).map_err(e)?;
        let rows = st.query_map(params_from_iter(params.iter()), |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)? as f32))).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Run a scope + query + sort grouped into contiguous runs, returning `(id, aspect)` pairs and compact group spans.
    pub fn search_grouped(&self, scope: &Scope, query_text: &str, sort: &Sort, group: &GroupBy, include_subfolders: bool, cat: &FieldCatalog) -> DbResult<(Vec<(AssetId, f32)>, Vec<GroupSpan>)> {
        let Some(gs) = group_sql(group, cat) else {
            return Ok((self.search(scope, query_text, sort, include_subfolders, cat)?, vec![]));
        };
        let plan = self.search_plan(scope, query_text, sort, include_subfolders, cat)?;
        let mut sort_params = Vec::new();
        let order = match plan.forced_sort {
            Some(s) => s,
            None => sort_sql(sort, cat, &mut sort_params),
        };
        let gdir = if group.descending { "DESC" } else { "ASC" };
        let collate = if gs.nocase { " COLLATE NOCASE" } else { "" };
        // extra_join precedes plan.joins so positional params follow select -> join -> plan -> sort without interleaving
        let sql = format!(
            "SELECT a.id, a.aspect, {} AS _gk, {} AS _gn, {} AS _gr FROM assets a{}{} WHERE {} ORDER BY _gn ASC, _gr{collate} {gdir}, _gk{collate} ASC, {order}, a.added_at DESC, a.id",
            gs.select_key, gs.select_null, gs.select_rank, gs.extra_join, plan.joins, plan.filter
        );
        let mut params = Vec::with_capacity(gs.select_params.len() + gs.join_params.len() + plan.params.len() + sort_params.len());
        params.extend(gs.select_params);
        params.extend(gs.join_params);
        params.extend(plan.params);
        params.extend(sort_params);
        let mut st = self.conn.prepare(&sql).map_err(e)?;
        let iter = st.query_map(params_from_iter(params.iter()), |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)? as f32, r.get::<_, String>(2)?))).map_err(e)?;
        let mut rows: Vec<(AssetId, f32)> = Vec::new();
        let mut groups: Vec<GroupSpan> = Vec::new();
        let mut last_key = String::new();
        for item in iter.flatten() {
            let (id, aspect, gk) = item;
            let idx = rows.len() as u32;
            rows.push((id, aspect));
            let cmp = if gs.nocase { gk.to_lowercase() } else { gk.clone() };
            if let Some(last) = groups.last_mut().filter(|_| cmp == last_key) {
                last.count += 1;
            } else {
                let (kind, val) = gk.split_once(':').unwrap_or(("text", &gk));
                last_key = cmp.clone();
                groups.push(GroupSpan { key: cmp, kind: kind.to_string(), val: val.to_string(), start: idx, count: 1 });
            }
        }
        Ok((rows, groups))
    }

    /// How many items a search finds, without sorting them.
    pub fn search_count(&self, scope: &Scope, query_text: &str, include_subfolders: bool, cat: &FieldCatalog) -> DbResult<i64> {
        let plan = self.search_plan(scope, query_text, &Sort::default(), include_subfolders, cat)?;
        let sql = format!("SELECT count(*) FROM assets a{} WHERE {}", plan.joins, plan.filter);
        self.conn.query_row(&sql, params_from_iter(plan.params.iter()), |r| r.get(0)).map_err(e)
    }

    fn search_plan(&self, scope: &Scope, query_text: &str, sort: &Sort, include_subfolders: bool, cat: &FieldCatalog) -> DbResult<SearchPlan> {
        let mut params: Vec<Value> = Vec::new();
        let mut joins = String::new();
        let mut wh: Vec<String> = Vec::new();
        let mut forced_sort: Option<String> = None;
        wh.push(ITEMS.into());
        match scope {
            Scope::All => wh.push("a.deleted = 0".into()),
            Scope::Favorites => wh.push("a.deleted = 0 AND a.favorite = 1".into()),
            Scope::Uncategorized => {
                wh.push("a.deleted = 0".into());
                wh.push(UNCATEGORIZED.into());
            }
            Scope::Untagged => {
                wh.push("a.deleted = 0".into());
                wh.push("NOT EXISTS (SELECT 1 FROM asset_tags t WHERE t.asset_id = a.id)".into());
            }
            Scope::Recent => {
                wh.push("a.deleted = 0 AND a.last_used IS NOT NULL".into());
                forced_sort = Some("a.last_used DESC".into());
            }
            Scope::Random => {
                wh.push("a.deleted = 0".into());
                forced_sort = Some("random()".into());
            }
            Scope::Trash => wh.push(format!("({IN_TRASH})")),
            Scope::Missing => wh.push("a.deleted = 0 AND a.status <> 0".into()),
            Scope::Roots(list) => {
                let ph = vec!["?"; list.len().max(1)].join(",");
                wh.push(format!("a.deleted = 0 AND a.root_id IN ({ph})"));
                if list.is_empty() {
                    params.push(Value::Integer(-1));
                }
                params.extend(list.iter().map(|i| Value::Integer(*i)));
            }
            Scope::Folders(list) => {
                wh.push("a.deleted = 0".into());
                let ph = vec!["?"; list.len().max(1)].join(",");
                if list.is_empty() {
                    params.push(Value::Integer(-1));
                }
                if include_subfolders {
                    let seeds = vec!["SELECT ?"; list.len().max(1)].join(" UNION ALL ");
                    wh.push(format!(
                        "EXISTS (SELECT 1 FROM asset_folders f WHERE f.asset_id = a.id AND f.folder_id IN (
                            WITH RECURSIVE sub(id) AS ({seeds} UNION ALL SELECT fo.id FROM folders fo JOIN sub ON fo.parent_id = sub.id)
                            SELECT id FROM sub))"
                    ));
                } else {
                    wh.push(format!("EXISTS (SELECT 1 FROM asset_folders f WHERE f.asset_id = a.id AND f.folder_id IN ({ph}))"));
                }
                params.extend(list.iter().map(|i| Value::Integer(*i)));
            }
            Scope::Root(id) => {
                if include_subfolders || self.root(*id).is_some_and(|r| crate::core::import::eagle::eagle_images_path(&r.path)) {
                    wh.push("a.deleted = 0 AND a.root_id = ?".into());
                } else {
                    wh.push("a.deleted = 0 AND a.root_id = ? AND instr(a.rel_path, '/') = 0".into());
                }
                params.push(Value::Integer(*id));
            }
            Scope::RootDir(id, dir) => {
                let prefix = format!("{}/", dir.trim_matches('/'));
                let plen = prefix.chars().count() as i64;
                if include_subfolders {
                    wh.push("a.deleted = 0 AND a.root_id = ? AND substr(a.rel_path, 1, ?) = ?".into());
                    params.push(Value::Integer(*id));
                    params.push(Value::Integer(plen));
                    params.push(Value::Text(prefix));
                } else {
                    wh.push("a.deleted = 0 AND a.root_id = ? AND substr(a.rel_path, 1, ?) = ? AND instr(substr(a.rel_path, ?), '/') = 0".into());
                    params.push(Value::Integer(*id));
                    params.push(Value::Integer(plen));
                    params.push(Value::Text(prefix));
                    params.push(Value::Integer(plen + 1));
                }
            }
            Scope::Tag(id) => {
                wh.push("a.deleted = 0 AND EXISTS (SELECT 1 FROM asset_tags t WHERE t.asset_id = a.id AND t.tag_id = ?)".into());
                params.push(Value::Integer(*id));
            }
            Scope::Folder(id) => {
                wh.push("a.deleted = 0".into());
                if sort.key == SortKey::Manual && !include_subfolders {
                    joins.push_str(" JOIN asset_folders af ON af.asset_id = a.id AND af.folder_id = ?");
                    params.push(Value::Integer(*id));
                    forced_sort = Some(format!("af.position {}", if sort.descending { "DESC" } else { "ASC" }));
                } else if include_subfolders {
                    wh.push(
                        "EXISTS (SELECT 1 FROM asset_folders f WHERE f.asset_id = a.id AND f.folder_id IN (
                            WITH RECURSIVE sub(id) AS (SELECT ? UNION ALL SELECT fo.id FROM folders fo JOIN sub ON fo.parent_id = sub.id)
                            SELECT id FROM sub))"
                            .into(),
                    );
                    params.push(Value::Integer(*id));
                } else {
                    wh.push("EXISTS (SELECT 1 FROM asset_folders f WHERE f.asset_id = a.id AND f.folder_id = ?)".into());
                    params.push(Value::Integer(*id));
                }
            }
            Scope::SmartFolder(id) | Scope::SmartBucket(id, _) => {
                wh.push("a.deleted = 0".into());
                // a subfolder narrows what its parents find; one without rules finds what its subfolders find together, or everything when grouped dynamically
                let mut chain = self.smart_folder_rules(*id)?;
                let own = chain.pop().unwrap_or_default();
                for q in chain.iter().filter(|q| !q.trim().is_empty()) {
                    let c = query::compile::compile_str(q, cat)?;
                    wh.push(c.sql);
                    params.extend(c.params);
                }
                let (sql, ps) = if own.trim().is_empty() {
                    let (g, gp) = self.smart_group(*id, cat, 0)?;
                    if g == "0" && self.smart_group_by(*id)?.is_some() { ("1".into(), vec![]) } else { (g, gp) }
                } else {
                    query::compile::compile_str(&own, cat).map(|c| (c.sql, c.params))?
                };
                wh.push(sql);
                params.extend(ps);
                if let Scope::SmartBucket(_, val) = scope {
                    let (bsql, bps) = match self.smart_group_by(*id)? {
                        Some(gb) => self.smart_bucket_cond(&gb, val, cat)?,
                        None => ("0".into(), vec![]),
                    };
                    wh.push(bsql);
                    params.extend(bps);
                }
            }
        }
        // params so far belong to joins + scope; the user query comes last
        let qt = query_text.trim();
        if !qt.is_empty() {
            let c = query::compile::compile_str(qt, cat)?;
            wh.push(c.sql);
            params.extend(c.params);
        }
        let filter = if wh.is_empty() { "1".to_string() } else { wh.join(" AND ") };
        Ok(SearchPlan { joins, filter, params, forced_sort })
    }

    pub fn record_search(&self, q: &str) -> DbResult<()> {
        let q = q.trim();
        if q.is_empty() {
            return Ok(());
        }
        self.conn
            .execute(
                "INSERT INTO search_history(query, used_at) VALUES(?, ?)
                 ON CONFLICT(query) DO UPDATE SET used_at = excluded.used_at, uses = uses + 1",
                params![q, now_ms()],
            )
            .map_err(e)?;
        self.conn
            .execute("DELETE FROM search_history WHERE query NOT IN (SELECT query FROM search_history ORDER BY used_at DESC LIMIT 50)", [])
            .map_err(e)?;
        Ok(())
    }

    pub fn search_history(&self) -> Vec<String> {
        let Ok(mut st) = self.conn.prepare("SELECT query FROM search_history ORDER BY used_at DESC LIMIT 20") else {
            return vec![];
        };
        st.query_map([], |r| r.get(0)).map(|r| r.filter_map(|x| x.ok()).collect()).unwrap_or_default()
    }

    /// Totals for a selection (or a whole folder): count, total size, and only the tags and
    /// folders shared by every item (what the Inspector shows for a multi-selection).
    pub fn selection_summary(&self, ids: &[AssetId]) -> DbResult<serde_json::Value> {
        self.conn.execute_batch("CREATE TEMP TABLE IF NOT EXISTS sel(id INTEGER PRIMARY KEY); DELETE FROM temp.sel;").map_err(e)?;
        {
            let mut st = self.conn.prepare_cached("INSERT OR IGNORE INTO temp.sel(id) VALUES(?)").map_err(e)?;
            for id in ids {
                st.execute([id]).map_err(e)?;
            }
        }
        let n: i64 = self.conn.query_row("SELECT count(*) FROM temp.sel", [], |r| r.get(0)).map_err(e)?;
        let (size, w, h, dur, rating_min, rating_max, fav_min, fav_max, exp_min, exp_max): (i64, Option<i64>, Option<i64>, Option<f64>, i64, i64, i64, i64, i64, i64) = self
            .conn
            .query_row(
                "SELECT COALESCE(sum(a.size),0), max(a.width), max(a.height), sum(a.duration), COALESCE(min(a.rating),0), COALESCE(max(a.rating),0), COALESCE(min(a.favorite),0), COALESCE(max(a.favorite),0), COALESCE(min(a.explicit),0), COALESCE(max(a.explicit),0)
                 FROM assets a JOIN temp.sel s ON s.id = a.id",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?)),
            )
            .map_err(e)?;
        let shared = |table: &str, key: &str, names: &str| -> DbResult<Vec<serde_json::Value>> {
            let sql = format!(
                "SELECT n.id, n.name FROM {table} x JOIN temp.sel s ON s.id = x.asset_id JOIN {names} n ON n.id = x.{key}
                 GROUP BY n.id HAVING count(*) = ?1 ORDER BY n.name COLLATE NOCASE"
            );
            let mut st = self.conn.prepare(&sql).map_err(e)?;
            let rows = st.query_map([n], |r| Ok(serde_json::json!({"id": r.get::<_, i64>(0)?, "name": r.get::<_, String>(1)?}))).map_err(e)?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        };
        // notes and source addresses the items share, or none when they differ
        let (notes, note, urls, url): (i64, String, i64, String) = self
            .conn
            .query_row(
                "SELECT count(DISTINCT COALESCE(a.annotation, '')), COALESCE(min(COALESCE(a.annotation, '')), ''), count(DISTINCT COALESCE(a.source_url, '')), COALESCE(min(COALESCE(a.source_url, '')), '')
                 FROM assets a JOIN temp.sel s ON s.id = a.id",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(e)?;
        let kinds: Vec<serde_json::Value> = {
            let mut st = self.conn.prepare("SELECT a.kind, count(*) FROM assets a JOIN temp.sel s ON s.id = a.id GROUP BY a.kind ORDER BY 2 DESC").map_err(e)?;
            let rows = st.query_map([], |r| Ok(serde_json::json!({"kind": r.get::<_, String>(0)?, "count": r.get::<_, i64>(1)?}))).map_err(e)?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let any_tag: bool = self.conn.query_row("SELECT EXISTS(SELECT 1 FROM asset_tags x JOIN temp.sel s ON s.id = x.asset_id)", [], |r| r.get(0)).map_err(e)?;
        Ok(serde_json::json!({
            "count": n, "size": size, "maxWidth": w, "maxHeight": h, "duration": dur,
            "ratingMin": rating_min, "ratingMax": rating_max,
            "favoriteMin": fav_min, "favoriteMax": fav_max,
            "explicitMin": exp_min, "explicitMax": exp_max,
            "annotation": (notes <= 1).then_some(note), "url": (urls <= 1).then_some(url),
            "tags": shared("asset_tags", "tag_id", "tags")?, "anyTag": any_tag, "folders": shared("asset_folders", "folder_id", "folders")?, "kinds": kinds,
        }))
    }

    /// Counts shown next to the Rating and Shape filter options, for the given items.
    /// A landscape picture also counts towards 4:3 / 16:9.
    pub fn filter_facets(&self, ids: &[AssetId]) -> DbResult<serde_json::Value> {
        use crate::core::consts::{PANORAMIC_RATIO as r, RATIO_TOLERANCE as t};
        self.conn.execute_batch("CREATE TEMP TABLE IF NOT EXISTS sel(id INTEGER PRIMARY KEY); DELETE FROM temp.sel;").map_err(e)?;
        {
            let mut st = self.conn.prepare_cached("INSERT OR IGNORE INTO temp.sel(id) VALUES(?)").map_err(e)?;
            for id in ids {
                st.execute([id]).map_err(e)?;
            }
        }
        let mut rating = serde_json::Map::new();
        {
            let mut st = self.conn.prepare("SELECT a.rating, count(*) FROM assets a JOIN temp.sel s ON s.id = a.id GROUP BY a.rating").map_err(e)?;
            let rows = st.query_map([], |x| Ok((x.get::<_, i64>(0)?, x.get::<_, i64>(1)?))).map_err(e)?;
            for (k, n) in rows.flatten() {
                rating.insert(k.to_string(), n.into());
            }
        }
        let a = "(a.width * 1.0 / a.height)";
        let i = "(a.height * 1.0 / a.width)";
        let sql = format!(
            "SELECT
               COALESCE(sum(a.width > a.height AND {a} < {r}), 0),
               COALESCE(sum(a.height > a.width AND {i} < {r}), 0),
               COALESCE(sum(a.width = a.height AND a.width > 0), 0),
               COALESCE(sum(a.width > a.height AND {a} >= {r}), 0),
               COALESCE(sum(a.height > a.width AND {i} >= {r}), 0),
               COALESCE(sum(a.height > 0 AND abs({a} - 4.0 / 3.0) <= {t}), 0),
               COALESCE(sum(a.height > 0 AND abs({a} - 16.0 / 9.0) <= {t}), 0)
             FROM assets a JOIN temp.sel s ON s.id = a.id"
        );
        let shape = self
            .conn
            .query_row(&sql, [], |x| {
                Ok(serde_json::json!({
                    "landscape": x.get::<_, i64>(0)?, "portrait": x.get::<_, i64>(1)?, "square": x.get::<_, i64>(2)?,
                    "wide": x.get::<_, i64>(3)?, "tall": x.get::<_, i64>(4)?,
                    "4:3": x.get::<_, i64>(5)?, "16:9": x.get::<_, i64>(6)?,
                }))
            })
            .map_err(e)?;
        let group = |col: &str| -> DbResult<serde_json::Map<String, serde_json::Value>> {
            let mut st = self.conn.prepare(&format!("SELECT {col}, count(*) FROM assets a JOIN temp.sel s ON s.id = a.id GROUP BY {col}")).map_err(e)?;
            let rows = st.query_map([], |x| Ok((x.get::<_, String>(0)?, x.get::<_, i64>(1)?))).map_err(e)?;
            let mut m = serde_json::Map::new();
            for (k, n) in rows.flatten() {
                m.insert(k, n.into());
            }
            Ok(m)
        };
        let kinds = group("a.kind")?;
        let exts = group("a.ext")?;
        let mut moving = serde_json::Map::new();
        {
            let mut st = self.conn.prepare("SELECT a.ext, count(*) FROM assets a JOIN temp.sel s ON s.id = a.id WHERE COALESCE(a.animated, 0) = 1 GROUP BY a.ext").map_err(e)?;
            let rows = st.query_map([], |x| Ok((x.get::<_, String>(0)?, x.get::<_, i64>(1)?))).map_err(e)?;
            for (k, n) in rows.flatten() {
                moving.insert(k, n.into());
            }
        }
        let (animated, favorite, explicit): (i64, i64, i64) = self.conn.query_row("SELECT COALESCE(sum(COALESCE(a.animated, 0)), 0), COALESCE(sum(a.favorite), 0), COALESCE(sum(a.explicit), 0) FROM assets a JOIN temp.sel s ON s.id = a.id", [], |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?))).map_err(e)?;
        // keyed in lower case, as folder_color compares colours
        let mut folder_colors = serde_json::Map::new();
        {
            let mut st = self.conn.prepare("SELECT lower(n.color), count(DISTINCT x.asset_id) FROM asset_folders x JOIN folders n ON n.id = x.folder_id JOIN temp.sel s ON s.id = x.asset_id WHERE COALESCE(n.color, '') <> '' GROUP BY lower(n.color)").map_err(e)?;
            let rows = st.query_map([], |x| Ok((x.get::<_, String>(0)?, x.get::<_, i64>(1)?))).map_err(e)?;
            for (k, n) in rows.flatten() {
                folder_colors.insert(k, n.into());
            }
        }
        Ok(serde_json::json!({ "rating": rating, "favorite": favorite, "explicit": explicit, "shape": shape, "kinds": kinds, "exts": exts, "animatedExts": moving, "animated": animated, "folderColors": folder_colors, "total": ids.len() }))
    }

    /// One field's values among the given items with how many items have each, most common first, for its filter menu. Values longer than a short line (prompts) are left to "contains".
    pub fn field_facets(&self, ids: &[AssetId], key: &str) -> DbResult<serde_json::Value> {
        const MAX_VALUES: i64 = 300;
        const MAX_LEN: i64 = 120;
        let Some((field_id, numeric)) = self.conn.query_row("SELECT id, type IN ('integer', 'float', 'boolean') FROM fields WHERE key = ?", [key], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, bool>(1)?))).optional().map_err(e)? else {
            return Ok(serde_json::json!({ "values": [], "set": 0 }));
        };
        self.conn.execute_batch("CREATE TEMP TABLE IF NOT EXISTS sel(id INTEGER PRIMARY KEY); DELETE FROM temp.sel;").map_err(e)?;
        {
            let mut st = self.conn.prepare_cached("INSERT OR IGNORE INTO temp.sel(id) VALUES(?)").map_err(e)?;
            for id in ids {
                st.execute([id]).map_err(e)?;
            }
        }
        let set: i64 = self.conn.query_row("SELECT count(DISTINCT v.asset_id) FROM field_values v JOIN temp.sel s ON s.id = v.asset_id WHERE v.field_id = ?", [field_id], |r| r.get(0)).map_err(e)?;
        let sql = if numeric {
            "SELECT v.num_value, count(DISTINCT v.asset_id) FROM field_values v JOIN temp.sel s ON s.id = v.asset_id WHERE v.field_id = ?1 AND v.num_value IS NOT NULL GROUP BY v.num_value ORDER BY 2 DESC, 1 LIMIT ?2"
        } else {
            "SELECT v.text_value, count(DISTINCT v.asset_id) FROM field_values v JOIN temp.sel s ON s.id = v.asset_id WHERE v.field_id = ?1 AND v.text_value <> '' AND length(v.text_value) <= ?3 GROUP BY v.text_value COLLATE NOCASE ORDER BY 2 DESC, 1 COLLATE NOCASE LIMIT ?2"
        };
        let mut st = self.conn.prepare(sql).map_err(e)?;
        let row = |r: &rusqlite::Row| -> rusqlite::Result<serde_json::Value> {
            let v: rusqlite::types::Value = r.get(0)?;
            let n: i64 = r.get(1)?;
            Ok(match v {
                rusqlite::types::Value::Real(x) => serde_json::json!([x, n]),
                rusqlite::types::Value::Integer(x) => serde_json::json!([x, n]),
                rusqlite::types::Value::Text(t) => serde_json::json!([t, n]),
                _ => serde_json::Value::Null,
            })
        };
        let values: Vec<serde_json::Value> = if numeric { st.query_map(params![field_id, MAX_VALUES], row) } else { st.query_map(params![field_id, MAX_VALUES, MAX_LEN], row) }.map_err(e)?.flatten().filter(|v| !v.is_null()).collect();
        Ok(serde_json::json!({ "values": values, "set": set, "total": ids.len() }))
    }
}
