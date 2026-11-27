//! Finding items: queries, counts and an item's details.

use std::collections::HashMap;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::core::api::{read, ApiError, ApiResult, Ctx};
use crate::core::model::*;
use crate::core::api::operations::items_with_fields;
use crate::core::api::operations::ids_of;
use crate::core::api::operations::ItemRef;
use crate::core::api::operations::item_json;
use crate::core::api::operations::MAX_LIMIT;
use crate::core::api::operations::read_checked;
use crate::core::api::operations::bad;
use crate::core::api::operations::file_missing;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct QueryParams {
    /// Query language, e.g. `model = "Anima" AND steps <= 12`; empty for every item.
    #[serde(default)]
    query: String,
    /// all (default), uncategorized, untagged, recent, trash or missing.
    #[serde(default)]
    scope: Option<String>,
    /// imported (default), modified, created, name, size, rating, resolution, duration or field:<key>.
    #[serde(default)]
    sort: Option<String>,
    /// Newest, largest or highest first; true by default.
    #[serde(default)]
    descending: Option<bool>,
    #[serde(default)]
    offset: Option<usize>,
    /// 50 by default, at most 1000.
    #[serde(default)]
    limit: Option<usize>,
}

pub(super) fn scope_of(s: Option<&str>) -> Result<Scope, ApiError> {
    Ok(match s.unwrap_or("all") {
        "all" => Scope::All,
        "uncategorized" => Scope::Uncategorized,
        "untagged" => Scope::Untagged,
        "recent" => Scope::Recent,
        "trash" => Scope::Trash,
        "missing" => Scope::Missing,
        other => return Err(bad(format!("Unknown scope {other}"))),
    })
}

pub(super) fn sort_of(s: Option<&str>, descending: Option<bool>) -> Result<Sort, ApiError> {
    let s = s.unwrap_or("imported");
    let (key, field) = s.split_once(':').unwrap_or((s, ""));
    let key = match key {
        "imported" => SortKey::Imported,
        "modified" => SortKey::Modified,
        "created" => SortKey::Created,
        "name" => SortKey::Name,
        "size" => SortKey::Size,
        "rating" => SortKey::Rating,
        "resolution" => SortKey::Resolution,
        "duration" => SortKey::Duration,
        "field" if !field.is_empty() => SortKey::Field(field.to_string()),
        _ => return Err(bad(format!("Unknown sort {s}"))),
    };
    Ok(Sort { key, descending: descending.unwrap_or(true) })
}

pub(super) fn found(c: &Ctx, scope: Scope, q: &str, sort: Sort, subfolders: bool) -> Result<Vec<AssetId>, ApiError> {
    read_checked(c, |db| {
        let cat = db.field_catalog();
        if let Err(e) = crate::core::query::compile::compile_str(q, &cat) {
            return Ok(Err(bad(e)));
        }
        Ok(Ok(db.search(&scope, q, &sort, subfolders, &cat)?.into_iter().map(|r| r.0).collect()))
    })
}

pub(super) fn page(c: &Ctx, ids: Vec<AssetId>, offset: usize, limit: usize) -> ApiResult {
    let total = ids.len();
    let shown: Vec<AssetId> = ids.into_iter().skip(offset).take(limit.min(MAX_LIMIT)).collect();
    let items = read(c.host, |db| {
        let assets = db.assets(&shown)?;
        let by_id: HashMap<AssetId, &Asset> = assets.iter().map(|x| (x.id, x)).collect();
        Ok(shown.iter().filter_map(|i| by_id.get(i)).map(|a| item_json(c, db, a)).collect::<Vec<_>>())
    })?;
    Ok(json!({"total": total, "offset": offset, "items": items}))
}

pub(super) fn item_query(c: &Ctx, p: QueryParams) -> ApiResult {
    let ids = found(c, scope_of(p.scope.as_deref())?, &p.query, sort_of(p.sort.as_deref(), p.descending)?, false)?;
    page(c, ids, p.offset.unwrap_or(0), p.limit.unwrap_or(50))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct CountParams {
    #[serde(default)]
    query: String,
    /// all (default), uncategorized, untagged, recent, trash or missing.
    #[serde(default)]
    scope: Option<String>,
}

pub(super) fn item_count(c: &Ctx, p: CountParams) -> ApiResult {
    Ok(json!({"count": found(c, scope_of(p.scope.as_deref())?, &p.query, Sort::default(), false)?.len()}))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct OneItem {
    /// The item's id.
    pub(super) id: ItemRef,
}

pub(super) fn item_get(c: &Ctx, p: OneItem) -> ApiResult {
    let ids = ids_of(c, &[p.id])?;
    Ok(items_with_fields(c, &ids)?.into_iter().next().unwrap_or(Value::Null))
}

/// The most text `item.exif` and `item.pngText` answer, so a page can't be handed a whole file of it.
const MAX_TEXT: usize = 16 << 20;

/// An item's own file on disk, which must be there.
fn file_of(c: &Ctx, r: &ItemRef) -> Result<std::path::PathBuf, ApiError> {
    let id = ids_of(c, std::slice::from_ref(r))?[0];
    read_checked(c, |db| Ok(Ok(db.abs_path(id))))?.filter(|p| p.is_file()).ok_or_else(file_missing)
}

pub(super) fn item_exif(c: &Ctx, p: OneItem) -> ApiResult {
    let path = file_of(c, &p.id)?;
    Ok(json!({ "tags": crate::core::metadata::extract::exif_tags(&path, MAX_TEXT).unwrap_or_default() }))
}

pub(super) fn item_png_text(c: &Ctx, p: OneItem) -> ApiResult {
    let path = file_of(c, &p.id)?;
    let mut chunks = serde_json::Map::new();
    let mut size = 0;
    if let Ok(all) = crate::core::metadata::extract::png_text_chunks(&path) {
        let mut all: Vec<(String, String)> = all.into_iter().collect();
        all.sort();
        for (k, v) in all {
            size += v.len();
            if size > MAX_TEXT {
                break;
            }
            chunks.insert(k, Value::String(v));
        }
    }
    Ok(json!({ "chunks": chunks }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct Items {
    /// Item ids.
    pub(super) ids: Vec<ItemRef>,
}

pub(super) fn item_get_many(c: &Ctx, p: Items) -> ApiResult {
    let ids = ids_of(c, &p.ids)?;
    Ok(Value::Array(items_with_fields(c, &ids)?))
}

pub(super) fn query_validate(c: &Ctx, p: CountParams) -> ApiResult {
    let scope = scope_of(p.scope.as_deref())?;
    Ok(read(c.host, |db| {
        let cat = db.field_catalog();
        Ok(match crate::core::query::compile::compile_str(&p.query, &cat) {
            Err(e) => json!({"valid": false, "error": e}),
            Ok(_) => json!({"valid": true, "count": db.search(&scope, &p.query, &Sort::default(), false, &cat)?.len()}),
        })
    })?)
}

pub(super) fn query_facets(c: &Ctx, p: CountParams) -> ApiResult {
    let ids = found(c, scope_of(p.scope.as_deref())?, &p.query, Sort::default(), false)?;
    Ok(read(c.host, |db| db.filter_facets(&ids))?)
}
