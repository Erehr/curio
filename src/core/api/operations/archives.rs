//! Archives: their entries listed and read.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::core::api::{file_reply, ApiError, ApiResult, Ctx};
use crate::core::model::*;
use crate::core::api::operations::item_adds::PathsParams;
use crate::core::api::operations::item_adds::item_add_from_paths;
use crate::core::api::operations::said;
use crate::core::api::operations::ids_of;
use crate::core::api::operations::item_adds::Placing;
use crate::core::api::operations::file_missing;
use crate::core::api::operations::bad;
use crate::core::api::operations::not_found;
use crate::core::api::operations::read_checked;
use crate::core::api::operations::one_id;
use crate::core::api::operations::ItemRef;

/// An archive item's file, when Curio reads it as one.
fn archive_file(c: &Ctx, id: &ItemRef) -> Result<(AssetId, PathBuf, String, i64), ApiError> {
    let id = one_id(c, id)?;
    let (path, ext, ver) = read_checked(c, |db| Ok(db.asset(id)?.map(|a| (db.abs_path(id), a.ext.to_lowercase(), a.thumb_ver)).ok_or_else(|| not_found("No such item"))))?;
    if !crate::core::archive::EXTS.contains(&ext.as_str()) {
        return Err(bad(format!("Curio reads ZIP, CBZ, EPUB, 7z and CB7 archives, not .{ext}")));
    }
    let path = path.filter(|p| p.is_file()).ok_or_else(|| file_missing())?;
    Ok((id, path, ext, ver))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct ArchiveList {
    /// The item's id.
    id: ItemRef,
    /// Whether pictures come with the size they show at and whether they move, read from the first bytes of each: `false` gives only names, sizes and kinds, which reads a small part of a large archive (counting pages, finding a cover).
    #[serde(default = "with_sizes")]
    sizes: bool,
}

fn with_sizes() -> bool {
    true
}

pub(super) fn archive_list(c: &Ctx, p: ArchiveList) -> ApiResult {
    let (_, path, _, _) = archive_file(c, &p.id)?;
    let a = crate::core::archive::open(&path).map_err(bad)?;
    // each picture's first bytes are a read of their own: hundreds in a comic, all over the file
    let looks = if p.sizes { a.looks() } else { HashMap::new() };
    let entries: Vec<Value> = a
        .entries()
        .iter()
        .map(|e| match looks.get(&e.name) {
            Some(l) => json!({"name": e.name, "size": e.size, "picture": e.picture, "width": l.width, "height": l.height, "animated": l.animated}),
            None => json!({"name": e.name, "size": e.size, "picture": e.picture}),
        })
        .collect();
    Ok(json!({ "entries": entries }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct EntryParams {
    id: ItemRef,
    /// The entry's name, as archive.list gives it.
    name: String,
}

/// Where entries are put for the answer, cleared of those an hour old.
fn entries_dir() -> PathBuf {
    let d = std::env::temp_dir().join("curio-entries");
    if let Ok(rd) = std::fs::read_dir(&d) {
        let hour = std::time::SystemTime::now() - Duration::from_secs(3600);
        for e in rd.flatten() {
            if e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t < hour) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    d
}

pub(super) fn archive_entry(c: &Ctx, p: EntryParams) -> ApiResult {
    let (_, path, _, _) = archive_file(c, &p.id)?;
    let bytes = crate::core::archive::read(&path, &p.name).map_err(not_found)?;
    let ext = Path::new(&p.name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let dir = entries_dir();
    std::fs::create_dir_all(&dir).map_err(|e| ApiError::new("failed", e.to_string()))?;
    let out = dir.join(format!("{}.{ext}", uuid::Uuid::new_v4().simple()));
    std::fs::write(&out, bytes).map_err(|e| ApiError::new("failed", e.to_string()))?;
    Ok(file_reply(&out, mime_for_ext(&ext)))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct EntryUrlParams {
    id: ItemRef,
    /// The entry's name, as archive.list gives it.
    name: String,
    /// For a picture: its long edge in pixels, scaled down by Curio; the entry as it is without it.
    #[serde(default)]
    size: Option<u32>,
}

fn encode_path(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/') { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

pub(super) fn archive_entry_url(c: &Ctx, p: EntryUrlParams) -> ApiResult {
    let (id, path, ext, ver) = archive_file(c, &p.id)?;
    if !crate::core::archive::open(&path).map_err(bad)?.entries().iter().any(|e| e.name == p.name) {
        return Err(not_found(format!("The archive has no {}", p.name)));
    }
    let url = c.host.page_file(id, &ext).ok_or_else(|| ApiError::new("not_allowed", "Only a plugin's page reads entries by address"))?;
    let base = url.rsplit_once('/').map_or(url.as_str(), |(b, _)| b);
    let size = p.size.map(|s| s.clamp(64, 8192)).unwrap_or(0);
    Ok(json!({ "url": format!("{base}/~entry/{size}/{}?v={ver}", encode_path(&p.name)) }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct ArchiveCreate {
    /// The items whose files go in, in this order: the pages of a comic.
    items: Vec<ItemRef>,
    /// The new item's file name; `.cbz` is added unless it ends with `.cbz` or `.zip`.
    name: String,
    #[serde(flatten)]
    place: Placing,
}

pub(super) fn archive_create(c: &Ctx, p: ArchiveCreate) -> ApiResult {
    if p.items.is_empty() {
        return Err(bad("No items given"));
    }
    let ids = ids_of(c, &p.items)?;
    let files: Vec<(String, PathBuf)> = read_checked(c, |db| {
        let assets = db.assets(&ids)?;
        let by: HashMap<AssetId, &Asset> = assets.iter().map(|a| (a.id, a)).collect();
        let width = ids.len().to_string().len().max(3);
        let mut out = Vec::new();
        for (i, id) in ids.iter().enumerate() {
            let Some(a) = by.get(id) else { return Ok(Err(not_found("No such item"))) };
            let Some(path) = db.abs_path(*id).filter(|p| p.is_file()) else { return Ok(Err(said("not_found", "errors.files.missingNamed", json!({"name": a.name}), format!("{}'s file is missing or offline", a.name)))) };
            out.push((format!("{:0width$}.{}", i + 1, a.ext.to_lowercase()), path));
        }
        Ok(Ok(out))
    })?;
    let mut name = p.name.trim().to_string();
    if name.is_empty() {
        return Err(bad("Give the new file a name"));
    }
    if !name.to_lowercase().ends_with(".cbz") && !name.to_lowercase().ends_with(".zip") {
        name.push_str(".cbz");
    }
    let name = crate::core::store::clean_name(&name, false);
    let dir = std::env::temp_dir().join("curio-runs").join(format!("pack-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&dir).map_err(|e| ApiError::new("failed", e.to_string()))?;
    let out = dir.join(&name);
    let made = crate::core::archive::create(&out, &files).map_err(|e| ApiError::new("failed", e));
    let r = made.and_then(|_| item_add_from_paths(c, PathsParams { paths: vec![out.to_string_lossy().to_string()], copy: false, take: true, on_clash: None, wait: false, place: p.place }));
    // an import still waiting its turn takes the file later
    if !r.as_ref().is_ok_and(|v| v["queued"] == true) {
        let _ = std::fs::remove_dir_all(&dir);
    }
    r
}
