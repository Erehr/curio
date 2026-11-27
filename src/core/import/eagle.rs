//! Importing an Eagle `.library` folder: its items, folders, smart folders and colours.

use crate::core::import::import_one;
use crate::core::import::Incoming;
use crate::core::import::ImportStep;
use crate::core::library::paths::LibraryPaths;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use crate::core::db::{Db, DbResult, assets::ScannedFile};
use crate::core::system::filesystem;
use crate::core::model::*;
use crate::core::said::Said;
use crate::core::store::{OnClash, SuffixStyle};

#[derive(Debug, Default)]
pub struct DotLibraryStats {
    pub items: usize,
    pub folders: usize,
    pub smart_folders: usize,
    /// Smart folders whose rules have no counterpart in the query language.
    pub smart_skipped: usize,
    pub skipped: usize,
    /// Items imported from this library before, left as they are.
    pub already: usize,
    /// Whether the files came in as hard links to the library's own rather than as copies.
    pub linked: bool,
    /// The location its `images` folder became, when linked in place.
    pub location: Option<i64>,
}

/// A `.library` folder's own `metadata.json`.
fn dot_library_meta(lib: &Path) -> DbResult<serde_json::Value> {
    std::fs::read_to_string(lib.join("metadata.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .ok_or_else(|| format!("{} isn't an Eagle library: it has no metadata.json", lib.display()))
}

/// What earlier imports made for a `.library`'s folders, smart folders and tag groups, by their ids there, so importing it again fills those instead of making more. Folders and smart folders are kept by uid, as a row id can be taken again by a folder made later; tag groups, which have none, by id and name.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct ImportedStructure {
    folders: HashMap<String, String>,
    smart: HashMap<String, String>,
    groups: HashMap<String, (i64, String)>,
    /// Eagle's auto-tags for the folders this import made, set once its items are in: Eagle tags only what arrives after them, so the items it already had keep their own tags.
    #[serde(skip)]
    auto_tags: Vec<(i64, Vec<String>)>,
}

const IMPORTED_STRUCTURE: &str = "imported_structure";

/// Eagle's named colours as the swatches Curio offers; a colour already in hex stays.
fn eagle_color(c: &str) -> Option<String> {
    let hex = match c {
        "red" => "#e5686b",
        "orange" => "#ef9a3d",
        "yellow" => "#e9c944",
        "green" => "#4fc57c",
        "aqua" => "#38bdc9",
        "blue" => "#4a90e2",
        "purple" => "#a98bea",
        "pink" => "#e784b4",
        c if c.starts_with('#') => c,
        _ => return None,
    };
    Some(hex.to_string())
}

/// The `.library` folder tree (inside `under`, or at the top) and tag groups as library folders and tag groups, with its starred tags and pinned folders. A folder never joins one of the same name: each is a directory, so a sibling's name gets a number. Returns its folder id → library folder.
fn dot_library_structure(db: &Db, lib: &Path, meta: &serde_json::Value, under: Option<i64>, made: &mut ImportedStructure, stats: &mut DotLibraryStats) -> DbResult<HashMap<String, i64>> {
    let pinned: HashSet<&str> = meta["quickAccess"].as_array().into_iter().flatten().filter(|q| q["type"] == "folder").filter_map(|q| q["id"].as_str()).collect();
    let mut stack: Vec<(&serde_json::Value, Option<i64>)> = meta["folders"].as_array().into_iter().flatten().rev().map(|f| (f, under)).collect();
    let mut map = HashMap::new();
    while let Some((f, parent)) = stack.pop() {
        let eid = f["id"].as_str();
        let id = match ImportedStructure::made(db, &made.folders, eid, "folders") {
            Some(id) => id,
            None => {
                let id = db.create_folder(f["name"].as_str().unwrap_or("Folder"), parent)?;
                if let Some(d) = f["description"].as_str().filter(|d| !d.is_empty()) {
                    db.conn.execute("UPDATE folders SET description = ? WHERE id = ?", rusqlite::params![d, id]).map_err(|e| e.to_string())?;
                }
                if let Some(c) = f["iconColor"].as_str().and_then(eagle_color) {
                    db.set_folder_color(id, Some(&c))?;
                }
                let auto: Vec<String> = f["tags"].as_array().into_iter().flatten().filter_map(|t| t.as_str().map(String::from)).collect();
                if !auto.is_empty() {
                    made.auto_tags.push((id, auto));
                }
                if eid.is_some_and(|e| pinned.contains(e)) {
                    let mut favs = db.favorites();
                    if !favs.iter().any(|v| v["kind"].as_str() == Some("folder") && v["id"].as_i64() == Some(id)) {
                        favs.push(serde_json::json!({ "kind": "folder", "id": id }));
                        db.set_favorites(&favs)?;
                    }
                }
                if let Some(e) = eid {
                    made.folders.insert(e.to_string(), ImportedStructure::uid_of(db, "folders", id)?);
                }
                stats.folders += 1;
                id
            }
        };
        if let Some(e) = eid {
            map.insert(e.to_string(), id);
        }
        stack.extend(f["children"].as_array().into_iter().flatten().rev().map(|c| (c, Some(id))));
    }
    for g in meta["tagsGroups"].as_array().into_iter().flatten() {
        let name = g["name"].as_str().unwrap_or("Group");
        let before = g["id"].as_str().and_then(|e| made.groups.get(e)).filter(|(id, n)| db.conn.query_row("SELECT name FROM tag_groups WHERE id = ?", [id], |r| r.get::<_, String>(0)).is_ok_and(|now| &now == n));
        let gid = match before {
            Some((id, _)) => *id,
            None => {
                let id = db.create_tag_group(name, g["color"].as_str().and_then(eagle_color).as_deref())?;
                if let Some(e) = g["id"].as_str() {
                    made.groups.insert(e.to_string(), (id, name.to_string()));
                }
                id
            }
        };
        for t in g["tags"].as_array().into_iter().flatten().filter_map(|t| t.as_str()) {
            let tid = db.get_or_create_tag(t)?;
            db.set_tag_group(tid, Some(gid))?;
        }
    }
    // its tag list holds tags no item has yet, and the ones starred there
    let tags: serde_json::Value = std::fs::read_to_string(lib.join("tags.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    for t in tags["historyTags"].as_array().into_iter().flatten().filter_map(|t| t.as_str()) {
        db.get_or_create_tag(t)?;
    }
    for t in tags["starredTags"].as_array().into_iter().flatten().filter_map(|t| t.as_str()) {
        let tid = db.get_or_create_tag(t)?;
        db.set_tag_starred(tid, true)?;
    }
    Ok(map)
}

fn dot_library_smart_folders(db: &Db, meta: &serde_json::Value, folder_map: &HashMap<String, i64>, made: &mut ImportedStructure, stats: &mut DotLibraryStats) -> DbResult<()> {
    let folder_name = |e: &str| folder_map.get(e).and_then(|id| db.conn.query_row("SELECT name FROM folders WHERE id = ?", [id], |r| r.get::<_, String>(0)).ok());
    let mut stack: Vec<(&serde_json::Value, Option<i64>)> = meta["smartFolders"].as_array().into_iter().flatten().rev().map(|f| (f, None)).collect();
    while let Some((sf, parent)) = stack.pop() {
        let eid = sf["id"].as_str();
        let id = match ImportedStructure::made(db, &made.smart, eid, "smart_folders") {
            Some(id) => Some(id),
            None => match crate::core::query::saved_rules::from_saved_rules(&sf["conditions"], &folder_name) {
                Ok(q) => {
                    let id = db.create_smart_folder_in(sf["name"].as_str().unwrap_or("Smart folder"), &q, parent, None)?;
                    if let Some(e) = eid {
                        made.smart.insert(e.to_string(), ImportedStructure::uid_of(db, "smart_folders", id)?);
                    }
                    stats.smart_folders += 1;
                    Some(id)
                }
                Err(_) => {
                    stats.smart_skipped += 1;
                    None
                }
            },
        };
        // the smart folders inside one that can't come along go to its parent
        stack.extend(sf["children"].as_array().into_iter().flatten().rev().map(|c| (c, id.or(parent))));
    }
    Ok(())
}

/// What an Eagle library import did, for the notice at its end.
pub fn eagle_report(s: &DotLibraryStats, copy: bool) -> Said {
    let n = |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let mut out = if copy {
        Said::new("results.eagle.imported", json!({"count": s.items}), format!("Imported {} from the Eagle library.", n(s.items, "item", "items")))
    } else {
        Said::new("results.eagle.linked", json!({"count": s.items}), format!("Linked the Eagle library in place: {}.", n(s.items, "item", "items")))
    };
    // what a second import of the same library didn't make again goes unsaid
    if s.folders > 0 || s.smart_folders > 0 {
        out = out.and("results.eagle.folders", json!({"folders": s.folders, "smart": s.smart_folders}), format!("Folders: {}, smart folders: {}.", s.folders, s.smart_folders));
    }
    if copy && s.linked {
        out = out.and("results.eagle.hardLinked", Value::Null, "Its files are hard-linked, so they take no extra space.");
    } else if copy && s.items > 0 {
        out = out.and("results.eagle.copied", Value::Null, "Its files were copied: hard links can't reach its drive.");
    }
    if s.already > 0 {
        out = out.and("results.eagle.already", json!({"count": s.already}), format!("{} already here.", n(s.already, "item was", "items were")));
    }
    if s.skipped > 0 {
        out = out.and("results.eagle.unreadable", json!({"count": s.skipped}), format!("{} couldn't be read.", n(s.skipped, "item", "items")));
    }
    if s.smart_skipped > 0 {
        out = out.and("results.eagle.smartSkipped", json!({"count": s.smart_skipped}), format!("{} rules Curio doesn't have.", n(s.smart_skipped, "smart folder uses", "smart folders use")));
    }
    out
}

/// Whether `dir` is an Eagle library: a `.library` folder with its `metadata.json` and `images`.
pub fn is_eagle_library(dir: &Path) -> bool {
    is_eagle_images(&dir.join("images")) && dir.join("images").is_dir()
}

/// Whether a location's path is an Eagle library's `images` folder, by its name alone: a folder inside a `.library` can't be added as a location otherwise.
pub fn eagle_images_path(path: &str) -> bool {
    let p = path.to_lowercase();
    p.ends_with(".library/images") || p.ends_with(".library\\images")
}

/// Whether `dir` is the `images` folder of an Eagle library, which holds a folder per item.
pub fn is_eagle_images(dir: &Path) -> bool {
    dir.file_name().is_some_and(|n| n == "images") && dir.parent().is_some_and(|l| l.extension().is_some_and(|e| e.eq_ignore_ascii_case("library")) && l.join("metadata.json").is_file())
}

/// The `.info` folders of a `.library`, one per item.
fn dot_library_infos(lib: &Path) -> DbResult<Vec<PathBuf>> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(lib.join("images"))
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "info").unwrap_or(false))
        .collect();
    v.sort();
    Ok(v)
}

/// A `.library` item's metadata and the file it describes (`<name>.<ext>` beside it), or why it can't be read.
fn dot_library_item(info: &Path) -> Option<(serde_json::Value, String, PathBuf)> {
    let item: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(info.join("metadata.json")).ok()?).ok()?;
    let name = item["name"].as_str().unwrap_or("");
    let ext = item["ext"].as_str().unwrap_or("").to_lowercase();
    let file_name = if ext.is_empty() { name.to_string() } else { format!("{name}.{ext}") };
    // the file lies beside metadata.json, so anything but a plain name points somewhere else
    let mut parts = Path::new(&file_name).components();
    if file_name.contains(['/', '\\']) || !matches!((parts.next(), parts.next()), (Some(std::path::Component::Normal(_)), None)) {
        return None;
    }
    let file = info.join(&file_name);
    file.is_file().then_some((item, file_name, file))
}

/// The field that keeps an item's id from its `.library`, so importing it again leaves its items alone.
fn imported_id_field(db: &Db) -> DbResult<FieldDef> {
    let mut f = FieldDef::new("imported_id", "Imported ID", crate::core::model::FieldType::Text, "import");
    f.id = db.define_field(&f)?;
    Ok(f)
}

fn already_imported(db: &Db, field: &FieldDef, imported_id: &str) -> bool {
    db.conn.query_row("SELECT 1 FROM field_values WHERE field_id = ? AND text_value = ?", rusqlite::params![field.id, imported_id], |_| Ok(())).is_ok()
}

/// Everything the `.library` knew about an item, onto `id`: tags, folders, rating, note, link, palette, dates, trash, and its id there.
fn dot_library_metadata(db: &Db, id: AssetId, item: &serde_json::Value, folders: &HashMap<String, i64>, field: &FieldDef) -> DbResult<()> {
    if let Some(t) = item["modificationTime"].as_i64() {
        db.conn.execute("UPDATE assets SET added_at = ? WHERE id = ?", rusqlite::params![t, id]).map_err(|e| e.to_string())?;
    }
    // Eagle keeps the date the file was first made; its copy in the library, or a link to it, is younger
    if let Some(t) = item["btime"].as_i64().filter(|t| *t > 0) {
        db.conn.execute("UPDATE assets SET ctime = ? WHERE id = ?", rusqlite::params![t, id]).map_err(|e| e.to_string())?;
    }
    if let Some(tags) = item["tags"].as_array() {
        let names: Vec<String> = tags.iter().filter_map(|t| t.as_str().map(String::from)).collect();
        if !names.is_empty() {
            db.add_tags(&[id], &names)?;
        }
    }
    for fid in item["folders"].as_array().into_iter().flatten().filter_map(|f| f.as_str()) {
        if let Some(local) = folders.get(fid) {
            db.add_to_folder(&[id], *local)?;
        }
    }
    if let Some(s) = item["star"].as_i64() {
        db.set_rating(&[id], s)?;
    }
    if let Some(a) = item["annotation"].as_str().filter(|a| !a.is_empty()) {
        db.set_annotation(&[id], a)?;
    }
    if let Some(u) = item["url"].as_str().filter(|u| !u.is_empty()) {
        db.set_source_url(&[id], u)?;
    }
    if let Some(p) = item["palettes"].as_array() {
        let pal: Vec<PaletteColor> = p.iter().filter_map(|c| serde_json::from_value(c.clone()).ok()).collect();
        if !pal.is_empty() {
            db.conn.execute("UPDATE assets SET palette = ? WHERE id = ?", rusqlite::params![crate::core::images::color::palette_to_json(&pal), id]).map_err(|e| e.to_string())?;
        }
    }
    if let Some(eid) = item["id"].as_str() {
        db.set_field(&[id], field, &RawValue::Text(eid.to_string()))?;
    }
    if item["isDeleted"].as_bool() == Some(true) {
        db.set_deleted(&[id], true)?;
    }
    Ok(())
}

/// Index a `.library` folder in place (its files are not moved or copied) and bring over
/// folders, tags, ratings, notes, URLs, palettes and smart folders.
pub fn import_dot_library(db: &mut Db, lib: &Path, under: Option<i64>) -> DbResult<DotLibraryStats> {
    let mut stats = DotLibraryStats::default();
    let meta = dot_library_meta(lib)?;
    let images = lib.join("images");
    let root_id = db.add_root(&images.to_string_lossy(), RootKind::Linked, true)?;
    db.conn.execute("UPDATE roots SET label = ? WHERE id = ?", rusqlite::params![format!("Eagle: {}", filesystem::stem_of(lib)), root_id]).map_err(|e| e.to_string())?;
    stats.location = Some(root_id);
    let mut made = ImportedStructure::load(db);
    let folder_map = dot_library_structure(db, lib, &meta, under, &mut made, &mut stats)?;
    made.save(db)?;
    let field = imported_id_field(db)?;
    for chunk in dot_library_infos(lib)?.chunks(crate::core::consts::SCAN_BATCH) {
        // read and hash the batch first, so the transaction only writes
        let mut ready = Vec::with_capacity(chunk.len());
        for info in chunk {
            let Some((item, file_name, file)) = dot_library_item(info) else {
                stats.skipped += 1;
                continue;
            };
            let Ok(st) = filesystem::stat(&file) else {
                stats.skipped += 1;
                continue;
            };
            let rel = filesystem::rel_to_db(file.strip_prefix(&images).unwrap_or(&file));
            if db.asset_at(root_id, &rel).is_some() {
                stats.already += 1;
                continue;
            }
            let ext = filesystem::ext_of(&file);
            let animated = matches!(ext.as_str(), "gif" | "webp" | "png" | "apng").then(|| crate::core::metadata::extract::is_animated(&file, &ext));
            let f = ScannedFile {
                rel_path: rel,
                name: file_name,
                ext,
                size: st.size as i64,
                // what a scan compares, so the file's own
                mtime: st.mtime,
                ctime: item["btime"].as_i64().or(st.ctime),
                file_key: st.file_key.clone(),
                quick_hash: filesystem::quick_hash(&file, st.size).ok(),
                width: item["width"].as_i64().filter(|w| *w > 0),
                height: item["height"].as_i64().filter(|h| *h > 0),
                duration: item["duration"].as_f64(),
                animated,
            };
            ready.push((item, f));
        }
        db.tx(|db| {
            for (item, f) in &ready {
                let id = db.insert_scanned(root_id, f)?;
                dot_library_metadata(db, id, item, &folder_map, &field)?;
                stats.items += 1;
            }
            Ok(())
        })?;
    }
    dot_library_smart_folders(db, &meta, &folder_map, &mut made, &mut stats)?;
    made.set_auto_tags(db)?;
    made.save(db)?;
    Ok(stats)
}

/// Whether a file at `src` can be hard-linked into the store at `managed`: both on one drive that has links.
fn links_reach(src: &Path, managed: &Path) -> bool {
    let probe = managed.join(format!(".curio-link-probe-{}", fastrand::u32(..)));
    let ok = std::fs::create_dir_all(managed).is_ok() && std::fs::hard_link(src, &probe).is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// Bring a `.library` into this library's store: each item's file goes into the directory of its first folder there under its name there (the others get links), with the same folders, tags, ratings, notes, links, palettes and smart folders; items in its trash land in the trash. With `link`, files on the store's drive come in as hard links to the `.library`'s own, so they take no more space and stay when it is deleted; the rest are copied. Items imported from it before are left alone. The `.library` itself is only read.
#[allow(clippy::too_many_arguments)]
pub fn import_dot_library_copy(db: &mut Db, paths: &LibraryPaths, lib: &Path, under: Option<i64>, style: SuffixStyle, link: bool, step: &dyn Fn(ImportStep), cancel: &std::sync::atomic::AtomicBool) -> DbResult<(DotLibraryStats, Vec<AssetId>)> {
    let mut stats = DotLibraryStats::default();
    let meta = dot_library_meta(lib)?;
    let mut made = ImportedStructure::load(db);
    let folder_map = dot_library_structure(db, lib, &meta, under, &mut made, &mut stats)?;
    made.save(db)?;
    let field = imported_id_field(db)?;
    let infos = dot_library_infos(lib)?;
    // decided once, so a library on another drive doesn't try and fail for every file
    let mut linking: Option<bool> = (!link).then_some(false);
    let total = infos.len();
    let mut ids = vec![];
    for (n, info) in infos.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        let Some((item, file_name, file)) = dot_library_item(info) else {
            stats.skipped += 1;
            continue;
        };
        if item["id"].as_str().is_some_and(|eid| already_imported(db, &field, eid)) {
            stats.already += 1;
            continue;
        }
        let first = item["folders"].as_array().into_iter().flatten().filter_map(|f| f.as_str()).find_map(|f| folder_map.get(f).copied());
        let linked = *linking.get_or_insert_with(|| links_reach(&file, &paths.managed));
        let incoming = if linked { Incoming::Linked(&file) } else { Incoming::File(&file) };
        let (id, _) = match import_one(db, paths, incoming, &file_name, first, OnClash::Keep, style, None) {
            Ok(r) => r,
            Err(e) => {
                crate::core::system::logging::warn(&format!("{}: {e}", file.display()));
                stats.skipped += 1;
                continue;
            }
        };
        db.tx(|db| {
            if let (Some(w), Some(h)) = (item["width"].as_i64().filter(|w| *w > 0), item["height"].as_i64().filter(|h| *h > 0)) {
                db.conn.execute("UPDATE assets SET width = COALESCE(width, ?), height = COALESCE(height, ?) WHERE id = ?", rusqlite::params![w, h, id]).map_err(|e| e.to_string())?;
            }
            dot_library_metadata(db, id, &item, &folder_map, &field)
        })?;
        stats.items += 1;
        ids.push(id);
        step(ImportStep { done: n + 1, total, name: &file_name, new: std::slice::from_ref(&id), location: None });
    }
    stats.linked = linking == Some(true);
    dot_library_smart_folders(db, &meta, &folder_map, &mut made, &mut stats)?;
    made.set_auto_tags(db)?;
    made.save(db)?;
    Ok((stats, ids))
}

impl ImportedStructure {
    fn load(db: &Db) -> Self {
        db.meta_get(IMPORTED_STRUCTURE).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }
    fn save(&self, db: &Db) -> DbResult<()> {
        db.meta_set(IMPORTED_STRUCTURE, &serde_json::to_string(self).map_err(|e| e.to_string())?)
    }
    /// The folder or smart folder made for `id` before, while it still exists.
    fn made(db: &Db, map: &HashMap<String, String>, id: Option<&str>, table: &str) -> Option<i64> {
        let uid = map.get(id?)?;
        db.conn.query_row(&format!("SELECT id FROM {table} WHERE uid = uid_bytes(?)"), [uid], |r| r.get(0)).ok()
    }
    fn set_auto_tags(&mut self, db: &Db) -> DbResult<()> {
        for (id, tags) in self.auto_tags.drain(..) {
            db.conn.execute("UPDATE folders SET auto_tags = ? WHERE id = ?", rusqlite::params![serde_json::to_string(&tags).map_err(|e| e.to_string())?, id]).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn uid_of(db: &Db, table: &str, row: i64) -> DbResult<String> {
        db.conn.query_row(&format!("SELECT uid_text(uid) FROM {table} WHERE id = ?"), [row], |r| r.get(0)).map_err(|e| e.to_string())
    }
}
