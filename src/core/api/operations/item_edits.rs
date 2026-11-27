//! Editing items: their fields, names, groups, trash and files.

use crate::core::api::operations::write;
use std::collections::HashMap;
use std::path::PathBuf;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::core::api::{file_reply, library_named, read, same_path, with_library, ApiError, ApiResult, Ctx};
use crate::core::db::Db;
use crate::core::library::instance;
use crate::core::library::{changes::Aspect, changes::Changed, changes::Parts};
use crate::core::model::*;
use crate::core::library::edits::{self, FolderArgs};
use crate::core::store::OnClash;
use crate::core::thumbs;
use crate::core::api::operations::image_type;
use crate::core::api::operations::item_adds::how;
use crate::core::api::operations::unreachable_library;
use crate::core::api::operations::file_missing;
use crate::core::api::operations::base64_bytes;
use crate::core::api::operations::no_file;
use crate::core::api::operations::jobs::may_name;
use crate::core::api::operations::said;
use crate::core::api::operations::done;
use crate::core::api::operations::item_reads::OneItem;
use crate::core::api::operations::uids;
use crate::core::api::operations::FILE_WAIT;
use crate::core::api::operations::job_for;
use crate::core::api::operations::bad;
use crate::core::api::operations::one_id;
use crate::core::api::operations::item_reads::Items;
use crate::core::api::operations::ids_of;
use crate::core::api::operations::not_found;
use crate::core::api::operations::read_checked;
use crate::core::api::operations::either;
use crate::core::api::operations::items_of;
use crate::core::api::operations::ItemRef;

#[derive(Deserialize, JsonSchema)]
pub(super) struct UpdateParams {
    /// One item's id.
    #[serde(default)]
    id: Option<ItemRef>,
    /// Several items' ids, all given the same values.
    #[serde(default)]
    ids: Option<Vec<ItemRef>>,
    #[serde(default)]
    name: Option<String>,
    /// Notes.
    #[serde(default)]
    annotation: Option<String>,
    /// Where the item came from.
    #[serde(default)]
    url: Option<String>,
    /// 0 to 5.
    #[serde(default)]
    rating: Option<i64>,
    #[serde(default)]
    favorite: Option<bool>,
    #[serde(default)]
    explicit: Option<bool>,
    /// Custom field key to value; null clears a field.
    #[serde(default)]
    fields: Option<serde_json::Map<String, Value>>,
}

pub(super) fn item_update(c: &Ctx, p: UpdateParams) -> ApiResult {
    let ids = items_of(c, &either(p.id, p.ids)?)?;
    write(c, move |db| {
        if let Some(n) = &p.name {
            edits::set_text(db, &ids, "name", n)?;
        }
        if let Some(n) = &p.annotation {
            edits::set_text(db, &ids, "annotation", n)?;
        }
        if let Some(n) = &p.url {
            edits::set_text(db, &ids, "url", n)?;
        }
        if let Some(r) = p.rating {
            edits::set_rating(db, &ids, r)?;
        }
        if let Some(v) = p.favorite {
            edits::set_favorite(db, &ids, v)?;
        }
        if let Some(v) = p.explicit {
            edits::set_explicit(db, &ids, v)?;
        }
        for (k, v) in p.fields.iter().flatten() {
            edits::set_field(db, &ids, k, v)?;
        }
        let c = Changed::items(&ids);
        Ok(if p.favorite.is_some() { c.with(Parts::SIDEBAR) } else { c })
    })
}

#[derive(Deserialize, JsonSchema)]
struct NewName {
    id: ItemRef,
    name: String,
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct RenameItems {
    items: Vec<NewName>,
}

pub(super) fn item_rename(c: &Ctx, p: RenameItems) -> ApiResult {
    let refs: Vec<ItemRef> = p.items.iter().map(|i| i.id.clone()).collect();
    let ids = items_of(c, &refs)?;
    let names: Vec<String> = p.items.into_iter().map(|i| i.name).collect();
    write(c, move |db| {
        for (id, name) in ids.iter().zip(&names) {
            db.rename_item(*id, name)?;
        }
        Ok(Changed::items(&ids))
    })
}

/// A tag by name, or by its number in this library.
#[derive(Deserialize, JsonSchema, Clone)]
#[serde(untagged)]
pub(super) enum TagRef {
    Name(String),
    Id(i64),
}

pub(super) fn tag_ids(c: &Ctx, refs: &[TagRef]) -> Result<Vec<i64>, ApiError> {
    read_checked(c, |db| {
        let mut out = Vec::new();
        for r in refs {
            let id = match r {
                TagRef::Name(n) => db.tag_id(n),
                TagRef::Id(i) => db.conn.query_row("SELECT id FROM tags WHERE id = ?", [i], |r| r.get(0)).ok(),
            };
            match id {
                Some(i) => out.push(i),
                None => {
                    let shown = match r {
                        TagRef::Name(n) => n.clone(),
                        TagRef::Id(i) => i.to_string(),
                    };
                    return Ok(Err(not_found(format!("There is no tag {shown}"))));
                }
            }
        }
        Ok(Ok(out))
    })
}

pub(super) fn one_tag(c: &Ctx, r: &TagRef) -> Result<i64, ApiError> {
    Ok(tag_ids(c, std::slice::from_ref(r))?[0])
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct TagItems {
    ids: Vec<ItemRef>,
    /// Tag names.
    tags: Vec<String>,
}

pub(super) fn item_add_tags(c: &Ctx, p: TagItems) -> ApiResult {
    let ids = ids_of(c, &p.ids)?;
    write(c, move |db| edits::add_tags(db, &ids, &p.tags))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct UntagItems {
    ids: Vec<ItemRef>,
    /// Tags by name or number.
    #[serde(default)]
    tags: Vec<TagRef>,
    /// Every tag the items have.
    #[serde(default)]
    all: bool,
}

pub(super) fn item_remove_tags(c: &Ctx, p: UntagItems) -> ApiResult {
    let ids = ids_of(c, &p.ids)?;
    if p.all {
        return write(c, move |db| {
            db.clear_tags(&ids)?;
            Ok(Changed::items(&ids))
        });
    }
    let tags = tag_ids(c, &p.tags)?;
    write(c, move |db| edits::remove_tags(db, &ids, &tags))
}

pub(super) fn folders_exist(c: &Ctx, folders: &[i64]) -> Result<(), ApiError> {
    if folders.is_empty() {
        return Ok(());
    }
    let have: Vec<i64> = read(c.host, |db| Ok(db.folders()?.into_iter().map(|f| f.id).collect()))?;
    match folders.iter().find(|f| !have.contains(f)) {
        Some(f) => Err(not_found(format!("There is no folder {f}"))),
        None => Ok(()),
    }
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct FolderItems {
    ids: Vec<ItemRef>,
    /// Folder ids.
    folders: Vec<i64>,
}

fn folder_items_op(c: &Ctx, p: FolderItems, op: &'static str) -> ApiResult {
    let ids = ids_of(c, &p.ids)?;
    folders_exist(c, &p.folders)?;
    write(c, move |db| {
        for f in &p.folders {
            edits::folder_op(db, op, FolderArgs { id: Some(*f), ids: ids.clone(), ..Default::default() })?;
        }
        Ok(Changed::items(&ids))
    })
}

pub(super) fn item_add_to_folders(c: &Ctx, p: FolderItems) -> ApiResult {
    folder_items_op(c, p, "add")
}

pub(super) fn item_remove_from_folders(c: &Ctx, p: FolderItems) -> ApiResult {
    folder_items_op(c, p, "unadd")
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct MoveToFolder {
    ids: Vec<ItemRef>,
    /// The folder they go to.
    folder: i64,
    /// The folder they leave, with its subfolders; without it they leave every folder they are in.
    #[serde(default)]
    from: Option<i64>,
}

pub(super) fn item_move_to_folder(c: &Ctx, p: MoveToFolder) -> ApiResult {
    let ids = ids_of(c, &p.ids)?;
    folders_exist(c, &[p.folder])?;
    write(c, move |db| edits::folder_op(db, "move_items", FolderArgs { id: Some(p.folder), ids, value: p.from.map(|f| f.to_string()), ..Default::default() }))
}

pub(super) fn item_trash(c: &Ctx, p: Items) -> ApiResult {
    let ids = ids_of(c, &p.ids)?;
    write(c, move |db| edits::trash(db, &ids, true))
}

pub(super) fn item_restore(c: &Ctx, p: Items) -> ApiResult {
    let ids = ids_of(c, &p.ids)?;
    write(c, move |db| edits::trash(db, &ids, false))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct Purge {
    /// Items in the trash.
    #[serde(default)]
    ids: Vec<ItemRef>,
    /// Everything in the trash.
    #[serde(default)]
    all: bool,
}

pub(super) fn item_purge(c: &Ctx, p: Purge) -> ApiResult {
    let ids = if p.all {
        None
    } else if p.ids.is_empty() {
        return Err(bad("Give the `ids` of items in the trash, or `all: true` to empty it"));
    } else {
        let ids = items_of(c, &p.ids)?;
        let kept = read(c.host, |db| Ok(db.assets(&ids)?.into_iter().filter(|a| !a.deleted).count()))?;
        if kept > 0 {
            return Err(said("bad_request", "errors.trash.notInTrash", Value::Null, "Only items in the trash can be deleted for good; move them to the trash first"));
        }
        Some(ids)
    };
    with_library(c.host, |lib| {
        lib.purge(ids);
        Ok(())
    })?;
    Ok(done())
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct Rotate {
    ids: Vec<ItemRef>,
    /// 0, 90, 180 or 270, clockwise.
    degrees: i64,
}

pub(super) fn item_rotate(c: &Ctx, p: Rotate) -> ApiResult {
    if p.degrees.rem_euclid(90) != 0 {
        return Err(bad("Turn by 0, 90, 180 or 270 degrees"));
    }
    let ids = items_of(c, &p.ids)?;
    write(c, move |db| {
        db.set_rotation(&ids, p.degrees.rem_euclid(360))?;
        Ok(Changed::shown(&ids, Aspect::Metadata))
    })
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct RotateFile {
    id: ItemRef,
    /// Quarter turns clockwise, 1 by default.
    #[serde(default)]
    turns: Option<u32>,
}

pub(super) fn writable(c: &Ctx, id: AssetId) -> Result<Asset, ApiError> {
    let a = read_checked(c, |db| Ok(db.asset(id)?.ok_or_else(|| not_found("No such item"))))?;
    read(c.host, |db| db.require_writable(a.root_id))?;
    Ok(a)
}

pub(super) fn item_rotate_file(c: &Ctx, p: RotateFile) -> ApiResult {
    let id = one_id(c, &p.id)?;
    let a = writable(c, id)?;
    let animated = read(c.host, |db| Ok(db.grid_items(&[id])?.first().map(|g| g.animated).unwrap_or(false)))?;
    if !crate::core::images::rotate::can_rotate(&a.ext, animated) {
        return Err(said("bad_request", "errors.files.cantRotate", json!({"ext": a.ext}), format!(".{} files can't be turned; JPG, PNG, BMP and still WebP can", a.ext)));
    }
    let turns = p.turns.unwrap_or(1);
    job_for(c, FILE_WAIT, move |db, ctx| {
        let a = db.asset(id)?.ok_or("not found")?;
        let path = PathBuf::from(&a.abs_path);
        let (w, h) = crate::core::images::rotate::rotate_in_place(&path, &a.ext, turns)?;
        let st = crate::core::system::filesystem::stat(&path).map_err(|e| e.to_string())?;
        db.file_rewritten(id, Some((w as i64, h as i64)), st.size as i64, st.mtime)?;
        // the turned picture is a new file under the old name; the item's other folders get it too
        crate::core::store::relink(db, &ctx.paths, id)?;
        Ok(((), Changed::files(&[id])))
    })?;
    Ok(done())
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct ReplaceFile {
    id: ItemRef,
    /// A file of the same type on this computer.
    #[serde(default)]
    path: Option<String>,
    /// The new file's contents as base64, or a data: address, for callers that have no file on this computer, such as plugin pages.
    #[serde(default)]
    data: Option<String>,
}

pub(super) fn item_replace_file(c: &Ctx, p: ReplaceFile) -> ApiResult {
    let id = one_id(c, &p.id)?;
    let a = writable(c, id)?;
    // bytes are written beside the library's other temporary files, then put in place as a file would be
    let (from, temporary) = match (p.path, p.data) {
        (Some(path), None) => {
            may_name(c, std::slice::from_ref(&path))?;
            let from = PathBuf::from(&path);
            if !from.is_file() {
                return Err(no_file(&path));
            }
            let ext = from.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            if ext != a.ext {
                return Err(said("bad_request", "errors.files.sameType", json!({"ext": a.ext}), format!("Choose a .{} file: the item keeps its type", a.ext)));
            }
            (from, false)
        }
        (None, Some(data)) => {
            let bytes = base64_bytes(&data)?;
            let tmp = with_library(c.host, |lib| Ok(lib.paths().tmp.clone()))?;
            std::fs::create_dir_all(&tmp).map_err(|e| ApiError::new("failed", e.to_string()))?;
            let file = tmp.join(format!("replace-{}.{}", fastrand::u64(..), a.ext));
            std::fs::write(&file, bytes).map_err(|e| ApiError::new("failed", e.to_string()))?;
            (file, true)
        }
        _ => return Err(bad("Give either `path` or `data`")),
    };
    let src = from.clone();
    let out = job_for(c, FILE_WAIT, move |db, ctx| {
        crate::core::store::journal::replace_with(db, id, &src)?;
        crate::core::store::relink(db, &ctx.paths, id)?;
        Ok(((), Changed::files(&[id])))
    });
    if temporary {
        let _ = std::fs::remove_file(&from);
    }
    out?;
    Ok(done())
}

/// Previews come in steps of this many pixels, so a slightly larger request reuses the one made.
const PREVIEW_STEP: u32 = 256;

#[derive(Deserialize, JsonSchema)]
pub(super) struct PreviewParams {
    /// The item's id.
    id: ItemRef,
    /// The long edge wanted, in pixels; rounded up to a step of 256, from 256 to 8192.
    size: u32,
}

pub(super) fn item_preview_url(c: &Ctx, p: PreviewParams) -> ApiResult {
    let id = one_id(c, &p.id)?;
    let size = p.size.clamp(PREVIEW_STEP, 8192).div_ceil(PREVIEW_STEP) * PREVIEW_STEP;
    let (ok, ext, ver) = read_checked(c, |db| Ok(db.asset(id)?.map(|a| (a.status == crate::core::model::Availability::Ok, a.ext, a.thumb_ver)).ok_or_else(|| not_found("No such item"))))?;
    if !ok {
        return Err(file_missing());
    }
    let url = c.host.page_file(id, &ext).ok_or_else(|| ApiError::new("not_allowed", "Only a plugin's page reads previews by address"))?;
    let base = url.rsplit_once('/').map_or(url.as_str(), |(b, _)| b);
    Ok(json!({ "url": format!("{base}/~preview/{size}?v={ver}") }))
}

pub(super) fn item_file_url(c: &Ctx, p: OneItem) -> ApiResult {
    let id = one_id(c, &p.id)?;
    // judged by the item's state rather than the file this instant, which may be in the middle of being replaced
    let (ok, ext) = read_checked(c, |db| Ok(db.asset(id)?.map(|a| (a.status == crate::core::model::Availability::Ok, a.ext)).ok_or_else(|| not_found("No such item"))))?;
    if !ok {
        return Err(file_missing());
    }
    let url = c.host.page_file(id, &ext).ok_or_else(|| ApiError::new("not_allowed", "Only a plugin's page reads files by address"))?;
    Ok(json!({ "url": url }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct SetThumbnail {
    ids: Vec<ItemRef>,
    /// A picture file on this computer.
    #[serde(default)]
    path: Option<String>,
    /// A picture as base64, or a data: address.
    #[serde(default)]
    data: Option<String>,
}

pub(super) fn item_set_thumbnail(c: &Ctx, p: SetThumbnail) -> ApiResult {
    let img = match (&p.path, &p.data) {
        (Some(path), _) => may_name(c, std::slice::from_ref(path)).and_then(|_| image::open(path).map_err(|e| bad(format!("Couldn't read {path} as a picture: {e}"))))?,
        (None, Some(d)) => image::load_from_memory(&base64_bytes(d)?).map_err(|e| bad(format!("`data` isn't a picture: {e}")))?,
        (None, None) => return Err(bad("Give a picture as `path` or `data`")),
    };
    let ids = items_of(c, &p.ids)?;
    let t = thumbs::custom_thumbnail(&img)?;
    let dir = with_library(c.host, |lib| Ok(lib.paths().thumbs.clone()))?;
    write(c, move |db| edits::custom_thumbnail(db, &dir, &ids, &t))
}

pub(super) fn item_refresh_thumbnail(c: &Ctx, p: Items) -> ApiResult {
    let ids = items_of(c, &p.ids)?;
    let r = write(c, move |db| {
        db.invalidate_thumbs(&ids)?;
        Ok(Changed::shown(&ids, Aspect::Thumbnail))
    })?;
    with_library(c.host, |lib| {
        lib.kick_thumbs();
        Ok(())
    })?;
    Ok(r)
}

pub(super) fn item_refresh_palette(c: &Ctx, p: Items) -> ApiResult {
    let ids = items_of(c, &p.ids)?;
    let (dir, ctx) = with_library(c.host, |lib| Ok((lib.paths().thumbs.clone(), lib.ctx.clone())))?;
    let thumbs: Vec<(i64, PathBuf)> = read(c.host, |db| Ok(db.assets(&ids)?.into_iter().map(|a| (a.id, thumbs::thumb_path(&dir, &a.uid, a.thumb_ver))).collect()))?;
    std::thread::spawn(move || {
        // thumbnails carry no extension that names their format, so it is read from the bytes
        let palettes: Vec<(i64, Vec<PaletteColor>)> = thumbs
            .into_iter()
            .filter_map(|(id, p)| {
                let img = image::load_from_memory(&std::fs::read(p).ok()?).ok()?;
                Some((id, crate::core::images::color::palette_from_rgba(img.to_rgba8().as_raw())))
            })
            .collect();
        ctx.write(move |db, _| {
            for (id, palette) in &palettes {
                db.set_thumb_done(*id, true, None, Some(palette), None)?;
            }
            Ok(Changed::shown(&palettes.iter().map(|p| p.0).collect::<Vec<_>>(), Aspect::Palette))
        });
    });
    Ok(json!({"success": true, "queued": true}))
}

pub(super) fn item_duplicate(c: &Ctx, p: Items) -> ApiResult {
    let ids = ids_of(c, &p.ids)?;
    let plan = read(c.host, |db| {
        let fields: HashMap<i64, FieldDef> = db.fields()?.into_iter().map(|f| (f.id, f)).collect();
        let mut out = Vec::new();
        for a in db.assets(&db.items_of(&ids)?)? {
            let folders: Vec<i64> = db.asset_folders(a.id)?.into_iter().map(|f| f.id).collect();
            out.push((PathBuf::from(&a.abs_path), crate::core::export::asset_json(db, &a, &fields), folders, a.id));
        }
        Ok(out)
    })?;
    let (style, lp) = with_library(c.host, |lib| Ok((lib.settings().name_suffix, lib.paths().clone())))?;
    let made = job_for(c, FILE_WAIT, move |db, _| {
        let mut made = Vec::new();
        for (src, meta, folders, from) in &plan {
            // the copy and its row first, then metadata and the other folders; a crash between leaves an item without them, never a lost file
            let (id, _) = crate::core::import::import_one(db, &lp, crate::core::import::Incoming::File(src), &crate::core::system::filesystem::name_of(src), folders.first().copied(), OnClash::Keep, style, None)?;
            db.tx(|db| {
                crate::core::import::apply_incoming(db, id, meta)?;
                for f in folders.iter().skip(1) {
                    db.add_to_folder(&[id], *f)?;
                }
                Ok(())
            })?;
            crate::core::import::copy_members(db, &lp, *from, id, style)?;
            made.push(id);
        }
        let out = uids(db, &made)?;
        Ok((out, Changed::added(&made, "duplicate")))
    })?;
    Ok(json!({"success": true, "ids": made}))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct ToLibrary {
    ids: Vec<ItemRef>,
    /// The library they go to: its id from library.list, its folder or its name.
    to: String,
}

fn send_to_library(c: &Ctx, p: ToLibrary, then_trash: bool) -> ApiResult {
    let target = library_named(c.host, c.caller, &p.to)?;
    if c.host.library().is_some_and(|m| same_path(&m, &target.path)) {
        return Err(said("bad_request", "errors.library.alreadyThere", Value::Null, "The items are already in that library"));
    }
    if !instance::is_library(&target.path) {
        return Err(unreachable_library(&target));
    }
    let ids = items_of(c, &p.ids)?;
    let (files, meta) = read(c.host, |db| {
        let fields: HashMap<i64, FieldDef> = db.fields()?.into_iter().map(|f| (f.id, f)).collect();
        let list = db.assets(&ids)?;
        // the files grouped into them go along, and are grouped again where they arrive
        let files: Vec<PathBuf> = db.with_members(&ids)?.iter().filter_map(|i| db.abs_path(*i)).collect();
        Ok((files, list.iter().map(|a| instance::transfer_key(&a.abs_path)).zip(crate::core::export::assets_json(db, &list, &fields)).collect::<Vec<_>>()))
    })?;
    if files.is_empty() {
        return Err(said("failed", "errors.files.allMissing", Value::Null, "Those files are missing or offline"));
    }
    // a file only carries its path: the receiving library reads the rest from here
    instance::write_transfer(&c.host.data_dir(), meta);
    let n = files.len();
    match instance::holder(&target.path) {
        Some(h) => {
            instance::tell(&h, "add", &json!({"paths": files, "how": how(c)})).map_err(|e| ApiError::new("busy", e))?;
        }
        None => {
            // nobody has it open: hold it just long enough to add the files
            let _lock = match instance::acquire(&target.path, &c.host.me())? {
                instance::Acquired::Mine(l) => l,
                instance::Acquired::Elsewhere(_) => return Err(said("busy", "errors.library.opening", Value::Null, "That library is being opened; try again")),
            };
            let lp = crate::core::library::paths::LibraryPaths::new(&target.path, None);
            lp.ensure().map_err(|e| e.to_string())?;
            let mut db = Db::open(&lp.db)?;
            db.repoint_managed_root(&lp.managed.to_string_lossy())?;
            crate::core::import::managed_root(&db, &lp)?;
            let transfer = instance::read_transfer(&c.host.data_dir());
            let cancel = std::sync::atomic::AtomicBool::new(false);
            crate::core::import::bring_in(&mut db, &lp, &files, false, None, Default::default(), OnClash::Keep, &transfer, &Default::default(), &|_, _| {}, &cancel)?;
        }
    }
    if then_trash {
        write(c, move |db| edits::trash(db, &ids, true))?;
    }
    Ok(json!({"success": true, "count": n}))
}

pub(super) fn item_copy_to_library(c: &Ctx, p: ToLibrary) -> ApiResult {
    send_to_library(c, p, false)
}

pub(super) fn item_move_to_library(c: &Ctx, p: ToLibrary) -> ApiResult {
    send_to_library(c, p, true)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct Export {
    ids: Vec<ItemRef>,
    /// A folder on this computer; made when missing. Files already there are kept, the copies get numbered names.
    folder: String,
    /// The files grouped into each item go too; true unless set.
    #[serde(default)]
    members: Option<bool>,
}

pub(super) fn item_export(c: &Ctx, p: Export) -> ApiResult {
    let ids = items_of(c, &p.ids)?;
    let dir = PathBuf::from(&p.folder);
    std::fs::create_dir_all(&dir).map_err(|e| bad(format!("Couldn't make {}: {e}", dir.display())))?;
    let with_members = p.members.unwrap_or(true);
    let files: Vec<PathBuf> = read(c.host, |db| {
        let mut files: Vec<PathBuf> = ids.iter().filter_map(|i| db.abs_path(*i)).collect();
        // a grouped file that is missing stays behind; a missing item is an error, as before
        if with_members {
            let members: Vec<AssetId> = db.with_members(&ids)?.into_iter().skip(ids.len()).collect();
            files.extend(members.iter().filter_map(|i| db.abs_path(*i)).filter(|p| p.is_file()));
        }
        Ok(files)
    })?;
    for f in &files {
        let dest = dir.join(crate::core::system::filesystem::name_of(f));
        crate::core::system::filesystem::copy_no_clobber(f, &dest).map_err(|e| format!("copy {}: {e}", f.display()))?;
    }
    Ok(json!({"success": true, "count": ids.len()}))
}

pub(super) fn item_thumbnail(c: &Ctx, p: OneItem) -> ApiResult {
    let id = one_id(c, &p.id)?;
    let (uid, ver) = read_checked(c, |db| Ok(db.asset(id)?.map(|a| (a.uid, a.thumb_ver)).ok_or_else(|| not_found("No such item"))))?;
    let dir = with_library(c.host, |lib| Ok(lib.paths().thumbs.clone()))?;
    let path = thumbs::thumb_path(&dir, &uid, ver);
    let mut head = [0u8; 12];
    let n = std::fs::File::open(&path).and_then(|mut f| std::io::Read::read(&mut f, &mut head)).map_err(|_| not_found("This item has no thumbnail yet"))?;
    Ok(file_reply(&path, image_type(&head[..n])))
}

pub(super) fn item_file(c: &Ctx, p: OneItem) -> ApiResult {
    let id = one_id(c, &p.id)?;
    let (path, ext) = read_checked(c, |db| Ok(db.asset(id)?.map(|a| (db.abs_path(id), a.ext)).ok_or_else(|| not_found("No such item"))))?;
    match path.filter(|p| p.is_file()) {
        Some(p) => Ok(file_reply(&p, mime_for_ext(&ext))),
        None => Err(file_missing()),
    }
}
