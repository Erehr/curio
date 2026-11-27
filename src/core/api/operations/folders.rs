//! Folders: their tree and what they hold.

use crate::core::api::operations::write;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use crate::core::api::{read, ApiError, ApiResult, Ctx};
use crate::core::library::{changes::Changed, changes::Parts};
use crate::core::model::*;
use crate::core::library::edits::{self, FolderArgs};
use crate::core::api::operations::write_for;
use crate::core::api::operations::bad;
use crate::core::api::operations::item_reads::page;
use crate::core::api::operations::item_reads::sort_of;
use crate::core::api::operations::item_reads::found;
use crate::core::api::operations::item_edits::folders_exist;
use crate::core::api::operations::not_found;
use crate::core::api::operations::to_json;
use crate::core::api::operations::libraries::Nothing;

pub(super) fn folder_list(c: &Ctx, _: Nothing) -> ApiResult {
    to_json(read(c.host, |db| db.folders())?)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct ById {
    pub(super) id: i64,
}

pub(super) fn folder_get(c: &Ctx, p: ById) -> ApiResult {
    let all = read(c.host, |db| db.folders())?;
    let f = all.iter().find(|f| f.id == p.id).ok_or_else(|| not_found(format!("There is no folder {}", p.id)))?;
    let mut v = to_json(f)?;
    v["children"] = json!(all.iter().filter(|x| x.parent_id == Some(p.id)).map(|x| x.id).collect::<Vec<_>>());
    Ok(v)
}

pub(super) fn folder_recent(c: &Ctx, _: Nothing) -> ApiResult {
    let recent = c.host.recent_folders();
    let all = read(c.host, |db| db.folders())?;
    to_json(recent.iter().filter_map(|id| all.iter().find(|f| f.id == *id)).collect::<Vec<_>>())
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct FolderPage {
    id: i64,
    /// Include the subfolders' items; true by default.
    #[serde(default)]
    subfolders: Option<bool>,
    /// imported (default), modified, created, name, size, rating, resolution, duration or field:<key>.
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    descending: Option<bool>,
    #[serde(default)]
    offset: Option<usize>,
    /// 50 by default, at most 1000.
    #[serde(default)]
    limit: Option<usize>,
}

pub(super) fn folder_items(c: &Ctx, p: FolderPage) -> ApiResult {
    folders_exist(c, &[p.id])?;
    let ids = found(c, Scope::Folder(p.id), "", sort_of(p.sort.as_deref(), p.descending)?, p.subfolders.unwrap_or(true))?;
    page(c, ids, p.offset.unwrap_or(0), p.limit.unwrap_or(50))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct FolderCreate {
    name: String,
    /// The folder to make it in; top level without one.
    #[serde(default)]
    parent: Option<i64>,
}

/// A name for a folder, smart folder or tag group: trimmed, never empty.
pub(super) fn given_name(name: &str) -> Result<String, ApiError> {
    let n = name.trim();
    if n.is_empty() {
        return Err(bad("Give it a name"));
    }
    Ok(n.to_string())
}

pub(super) fn folder_create(c: &Ctx, p: FolderCreate) -> ApiResult {
    let name = given_name(&p.name)?;
    if let Some(parent) = p.parent {
        folders_exist(c, &[parent])?;
    }
    let id = write_for(c, move |db| Ok((db.create_folder(&name, p.parent)?, Changed::looks(Parts::FOLDERS))))?;
    Ok(json!({"success": true, "id": id}))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct FolderRename {
    id: i64,
    name: String,
}

pub(super) fn folder_rename(c: &Ctx, p: FolderRename) -> ApiResult {
    let name = given_name(&p.name)?;
    write(c, move |db| edits::folder_op(db, "rename", FolderArgs { id: Some(p.id), name: Some(name), ..Default::default() }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct FolderMove {
    id: i64,
    /// The new parent folder; null or missing for the top level.
    #[serde(default)]
    parent: Option<i64>,
}

pub(super) fn folder_move(c: &Ctx, p: FolderMove) -> ApiResult {
    write(c, move |db| edits::folder_op(db, "move", FolderArgs { id: Some(p.id), parent: p.parent, ..Default::default() }))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct FolderUpdate {
    id: i64,
    /// A CSS colour; empty clears it.
    #[serde(default)]
    color: Option<String>,
    /// A Tabler icon name; empty clears it.
    #[serde(default)]
    icon: Option<String>,
    /// Tags given to items added to the folder.
    #[serde(default)]
    auto_tags: Option<Vec<String>>,
}

pub(super) fn folder_update(c: &Ctx, p: FolderUpdate) -> ApiResult {
    write(c, move |db| {
        let args = |v: Option<String>| FolderArgs { id: Some(p.id), value: v.filter(|s| !s.is_empty()), ..Default::default() };
        if let Some(col) = &p.color {
            edits::folder_op(db, "color", args(Some(col.clone())))?;
        }
        if let Some(i) = &p.icon {
            edits::folder_op(db, "icon", args(Some(i.clone())))?;
        }
        if let Some(t) = &p.auto_tags {
            edits::folder_op(db, "auto_tags", FolderArgs { id: Some(p.id), value: Some(t.join(",")), ..Default::default() })?;
        }
        // a search can ask for a folder's colour
        Ok(if p.color.is_some() { Changed::parts(Parts::FOLDERS) } else { Changed::looks(Parts::FOLDERS) })
    })
}

pub(super) fn folder_delete(c: &Ctx, p: ById) -> ApiResult {
    write(c, move |db| edits::folder_op(db, "delete", FolderArgs { id: Some(p.id), ..Default::default() }))
}
