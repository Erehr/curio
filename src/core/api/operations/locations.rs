//! Locations: the folders a library reads.

use crate::core::api::operations::write;
use std::path::PathBuf;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::core::api::{read, with_library, ApiError, ApiResult, Ctx, Perm};
use crate::core::library::instance;
use crate::core::library::{changes::Changed, changes::Parts};
use crate::core::model::*;
use crate::core::api::operations::done;
use crate::core::api::operations::WRITE_WAIT;
use crate::core::api::operations::job_for;
use crate::core::api::operations::folders::ById;
use crate::core::api::operations::windows::short;
use crate::core::api::operations::bad;
use crate::core::api::operations::write_for;
use crate::core::api::operations::said;
use crate::core::api::operations::libraries::Nothing;
use crate::core::api::operations::not_found;
use crate::core::api::operations::read_checked;

/// A folder indexed in place; the library's own store and loose files' entries aren't locations.
pub(super) fn location_of(c: &Ctx, id: i64) -> Result<Root, ApiError> {
    read_checked(c, |db| Ok(db.root(id).filter(|r| r.listed).ok_or_else(|| not_found(format!("There is no location {id}")))))
}

pub(super) fn location_list(c: &Ctx, _: Nothing) -> ApiResult {
    let files = c.caller.may(Perm::FilesRead);
    let list = read(c.host, |db| Ok(db.roots()?.into_iter().filter(|r| r.listed).collect::<Vec<_>>()))?;
    Ok(Value::Array(
        list.into_iter()
            .map(|r| {
                let mut v = json!({
                    "id": r.id, "name": r.label, "online": r.online, "watch": r.watch, "access": r.access.as_str(),
                    "count": r.asset_count, "lastScan": r.last_scan, "icon": r.icon, "color": r.color,
                });
                if files {
                    v["path"] = json!(r.path);
                }
                v
            })
            .collect(),
    ))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct LocationAdd {
    /// A folder on this computer.
    path: String,
    /// Follow changes to its files as they happen; true by default.
    #[serde(default)]
    watch: Option<bool>,
    /// What Curio may do to its files: read_only, full or organize, set before its first scan. Full by default.
    #[serde(default)]
    access: Option<String>,
    /// Its name in the sidebar; the folder's own name by default.
    #[serde(default)]
    label: Option<String>,
}

pub(super) fn location_add(c: &Ctx, p: LocationAdd) -> ApiResult {
    let dir = PathBuf::from(&p.path);
    if !dir.is_dir() {
        return Err(said("bad_request", "errors.folders.none", json!({"path": p.path}), format!("There is no folder at {}", p.path)));
    }
    if instance::in_library_store(&dir) {
        return Err(said("bad_request", "errors.locations.inStore", Value::Null, "That folder is inside a library's own store"));
    }
    // an Eagle library becomes a location of its items, with its folders and tags
    if crate::core::import::eagle::is_eagle_library(&dir) {
        let id = write_for(c, move |db| {
            let s = crate::core::import::eagle::import_dot_library(db, &dir, None)?;
            Ok((s.location, Changed::structure()))
        })?;
        if let Some(id) = id {
            with_library(c.host, |lib| {
                lib.scan_root(id);
                Ok(())
            })?;
        }
        return Ok(json!({"success": true, "id": id}));
    }
    if p.access.as_deref().is_some_and(|a| !matches!(a, "read_only" | "full" | "organize")) {
        return Err(bad("Unknown access: read_only, full or organize"));
    }
    let label = p.label.map(|l| l.trim().to_string()).filter(|l| !l.is_empty());
    if let Some(l) = &label {
        short("label", l, 100)?;
    }
    let path = crate::core::system::filesystem::canonical(&dir).to_string_lossy().to_string();
    let watch = p.watch.unwrap_or(true);
    let access = p.access.map(|a| RootAccess::parse(&a));
    let id = write_for(c, move |db| {
        let id = db.add_root(&path, RootKind::Linked, false)?;
        db.set_root_flags(id, true, watch)?;
        if let Some(a) = access {
            db.set_root_access(id, a)?;
        }
        if let Some(l) = &label {
            db.set_root_label(id, l)?;
        }
        Ok((id, Changed::parts(Parts::LOCATIONS)))
    })?;
    with_library(c.host, |lib| {
        lib.scan_root(id);
        Ok(())
    })?;
    Ok(json!({"success": true, "id": id}))
}

pub(super) fn location_remove(c: &Ctx, p: ById) -> ApiResult {
    location_of(c, p.id)?;
    write(c, move |db| {
        let gone: Vec<AssetId> = db.conn.prepare("SELECT id FROM assets WHERE root_id = ?").and_then(|mut st| st.query_map([p.id], |r| r.get(0))?.collect()).map_err(|e| e.to_string())?;
        let uids = db.uids(&gone)?;
        db.remove_root(p.id, true)?;
        Ok(Changed::removed(&gone, true).with(Parts::LOCATIONS).naming(uids))
    })
}

pub(super) fn location_rescan(c: &Ctx, p: ById) -> ApiResult {
    location_of(c, p.id)?;
    with_library(c.host, |lib| {
        lib.scan_root(p.id);
        Ok(())
    })?;
    Ok(json!({"success": true, "queued": true}))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct LocationAccess {
    id: i64,
    /// read_only, full or organize.
    access: String,
}

pub(super) fn location_set_access(c: &Ctx, p: LocationAccess) -> ApiResult {
    if !matches!(p.access.as_str(), "read_only" | "full" | "organize") {
        return Err(bad(format!("Unknown access {}: read_only, full or organize", p.access)));
    }
    location_of(c, p.id)?;
    let access = RootAccess::parse(&p.access);
    write(c, move |db| {
        db.set_root_access(p.id, access)?;
        Ok(Changed::parts(Parts::LOCATIONS))
    })
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct LocationLook {
    id: i64,
    /// A Tabler icon name; empty clears it.
    #[serde(default)]
    icon: Option<String>,
    /// A CSS colour; empty clears it.
    #[serde(default)]
    color: Option<String>,
}

pub(super) fn location_set_look(c: &Ctx, p: LocationLook) -> ApiResult {
    location_of(c, p.id)?;
    write(c, move |db| {
        if let Some(i) = &p.icon {
            db.set_root_look(p.id, true, Some(i))?;
        }
        if let Some(col) = &p.color {
            db.set_root_look(p.id, false, Some(col))?;
        }
        Ok(Changed::looks(Parts::LOCATIONS))
    })
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct Relocate {
    id: i64,
    /// Where the folder is now.
    path: String,
}

pub(super) fn location_relocate(c: &Ctx, p: Relocate) -> ApiResult {
    location_of(c, p.id)?;
    let dir = PathBuf::from(&p.path);
    if !dir.is_dir() {
        return Err(said("bad_request", "errors.folders.none", json!({"path": p.path}), format!("There is no folder at {}", p.path)));
    }
    let np = crate::core::system::filesystem::canonical(&dir).to_string_lossy().to_string();
    let id = p.id;
    job_for(c, WRITE_WAIT, move |db, _| {
        db.relocate_root(id, &np)?;
        Ok(((), Changed::parts(Parts::LOCATIONS)))
    })?;
    with_library(c.host, |lib| {
        lib.scan_root(id);
        Ok(())
    })?;
    Ok(done())
}
