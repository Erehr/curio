//! The app and libraries namespaces: the app's own information and the libraries a caller may reach.

use std::path::{Path, PathBuf};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::core::api::{library_named, read, ApiError, ApiResult, Ctx, KnownLibrary, LibraryChange, Perm};
use crate::core::db::Db;
use crate::core::library::instance;
use crate::core::model::*;
use crate::core::api::operations::said;
use crate::core::api::operations::done;
use crate::core::api::operations::bad;

#[derive(Deserialize, JsonSchema)]
pub(super) struct Nothing {}

pub(super) fn app_info(c: &Ctx, _: Nothing) -> ApiResult {
    Ok(json!({
        "name": "Curio", "version": env!("CURIO_VERSION"), "apiVersion": crate::core::api::API_VERSION,
        "platform": std::env::consts::OS, "arch": std::env::consts::ARCH, "language": c.host.language(),
    }))
}

pub(super) fn app_operations(c: &Ctx, _: Nothing) -> ApiResult {
    let list: Vec<Value> = crate::core::api::registry()
        .iter()
        .filter(|o| o.callers.contains(&c.caller.kind) && c.caller.may(o.perm))
        .map(|o| json!({"name": o.name, "description": o.doc, "permission": o.perm.name(), "kind": o.kind, "params": o.public_params()}))
        .collect();
    Ok(Value::Array(list))
}

pub(super) fn library_list(c: &Ctx, _: Nothing) -> ApiResult {
    Ok(crate::core::api::library_list(c.host, c.caller))
}

pub(super) fn library_info(c: &Ctx, _: Nothing) -> ApiResult {
    let path = c.host.library();
    let mut v = read(c.host, |db| {
        Ok(json!({
            "id": db.meta_get("library_id"), "name": path.as_deref().map(instance::library_name),
            "schema": db.schema_version(), "floor": db.schema_floor(), "counts": db.counts()?,
        }))
    })?;
    if c.caller.may(Perm::FilesRead) {
        v["path"] = json!(path);
    }
    Ok(v)
}

fn entry<'a>(c: &'a Ctx) -> Result<&'a KnownLibrary, ApiError> {
    c.entry.ok_or_else(|| bad("Name the library with `library`"))
}

fn named(c: &Ctx, path: &Path) -> Value {
    let mut v = json!({"success": true, "name": instance::library_name(path)});
    if c.caller.may(Perm::FilesRead) {
        v["path"] = json!(path);
    }
    v
}

pub(super) fn library_open(c: &Ctx, _: Nothing) -> ApiResult {
    c.host.change_library(LibraryChange::Open(entry(c)?.path.clone()))?;
    Ok(done())
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct LibraryCreate {
    name: String,
    /// The folder to make it in; Curio's own Libraries folder without one.
    #[serde(default)]
    folder: Option<String>,
    /// Open it in a window of its own once made.
    #[serde(default)]
    open: bool,
}

pub(super) fn library_create(c: &Ctx, p: LibraryCreate) -> ApiResult {
    let name = p.name.trim();
    if name.is_empty() {
        return Err(said("bad_request", "errors.library.noName", Value::Null, "The library needs a name"));
    }
    let base = p.folder.map(PathBuf::from).unwrap_or_else(|| c.host.data_dir().join("Libraries"));
    let path = base.join(format!("{}{}", crate::core::import::sanitize_name(name), instance::LIBRARY_SUFFIX));
    if path.exists() {
        return Err(said("bad_request", "errors.library.exists", json!({"name": instance::library_name(&path)}), format!("{} already exists", instance::library_name(&path))));
    }
    std::fs::create_dir_all(&path).map_err(|e| said("failed", "errors.library.folderFailed", json!({"error": e.to_string()}), format!("Couldn't make the library's folder: {e}")))?;
    let id = new_uid();
    Db::open(&path.join("library.db"))?.meta_set("library_id", &id)?;
    c.host.change_library(LibraryChange::Add { path: path.clone(), id: id.clone() })?;
    if p.open {
        c.host.change_library(LibraryChange::Open(path.clone()))?;
    }
    let mut v = named(c, &path);
    v["id"] = json!(id);
    Ok(v)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct NewLibraryName {
    /// The new name; the folder becomes `<name>.curio` beside the old one.
    name: String,
}

pub(super) fn library_rename(c: &Ctx, p: NewLibraryName) -> ApiResult {
    let to = c.host.change_library(LibraryChange::Rename(entry(c)?.path.clone(), p.name))?;
    Ok(named(c, &to))
}

pub(super) fn library_forget(c: &Ctx, _: Nothing) -> ApiResult {
    c.host.change_library(LibraryChange::Forget(entry(c)?.path.clone()))?;
    Ok(done())
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct LibraryPlace {
    /// The library it goes just before; last without one.
    #[serde(default)]
    before: Option<String>,
}

pub(super) fn library_place(c: &Ctx, p: LibraryPlace) -> ApiResult {
    let before = p.before.map(|b| library_named(c.host, c.caller, &b).map(|l| l.path)).transpose()?;
    c.host.change_library(LibraryChange::Place(entry(c)?.path.clone(), before))?;
    Ok(done())
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct Look {
    /// A Tabler icon name; empty clears it.
    #[serde(default)]
    icon: Option<String>,
    /// A CSS colour; empty clears it.
    #[serde(default)]
    color: Option<String>,
}

pub(super) fn library_set_look(c: &Ctx, p: Look) -> ApiResult {
    c.host.change_library(LibraryChange::Look { path: entry(c)?.path.clone(), icon: p.icon, color: p.color })?;
    Ok(done())
}
