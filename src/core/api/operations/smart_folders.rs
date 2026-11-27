//! Smart folders: saved queries in a tree.

use crate::core::api::operations::write;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use crate::core::api::{read, ApiError, ApiResult, Ctx};
use crate::core::library::{changes::Changed, changes::Parts};
use crate::core::model::*;
use crate::core::library::edits::{self, SmartArgs};
use crate::core::api::operations::item_reads::page;
use crate::core::api::operations::item_reads::found;
use crate::core::api::operations::write_for;
use crate::core::api::operations::folders::given_name;
use crate::core::api::operations::bad;
use crate::core::api::operations::not_found;
use crate::core::api::operations::folders::ById;
use crate::core::api::operations::to_json;
use crate::core::api::operations::libraries::Nothing;

pub(super) fn smart_list(c: &Ctx, _: Nothing) -> ApiResult {
    to_json(read(c.host, |db| db.smart_folders())?)
}

pub(super) fn smart_get(c: &Ctx, p: ById) -> ApiResult {
    let all = read(c.host, |db| db.smart_folders_counted())?;
    to_json(all.into_iter().find(|s| s.id == p.id).ok_or_else(|| not_found(format!("There is no smart folder {}", p.id)))?)
}

fn check_query(c: &Ctx, q: Option<&str>) -> Result<(), ApiError> {
    let Some(q) = q else { return Ok(()) };
    let r = read(c.host, |db| Ok(crate::core::query::compile::compile_str(q, &db.field_catalog()).map(|_| ())))?;
    r.map_err(bad)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct SmartCreate {
    name: String,
    query: String,
    /// A smart folder to narrow.
    #[serde(default)]
    parent_id: Option<i64>,
    /// Virtual subfolders grouped by a property or field (e.g. `year:added`, `rating`, `field:genre`).
    #[serde(default)]
    group_by: Option<String>,
}

pub(super) fn smart_create(c: &Ctx, p: SmartCreate) -> ApiResult {
    check_query(c, Some(&p.query))?;
    let name = given_name(&p.name)?;
    // a new subfolder changes what a parent without rules shows
    let id = write_for(c, move |db| Ok((db.create_smart_folder_in(&name, &p.query, p.parent_id, p.group_by.as_deref())?, Changed::parts(Parts::SMART))))?;
    Ok(json!({"success": true, "id": id}))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct SmartUpdate {
    id: i64,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    query: Option<String>,
    /// A Tabler icon name; empty clears it.
    #[serde(default)]
    icon: Option<String>,
    /// Virtual subfolders grouped by a property or field; empty clears it.
    #[serde(default)]
    group_by: Option<String>,
}

pub(super) fn smart_update(c: &Ctx, mut p: SmartUpdate) -> ApiResult {
    check_query(c, p.query.as_deref())?;
    p.name = p.name.as_deref().map(given_name).transpose()?;
    write(c, move |db| {
        let requery = p.query.is_some() || p.group_by.is_some();
        if p.name.is_some() || p.query.is_some() || p.group_by.is_some() {
            edits::smart_op(db, "update", SmartArgs { id: Some(p.id), name: p.name, query: p.query, group_by: p.group_by, ..Default::default() })?;
        }
        if let Some(i) = p.icon {
            edits::smart_op(db, "icon", SmartArgs { id: Some(p.id), icon: Some(i).filter(|s| !s.is_empty()), ..Default::default() })?;
        }
        Ok(if requery { Changed::parts(Parts::SMART) } else { Changed::looks(Parts::SMART) })
    })
}

pub(super) fn smart_delete(c: &Ctx, p: ById) -> ApiResult {
    write(c, move |db| edits::smart_op(db, "delete", SmartArgs { id: Some(p.id), ..Default::default() }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct SmartItems {
    id: i64,
    #[serde(default)]
    offset: Option<usize>,
    /// 50 by default, at most 1000.
    #[serde(default)]
    limit: Option<usize>,
}

pub(super) fn smart_items(c: &Ctx, p: SmartItems) -> ApiResult {
    let ids = found(c, Scope::SmartFolder(p.id), "", Sort::default(), false)?;
    page(c, ids, p.offset.unwrap_or(0), p.limit.unwrap_or(50))
}
