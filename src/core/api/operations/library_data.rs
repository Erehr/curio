//! A library's own data: settings, duplicates, statistics and maintenance.

use std::collections::HashMap;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::core::api::{read, with_library, ApiError, ApiResult, Ctx};
use crate::core::db::Db;
use crate::core::library::changes::Changed;
use crate::core::model::*;
use crate::core::api::operations::libraries::Nothing;
use crate::core::api::operations::write_for;
use crate::core::api::operations::ids_of;
use crate::core::api::operations::items_of;
use crate::core::api::operations::item_reads::found;
use crate::core::api::operations::item_edits::folders_exist;
use crate::core::api::operations::bad;
use crate::core::api::operations::not_found;
use crate::core::api::operations::read_checked;
use crate::core::api::operations::MAX_LIMIT;
use crate::core::api::operations::one_id;
use crate::core::api::operations::ItemRef;

#[derive(Deserialize, JsonSchema)]
pub(super) struct SimilarParams {
    id: ItemRef,
    /// How alike, 0 to 1: the share of the fingerprint's bits that agree. 0.8 unless set.
    #[serde(default)]
    threshold: Option<f64>,
    /// 50 unless set, at most 1000.
    #[serde(default)]
    limit: Option<usize>,
}

pub(super) fn item_similar(c: &Ctx, p: SimilarParams) -> ApiResult {
    let id = one_id(c, &p.id)?;
    let threshold = p.threshold.unwrap_or(0.8).clamp(0.0, 1.0);
    let limit = p.limit.unwrap_or(50).min(MAX_LIMIT);
    read_checked(c, |db| {
        let Some((_, mine)) = db.look_of(id)? else { return Ok(Err(not_found("That item has no fingerprint yet: its thumbnail isn't made"))) };
        let max = crate::core::duplicates::fingerprints::max_bits(threshold);
        let pool = match db.looks_near(mine.dct, max)? {
            Some(near) => near,
            None => db.looks()?,
        };
        let mut near: Vec<(AssetId, f64)> = pool.iter().filter(|x| x.0 != id).map(|x| (x.0, 1.0 - crate::core::duplicates::fingerprints::bits_apart(x.2.dct, mine.dct) as f64 / 64.0)).filter(|x| x.1 >= threshold).collect();
        near.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        near.truncate(limit);
        let names: HashMap<AssetId, String> = db.uids(&near.iter().map(|x| x.0).collect::<Vec<_>>())?.into_iter().collect();
        Ok(Ok(Value::Array(near.iter().filter_map(|(i, s)| names.get(i).map(|u| json!({"id": u, "similarity": (s * 1000.0).round() / 1000.0}))).collect())))
    })
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct DuplicatesParams {
    /// The ways to compare: `content` (identical files), `looks` (thumbnails that look alike: pictures, covers, first pages), `video` (frames through each video), `sound` (the first minute of each sound) and `archive` (the entries of ZIP, CBZ, EPUB and 7z files); `content` unless set.
    #[serde(default)]
    ways: Option<Vec<String>>,
    /// How alike, 0 to 1, by way: `{ looks: 0.9, video: 0.9, sound: 0.7, archive: 0.9 }` unless set.
    #[serde(default)]
    thresholds: Option<HashMap<String, f64>>,
    /// The kinds whose thumbnails `looks` compares; every kind but video and audio, which have ways of their own, unless set.
    #[serde(default, rename = "lookKinds")]
    look_kinds: Option<Vec<String>>,
    /// Only these items.
    #[serde(default)]
    ids: Option<Vec<ItemRef>>,
    /// Only the items in this folder and its subfolders.
    #[serde(default)]
    folder: Option<i64>,
    /// Only the items this query finds.
    #[serde(default)]
    query: Option<String>,
    /// Compare those items with every item in the library, not only with each other: copies of them anywhere.
    #[serde(default, rename = "withLibrary")]
    with_library: bool,
    /// Files smaller than this many bytes are left out.
    #[serde(default, rename = "minSize")]
    min_size: Option<i64>,
}

pub(super) fn duplicates_find(c: &Ctx, p: DuplicatesParams) -> ApiResult {
    use crate::core::duplicates::{self, Way};
    let mut ways = Vec::new();
    for w in p.ways.clone().unwrap_or_else(|| vec!["content".into()]) {
        let way = Way::parse(&w).ok_or_else(|| bad(format!("There is no way {w}; use content, looks, video, sound or archive")))?;
        let t = p.thresholds.as_ref().and_then(|t| t.get(&w).copied()).unwrap_or(way.default_threshold());
        if !(0.0..=1.0).contains(&t) {
            return Err(bad(format!("The threshold for {w} is from 0 to 1")));
        }
        if !ways.iter().any(|(x, _)| *x == way) {
            ways.push((way, t));
        }
    }
    if ways.is_empty() {
        return Err(bad("Give at least one way to compare"));
    }
    if let Some(k) = p.thresholds.as_ref().and_then(|t| t.keys().find(|k| Way::parse(k).is_none()).cloned()) {
        return Err(bad(format!("There is no way {k}; use looks, video, sound or archive")));
    }
    let look_kinds = match &p.look_kinds {
        Some(list) => list.iter().map(|k| AssetKind::ALL.iter().copied().find(|x| x.as_str() == k).ok_or_else(|| bad(format!("There is no kind {k}")))).collect::<Result<Vec<_>, _>>()?,
        None => AssetKind::ALL.iter().copied().filter(|k| !matches!(k, AssetKind::Video | AssetKind::Audio)).collect(),
    };
    let scoped = p.ids.is_some() || p.folder.is_some() || p.query.as_deref().is_some_and(|q| !q.trim().is_empty());
    if p.with_library && !scoped {
        return Err(bad("`withLibrary` compares some items with the library: give `ids`, `folder` or `query`"));
    }
    let scope = if scoped {
        if let Some(f) = p.folder {
            folders_exist(c, &[f])?;
        }
        let by_view = if p.folder.is_some() || p.query.as_deref().is_some_and(|q| !q.trim().is_empty()) {
            Some(found(c, p.folder.map_or(Scope::All, Scope::Folder), p.query.as_deref().unwrap_or(""), Sort::default(), true)?)
        } else {
            None
        };
        let given = match &p.ids {
            Some(refs) => Some(items_of(c, refs)?),
            None => None,
        };
        Some(match (given, by_view) {
            (Some(g), Some(v)) => {
                let v: std::collections::HashSet<AssetId> = v.into_iter().collect();
                g.into_iter().filter(|i| v.contains(i)).collect()
            }
            (Some(g), None) => g,
            (None, Some(v)) => v,
            (None, None) => vec![],
        })
    } else {
        None
    };
    let search = duplicates::Search { ways, look_kinds, scope, with_library: p.with_library, min_size: p.min_size.unwrap_or(0).max(0) };
    let (job, stop) = crate::core::api::jobs::start(c.host.events(), c.caller.plugin.as_deref(), "Looking for duplicates", 0.0);
    let (j, plugin) = (job.clone(), c.caller.plugin.clone());
    let queued = with_library(c.host, |lib| {
        let (jp, jd, plugin) = (j.clone(), j.clone(), plugin.clone());
        let progress = Box::new(move |done: f64, total: f64, text: &str| {
            let _ = crate::core::api::jobs::progress(&jp, None, Some(done), Some(total), Some(text));
        });
        let done = Box::new(move |db: Option<&Db>, r: Result<duplicates::Found, String>| {
            let r = r.and_then(|f| match db {
                Some(db) => found_json(db, &f),
                None => Err("The library closed".into()),
            });
            let _ = match r {
                Ok(v) => crate::core::api::jobs::finish(&jd, plugin.as_deref(), Some(v), None),
                Err(e) => crate::core::api::jobs::finish(&jd, plugin.as_deref(), None, Some(e)),
            };
        });
        duplicates::spawn(lib.ctx.clone(), search, stop.clone(), progress, done);
        Ok(())
    });
    if let Err(e) = queued {
        let _ = crate::core::api::jobs::finish(&job, None, None, Some(e.clone()));
        return Err(ApiError::new("failed", e));
    }
    Ok(json!({ "job": job }))
}

/// A search's groups and what it left out, items named by their stable ids.
fn found_json(db: &Db, f: &crate::core::duplicates::Found) -> Result<Value, String> {
    let mut ids: Vec<AssetId> = f.groups.iter().flat_map(|g| g.members.iter().map(|m| m.id)).collect();
    ids.extend(f.unreadable.iter().map(|u| u.0));
    let assets: HashMap<AssetId, Asset> = db.assets(&ids)?.into_iter().map(|a| (a.id, a)).collect();
    let uid = |id: &AssetId| assets.get(id).map(|a| a.uid.clone());
    let groups: Vec<Value> = f
        .groups
        .iter()
        .filter(|g| g.members.iter().all(|m| assets.contains_key(&m.id)))
        .map(|g| {
            let members: Vec<Value> = g.members.iter().map(|m| json!({"id": uid(&m.id), "same": m.same, "file": m.file, "store": m.store})).collect();
            let links: Vec<Value> = g.links.iter().map(|(a, b, w, s)| json!([a, b, w.name(), s])).collect();
            json!({"members": members, "links": links})
        })
        .collect();
    let checked: serde_json::Map<String, Value> = f.checked.iter().map(|(w, n)| (w.name().to_string(), json!(n))).collect();
    let unreadable: Vec<Value> = f.unreadable.iter().filter_map(|(id, why)| assets.get(id).map(|a| json!({"id": a.uid, "name": a.title(), "error": why}))).collect();
    Ok(json!({
        "groups": groups, "checked": checked, "missing": f.missing, "noThumbnail": f.no_thumbnail, "noFfmpeg": f.no_ffmpeg, "plain": f.plain,
        "unreadable": unreadable, "unreadableCount": f.unreadable_count, "stopped": f.stopped, "took": f.took_ms,
    }))
}

#[derive(Deserialize, JsonSchema)]
struct ResolveGroup {
    /// The item kept.
    keep: ItemRef,
    /// The copies moved to the trash.
    remove: Vec<ItemRef>,
    /// Those of the copies a search found identical to the kept item: each is compared with the kept file again (in full up to 8 MiB, at its start, middle and end beyond) and left alone if they differ.
    #[serde(default)]
    identical: Vec<ItemRef>,
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct ResolveParams {
    /// Up to 1000 groups.
    groups: Vec<ResolveGroup>,
    /// The kept item takes its copies' tags, folders, best rating, notes, source address and user fields; true unless set.
    #[serde(default)]
    combine: Option<bool>,
}

pub(super) fn duplicates_resolve(c: &Ctx, p: ResolveParams) -> ApiResult {
    if p.groups.len() > 1000 {
        return Err(bad("At most 1000 groups a call"));
    }
    let mut plan = Vec::new();
    let mut skipped: Vec<(AssetId, String)> = Vec::new();
    for g in &p.groups {
        let keep = one_id(c, &g.keep)?;
        let remove = ids_of(c, &g.remove)?;
        let identical = ids_of(c, &g.identical)?;
        // the files are read here, before the writer is asked for anything
        let (settle, left) = read(c.host, |db| crate::core::duplicates::resolution::check(db, keep, &remove, &identical))?;
        skipped.extend(left);
        plan.extend(settle);
    }
    let combine = p.combine.unwrap_or(true);
    let kept = plan.len();
    let removed = if plan.is_empty() { 0 } else { write_for(c, move |db| crate::core::duplicates::resolution::settle(db, &plan, combine))? };
    let names: HashMap<AssetId, String> = read(c.host, |db| Ok(db.uids(&skipped.iter().map(|s| s.0).collect::<Vec<_>>())?.into_iter().collect()))?;
    let skipped: Vec<Value> = skipped.iter().map(|(id, why)| json!({"id": names.get(id).cloned().unwrap_or_default(), "reason": why})).collect();
    Ok(json!({"removed": removed, "kept": kept, "skipped": skipped}))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct IgnoreParams {
    /// Two or more items, none of which is a duplicate of another.
    ids: Vec<ItemRef>,
}

pub(super) fn duplicates_ignore(c: &Ctx, p: IgnoreParams) -> ApiResult {
    let ids = items_of(c, &p.ids)?;
    if ids.len() < 2 {
        return Err(bad("Give two or more items"));
    }
    let pairs = write_for(c, move |db| Ok((crate::core::duplicates::resolution::ignore(db, &ids)?, Changed::default())))?;
    Ok(json!({ "pairs": pairs }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct UnignoreParams {
    /// The items whose marks go; every mark unless set.
    #[serde(default)]
    ids: Option<Vec<ItemRef>>,
}

pub(super) fn duplicates_unignore(c: &Ctx, p: UnignoreParams) -> ApiResult {
    let ids = match &p.ids {
        Some(refs) => Some(items_of(c, refs)?),
        None => None,
    };
    let pairs = write_for(c, move |db| Ok((crate::core::duplicates::resolution::unignore(db, ids.as_deref())?, Changed::default())))?;
    Ok(json!({ "pairs": pairs }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct StatsParams {
    /// kind, ext, folder, tag or month.
    by: String,
    /// Only the items this query finds; every item unless set.
    #[serde(default)]
    query: String,
}

pub(super) fn library_stats(c: &Ctx, p: StatsParams) -> ApiResult {
    use rusqlite::params_from_iter;
    read_checked(c, |db| {
        let cat = db.field_catalog();
        let q = match crate::core::query::compile::compile_str(&p.query, &cat) {
            Ok(q) => q,
            Err(e) => return Ok(Err(bad(e))),
        };
        let filter = format!("a.deleted = 0 AND {} AND ({})", crate::core::db::ITEMS, q.sql);
        let sql = match p.by.as_str() {
            "kind" => format!("SELECT a.kind, a.kind, count(*), sum(a.size) FROM assets a WHERE {filter} GROUP BY a.kind ORDER BY count(*) DESC"),
            "ext" => format!("SELECT lower(a.ext), lower(a.ext), count(*), sum(a.size) FROM assets a WHERE {filter} GROUP BY lower(a.ext) ORDER BY count(*) DESC"),
            "tag" => format!("SELECT t.id, t.name, count(*), sum(a.size) FROM assets a JOIN asset_tags at ON at.asset_id = a.id JOIN tags t ON t.id = at.tag_id WHERE {filter} GROUP BY t.id ORDER BY count(*) DESC"),
            "month" => format!("SELECT m, m, count(*), sum(size) FROM (SELECT strftime('%Y-%m', a.added_at / 1000, 'unixepoch', 'localtime') m, a.size FROM assets a WHERE {filter}) GROUP BY m ORDER BY m"),
            // each folder counts what is in it and in its subfolders, each item once
            "folder" => format!(
                "WITH RECURSIVE under(f, top) AS (SELECT id, id FROM folders UNION ALL SELECT fo.id, under.top FROM folders fo JOIN under ON fo.parent_id = under.f)
                 SELECT x.top, (SELECT name FROM folders WHERE id = x.top), count(*), sum(x.size) FROM
                   (SELECT DISTINCT under.top, a.id, a.size FROM under JOIN asset_folders af ON af.folder_id = under.f JOIN assets a ON a.id = af.asset_id WHERE {filter}) x
                 GROUP BY x.top ORDER BY count(*) DESC"
            ),
            other => return Ok(Err(bad(format!("`by` is kind, ext, folder, tag or month, not {other}")))),
        };
        let groups = {
            let mut st = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = st.query_map(params_from_iter(q.params.iter()), |r| {
                let key: rusqlite::types::Value = r.get(0)?;
                let key = match key {
                    rusqlite::types::Value::Integer(n) => n.to_string(),
                    rusqlite::types::Value::Text(s) => s,
                    _ => String::new(),
                };
                Ok(json!({"key": key, "name": r.get::<_, Option<String>>(1)?.unwrap_or_default(), "count": r.get::<_, i64>(2)?, "bytes": r.get::<_, Option<i64>>(3)?.unwrap_or(0)}))
            }).map_err(|e| e.to_string())?;
            rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
        };
        let (count, bytes): (i64, Option<i64>) = db.conn.query_row(&format!("SELECT count(*), sum(a.size) FROM assets a WHERE {filter}"), params_from_iter(q.params.iter()), |r| Ok((r.get(0)?, r.get(1)?))).map_err(|e| e.to_string())?;
        Ok(Ok(json!({"by": p.by, "total": {"count": count, "bytes": bytes.unwrap_or(0)}, "groups": groups})))
    })
}

pub(super) fn app_fonts(_: &Ctx, _: Nothing) -> ApiResult {
    Ok(json!(crate::core::system::filesystem::font_families()))
}
