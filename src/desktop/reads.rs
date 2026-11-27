//! What the window reads: the search and the grid, facets, the sidebar, an item's detail, folder covers and the selection.

use crate::desktop::AppState;
use crate::desktop::R;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use crate::core::api;
use crate::core::library::changes::Changed;
use crate::core::model::*;
use crate::core::plugins;

#[derive(Deserialize)]
pub(super) struct SearchReq {
    scope: String,
    #[serde(default)]
    scope_id: i64,
    text: String,
    sort_key: String,
    #[serde(default)]
    sort_field: String,
    descending: bool,
    #[serde(default)]
    subfolders: bool,
    /// What the person typed, for recent searches; `text` may be longer once scoped.
    #[serde(default)]
    history: String,
    /// Several selected folders or locations (`scope` = "folders" or "roots").
    #[serde(default)]
    scope_ids: Vec<i64>,
    /// A folder inside a location (`scope` = "rootdir"), relative, with `/` between levels.
    #[serde(default)]
    scope_path: String,
    /// The cards show notes: the answer ends with a byte for each match, 1 when it has one.
    #[serde(default)]
    notes: bool,
    #[serde(default)]
    group_by: String,
    #[serde(default)]
    group_field: String,
    #[serde(default)]
    group_desc: bool,
    /// Local calendar boundary timestamps (epoch ms): [tomorrow, today, yesterday, thisWeek, lastWeek, thisMonth, lastMonth, thisYear].
    #[serde(default)]
    date_bounds: Vec<i64>,
    #[serde(default)]
    tz_offset_ms: i64,
}

fn parse_scope(s: &str, id: i64, ids: &[i64], path: &str) -> Scope {
    match s {
        "rootdir" => Scope::RootDir(id, path.to_string()),
        "folders" => Scope::Folders(ids.to_vec()),
        "roots" => Scope::Roots(ids.to_vec()),
        "favorites" => Scope::Favorites,
        "uncategorized" => Scope::Uncategorized,
        "untagged" => Scope::Untagged,
        "recent" => Scope::Recent,
        "random" => Scope::Random,
        "trash" => Scope::Trash,
        "missing" => Scope::Missing,
        "folder" => Scope::Folder(id),
        "smart" => Scope::SmartFolder(id),
        "smartBucket" => Scope::SmartBucket(id, path.to_string()),
        "tag" => Scope::Tag(id),
        "root" => Scope::Root(id),
        _ => Scope::All,
    }
}

fn parse_sort(key: &str, field: &str, descending: bool) -> Sort {
    let key = match key {
        "manual" => SortKey::Manual,
        "modified" => SortKey::Modified,
        "created" => SortKey::Created,
        "name" => SortKey::Name,
        "ext" => SortKey::Ext,
        "size" => SortKey::Size,
        "resolution" => SortKey::Resolution,
        "rating" => SortKey::Rating,
        "duration" => SortKey::Duration,
        "random" => SortKey::Random,
        "field" => SortKey::Field(field.to_string()),
        _ => SortKey::Imported,
    };
    Sort { key, descending }
}

fn parse_group(key: &str, field: &str, descending: bool, bounds: &[i64], tz_offset_ms: i64) -> GroupBy {
    let key = match key {
        "kind" => GroupKey::Kind,
        "ext" => GroupKey::Ext,
        "imported" => GroupKey::Imported,
        "modified" => GroupKey::Modified,
        "created" => GroupKey::Created,
        "rating" => GroupKey::Rating,
        "size" => GroupKey::Size,
        "name" => GroupKey::Name,
        "location" => GroupKey::Location,
        "field" if !field.is_empty() => GroupKey::Field(field.to_string()),
        _ => GroupKey::None,
    };
    let dates = if bounds.len() >= 8 {
        DateBounds { tomorrow: bounds[0], today: bounds[1], yesterday: bounds[2], this_week: bounds[3], last_week: bounds[4], this_month: bounds[5], last_month: bounds[6], this_year: bounds[7], tz_offset_ms }
    } else {
        let day = 86_400_000i64;
        let today = (now_ms() / day) * day;
        DateBounds { tomorrow: today + day, today, yesterday: today - day, this_week: today - 7 * day, last_week: today - 14 * day, this_month: today - 30 * day, last_month: today - 60 * day, this_year: today - 365 * day, tz_offset_ms: 0 }
    };
    GroupBy { key, descending, dates }
}

/// Binary layout: 8-byte header (u32 count, u32 groups_json_len), then count * (u32 id, f32 aspect) LE pairs, optional count note bytes when `req.notes`, and trailing UTF-8 JSON for `groups`.
#[tauri::command]
pub(super) async fn search(state: tauri::State<'_, AppState>, req: SearchReq) -> R<tauri::ipc::Response> {
    let scope = parse_scope(&req.scope, req.scope_id, &req.scope_ids, &req.scope_path);
    let sort = parse_sort(&req.sort_key, &req.sort_field, req.descending);
    let group = parse_group(&req.group_by, &req.group_field, req.group_desc, &req.date_bounds, req.tz_offset_ms);
    let (rows, groups, noted) = state.read(|db| {
        let cat = db.field_catalog();
        let (rows, groups) = if group.key == GroupKey::None {
            (db.search(&scope, &req.text, &sort, req.subfolders, &cat)?, vec![])
        } else {
            db.search_grouped(&scope, &req.text, &sort, &group, req.subfolders, &cat)?
        };
        let noted = if req.notes {
            let with = db.noted_ids()?;
            Some(rows.iter().map(|(id, _)| with.contains(id) as u8).collect::<Vec<u8>>())
        } else {
            None
        };
        Ok((rows, groups, noted))
    })?;
    if !req.history.trim().is_empty() {
        let text = req.history.clone();
        state.write(move |db| db.record_search(&text).map(|_| Changed::default()))?;
    }
    let meta = if group.key == GroupKey::None { Vec::new() } else { serde_json::to_vec(&groups).unwrap_or_default() };
    let mut buf = Vec::with_capacity(8 + rows.len() * (if noted.is_some() { 9 } else { 8 }) + meta.len());
    buf.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    buf.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    for (id, a) in rows {
        buf.extend_from_slice(&(id as u32).to_le_bytes());
        buf.extend_from_slice(&a.to_le_bytes());
    }
    if let Some(flags) = noted {
        buf.extend_from_slice(&flags);
    }
    if !meta.is_empty() {
        buf.extend_from_slice(&meta);
    }
    Ok(tauri::ipc::Response::new(buf))
}

/// Where each match sits in the Spectrum view, in the search's order, for those with a palette: packed little-endian u32 id, u16 hue in tenths of a degree, u8 chroma, u8 lightness (0 to 255), and the picture's main colour as three bytes and one spare.
#[tauri::command]
pub(super) async fn spectrum_colors(state: tauri::State<'_, AppState>, req: SearchReq) -> R<tauri::ipc::Response> {
    let scope = parse_scope(&req.scope, req.scope_id, &req.scope_ids, &req.scope_path);
    let buf = state.read(|db| {
        let cat = db.field_catalog();
        let ids: Vec<i64> = db.search(&scope, &req.text, &Sort::default(), req.subfolders, &cat)?.into_iter().map(|(id, _)| id).collect();
        let palettes = db.palettes_of(&ids)?;
        let mut buf = Vec::with_capacity(palettes.len() * 12);
        for id in ids {
            let Some(p) = palettes.get(&id) else { continue };
            let Some((h, c, l)) = crate::core::images::color::spectrum_point(p) else { continue };
            buf.extend_from_slice(&(id as u32).to_le_bytes());
            buf.extend_from_slice(&((h * 10.0).round() as u16 % 3600).to_le_bytes());
            buf.push(c.round().clamp(0.0, 255.0) as u8);
            buf.push((l * 2.55).round().clamp(0.0, 255.0) as u8);
            buf.extend_from_slice(&p[0].color);
            buf.push(0);
        }
        Ok(buf)
    })?;
    Ok(tauri::ipc::Response::new(buf))
}

/// Counts for the Rating and Shape filter menus, within the current scope and other filters.
#[tauri::command]
pub(super) async fn filter_facets(state: tauri::State<'_, AppState>, req: SearchReq) -> R<serde_json::Value> {
    let scope = parse_scope(&req.scope, req.scope_id, &req.scope_ids, &req.scope_path);
    state.read(|db| {
        let cat = db.field_catalog();
        let rows = db.search(&scope, &req.text, &Sort::default(), req.subfolders, &cat)?;
        let ids: Vec<i64> = rows.into_iter().map(|(id, _)| id).collect();
        db.filter_facets(&ids)
    })
}

/// One field's values and their counts for its filter menu, within the current scope and the other filters.
#[tauri::command]
pub(super) async fn field_facets(state: tauri::State<'_, AppState>, req: SearchReq, key: String) -> R<serde_json::Value> {
    let scope = parse_scope(&req.scope, req.scope_id, &req.scope_ids, &req.scope_path);
    state.read(|db| {
        let cat = db.field_catalog();
        let rows = db.search(&scope, &req.text, &Sort::default(), req.subfolders, &cat)?;
        let ids: Vec<i64> = rows.into_iter().map(|(id, _)| id).collect();
        db.field_facets(&ids, &key)
    })
}

/// Virtual subfolders of a smart folder grouped by a property or field.
#[tauri::command]
pub(super) async fn smart_buckets(state: tauri::State<'_, AppState>, id: i64) -> R<Vec<SmartBucket>> {
    state.read(|db| db.smart_buckets(id))
}

/// The folders inside a location that hold items, with how many each holds (subfolders included).
#[tauri::command]
pub(super) async fn root_dirs(state: tauri::State<'_, AppState>, id: i64) -> R<Vec<(String, i64)>> {
    // an Eagle library's folders are one per item, nothing to browse by
    state.read(|db| if db.root(id).is_some_and(|r| crate::core::import::eagle::eagle_images_path(&r.path)) { Ok(vec![]) } else { db.root_dirs(id) })
}

#[tauri::command]
pub(super) async fn grid_items(state: tauri::State<'_, AppState>, ids: Vec<i64>) -> R<Vec<GridItem>> {
    state.read(|db| db.grid_items(&ids))
}

/// The grid's rows of items named by the API's ids, for the tool windows, which work with those; ids of no item are left out.
#[tauri::command]
pub(super) async fn grid_items_by_uid(state: tauri::State<'_, AppState>, uids: Vec<String>) -> R<Vec<GridItem>> {
    state.read(|db| {
        let ids: Vec<i64> = uids.iter().filter_map(|u| db.asset_by_uid(u)).collect();
        db.grid_items(&ids)
    })
}

/// Which dropped files are already items of this library (dragged out of one of its windows), and which aren't.
#[tauri::command]
pub(super) async fn items_at_paths(state: tauri::State<'_, AppState>, paths: Vec<PathBuf>) -> R<serde_json::Value> {
    state.read(|db| {
        let (mut ids, mut rest) = (vec![], vec![]);
        for p in paths {
            match db.asset_at_path(&p) {
                Some(id) => ids.push(id),
                None => rest.push(p),
            }
        }
        Ok(serde_json::json!({ "ids": ids, "rest": rest }))
    })
}

#[tauri::command]
pub(super) async fn asset_detail(state: tauri::State<'_, AppState>, id: i64) -> R<serde_json::Value> {
    state.read(|db| {
        let a = db.asset(id)?.ok_or("not found")?;
        Ok(serde_json::json!({
            "asset": a,
            "tags": db.asset_tags(id)?,
            "folders": db.asset_folders(id)?,
            "values": db.field_values(id)?.into_iter().map(|v| serde_json::json!({"field_id": v.field_id, "idx": v.idx, "text": v.text, "num": v.num})).collect::<Vec<_>>(),
            "overridden": db.overridden_ids(id),
            "parts": db.members_of(id)?,
        }))
    })
}

/// Tag names, folder names and the note of many items at once.
#[tauri::command]
pub(super) async fn assets_brief(state: tauri::State<'_, AppState>, ids: Vec<i64>) -> R<serde_json::Value> {
    state.read(|db| {
        let (tags, folders) = crate::core::export::tag_and_folder_names(db, &ids);
        let mut out = serde_json::Map::new();
        for a in db.assets(&ids)? {
            out.insert(a.id.to_string(), serde_json::json!({ "tags": tags.get(&a.id).cloned().unwrap_or_default(), "folders": folders.get(&a.id).cloned().unwrap_or_default(), "annotation": a.annotation }));
        }
        Ok(serde_json::Value::Object(out))
    })
}

#[tauri::command]
pub(super) async fn sidebar(state: tauri::State<'_, AppState>) -> R<serde_json::Value> {
    state.read(|db| {
        let readers = db.known_readers();
        // each field with the readers that fill it, none for one filled by hand
        let fields: Vec<serde_json::Value> = db.fields()?.into_iter().map(|f| {
            let mut j = serde_json::to_value(&f).unwrap_or_default();
            let fill: Vec<_> = readers.iter().filter(|r| r.fills(&f.key)).collect();
            j["readers"] = serde_json::json!(fill.iter().map(|r| r.id.as_str()).collect::<Vec<_>>());
            // the extensions of the files it applies to; null when it applies to every file
            let every = fill.is_empty() || fill.iter().any(|r| r.exts.is_empty());
            j["exts"] = if every { serde_json::Value::Null } else { serde_json::json!(fill.iter().flat_map(|r| r.exts.iter()).collect::<std::collections::BTreeSet<_>>()) };
            j
        }).collect();
        Ok(serde_json::json!({
            "counts": db.counts()?,
            "folders": db.folders()?,
            "tags": db.tags()?,
            "tagGroups": db.tag_groups()?,
            "smart": db.smart_folders_counted()?,
            "roots": db.roots()?.into_iter().map(|mut r| {
                r.hint = crate::core::system::filesystem::volume_hint(Path::new(&r.path));
                r
            }).collect::<Vec<_>>(),
            "fields": fields,
            "history": db.search_history(),
            "favorites": db.favorites(),
        }))
    })
}

#[tauri::command]
pub(super) fn status(state: tauri::State<AppState>) -> serde_json::Value {
    let g = state.lib.lock();
    match g.as_ref() {
        Some(l) => serde_json::json!({"open": true, "busy": l.busy(), "thumbs": l.thumbs_pending(), "import": l.import_state(), "jobs": api::jobs::running()}),
        None => serde_json::json!({"open": false, "jobs": api::jobs::running()}),
    }
}

#[tauri::command]
pub(super) fn prioritize_thumbs(state: tauri::State<AppState>, ids: Vec<i64>) -> R<()> {
    state.with_lib(|l| {
        l.prioritize_thumbs(&ids);
        Ok(())
    })
}

#[tauri::command]
pub(super) fn contributions(state: tauri::State<AppState>) -> R<serde_json::Value> {
    state.with_lib(|l| {
        let c = l.contributions();
        Ok(serde_json::json!({
            "widgets": c.widgets.iter().map(|(p, w)| serde_json::json!({"plugin": p, "id": w.id, "title": w.title, "fields": w.fields, "kinds": w.kinds, "exts": w.exts, "defaultVisible": w.default_visible, "page": w.page, "height": w.height, "selection": w.selection, "icon": w.icon, "headless": w.headless})).collect::<Vec<_>>(),
            "tabs": c.tabs.iter().map(|(p, t)| serde_json::json!({"plugin": p, "id": t.id, "title": t.title, "icon": t.icon, "page": t.page})).collect::<Vec<_>>(),
            "windows": l.plugins().iter().filter(|p| p.active() && p.source == plugins::manifest::Source::Native).flat_map(|p| p.manifest.windows.iter().map(move |w| serde_json::json!({"plugin": p.manifest.id, "id": w.id, "title": w.title, "icon": w.icon})).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "viewers": c.viewers.iter().map(|(p, exts, page, screen)| serde_json::json!({"plugin": p, "exts": exts, "page": page, "screen": screen})).collect::<Vec<_>>(),
            "names": l.plugins().iter().map(|p| (p.manifest.id.clone(), p.manifest.name.clone())).collect::<std::collections::HashMap<_, _>>(),
            "opensAsFilter": c.opens_as_filter.iter().map(|(p, exts, fields)| serde_json::json!({"plugin": p, "exts": exts, "fields": fields})).collect::<Vec<_>>(),
            "disabledExts": c.disabled_exts.iter().collect::<Vec<_>>(),
            "pageThumbExts": c.page_thumbs.iter().flat_map(|t| t.exts.iter()).collect::<Vec<_>>(),
            "viewerTools": c.viewer_tools.iter().map(|(p, t)| serde_json::json!({"plugin": p, "id": t.id, "title": t.title, "icon": t.icon, "kind": t.kind})).collect::<Vec<_>>(),
            "viewerKeys": c.viewer_keys.iter().map(|(p, k)| serde_json::json!({"plugin": p, "id": k.id, "title": k.title, "key": k.key})).collect::<Vec<_>>(),
            "actions": c.actions.iter().map(|(p, a)| serde_json::json!({"plugin": p, "id": a.id, "title": a.title, "icon": a.icon, "where": if a.places.is_empty() { vec!["item-menu".to_string()] } else { a.places.clone() }, "selection": a.selection, "kinds": a.kinds, "exts": a.exts.iter().map(|e| e.trim_start_matches('.').to_lowercase()).collect::<Vec<_>>(), "opens": a.opens, "disabled": a.disabled_reason, "key": a.key})).collect::<Vec<_>>(),
            "filters": c.filters.iter().map(|(p, f)| serde_json::json!({"plugin": p, "id": f.id, "label": f.label, "query": f.query})).collect::<Vec<_>>(),
            "sorts": c.sorts.iter().map(|(p, s)| serde_json::json!({"plugin": p, "label": s.label, "field": s.field})).collect::<Vec<_>>(),
            "panels": c.panels.iter().map(|(p, x)| serde_json::json!({"plugin": p, "id": x.id, "title": x.title, "text": x.text, "filters": x.filters.iter().map(|f| serde_json::json!({"label": f.label, "query": f.query})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "columns": c.columns.iter().map(|(p, x)| serde_json::json!({"plugin": p, "id": x.id, "label": x.label, "render": x.render, "width": x.width, "kinds": x.kinds})).collect::<Vec<_>>(),
            "formatTypes": c.format_types.iter().map(|(label, exts)| serde_json::json!({"label": label, "exts": exts})).collect::<Vec<_>>(),
            "themeVariables": c.theme_variables,
        }))
    })
}

/// Saved filter sets live with the library, in `filter-sets.json`.
#[tauri::command]
pub(super) async fn saved_filters(state: tauri::State<'_, AppState>, set: Option<serde_json::Value>) -> R<serde_json::Value> {
    let root = state.with_lib(|l| Ok(l.paths().root.clone()))?;
    let path = root.join("filter-sets.json");
    if let Some(v) = set {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        return Ok(v);
    }
    Ok(std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(|| serde_json::json!([])))
}

/// What the list view's columns show: dates, notes, colours, folders and every field of each row.
#[tauri::command]
pub(super) async fn list_cells(state: tauri::State<'_, AppState>, ids: Vec<i64>) -> R<serde_json::Value> {
    state.read(|db| {
        let mut out = serde_json::Map::new();
        for id in ids {
            let Some(a) = db.asset(id)? else { continue };
            out.insert(
                id.to_string(),
                serde_json::json!({
                    "asset": a,
                    "tags": db.asset_tags(id)?,
                    "folders": db.asset_folders(id)?,
                    "values": db.field_values(id)?.into_iter().map(|v| serde_json::json!({"field_id": v.field_id, "text": v.text, "num": v.num})).collect::<Vec<_>>(),
                }),
            );
        }
        Ok(serde_json::Value::Object(out))
    })
}

fn scope_covers(db: &crate::core::db::Db, entries: Vec<(String, Scope)>) -> crate::core::db::DbResult<serde_json::Value> {
    let cat = db.field_catalog();
    let mut rows = Vec::with_capacity(entries.len());
    let mut cover_ids = Vec::with_capacity(entries.len());
    for (key, scope) in entries {
        let first: Vec<i64> = db.search_first(&scope, "", &Sort::default(), true, &cat, Some(FOLDER_COVER_LOOK))?.into_iter().map(|(i, _)| i).collect();
        let ready: Vec<_> = db.grid_items(&first)?.into_iter().filter(|g| g.thumb_state == 1).take(FOLDER_COVERS).collect();
        let cover_id = ready.first().map(|g| g.id);
        if let Some(cid) = cover_id {
            cover_ids.push(cid);
        }
        let thumbs: Vec<serde_json::Value> = ready.into_iter().map(|g| serde_json::json!({"uid": g.uid, "ver": g.thumb_ver})).collect();
        let count = db.search_count(&scope, "", true, &cat)?;
        rows.push((key, cover_id, thumbs, count));
    }
    let palettes = if cover_ids.is_empty() { std::collections::HashMap::new() } else { db.palettes_of(&cover_ids)? };
    let mut out = serde_json::Map::new();
    for (key, cover_id, thumbs, count) in rows {
        let color = cover_id.and_then(|id| palettes.get(&id)).and_then(|p| crate::core::images::color::spectrum_point(p)).map(|(h, c, l)| [h, c, l]);
        out.insert(key, serde_json::json!({ "thumbs": thumbs, "count": count, "color": color }));
    }
    Ok(serde_json::Value::Object(out))
}

/// Pictures for subfolder cards: the first few items with a thumbnail in each folder.
#[tauri::command]
pub(super) async fn folder_covers(state: tauri::State<'_, AppState>, ids: Vec<i64>) -> R<serde_json::Value> {
    state.read(|db| scope_covers(db, ids.into_iter().map(|id| (id.to_string(), Scope::Folder(id))).collect()))
}

/// Pictures for location subfolder cards: the first few items with a thumbnail in each relative directory of a location.
#[tauri::command]
pub(super) async fn root_dir_covers(state: tauri::State<'_, AppState>, root: i64, paths: Vec<String>) -> R<serde_json::Value> {
    state.read(|db| scope_covers(db, paths.into_iter().map(|dir| (dir.clone(), Scope::RootDir(root, dir))).collect()))
}

const FOLDER_COVER_LOOK: usize = 12;

const FOLDER_COVERS: usize = 3;

#[tauri::command]
pub(super) async fn tag_group_items(state: tauri::State<'_, AppState>, id: i64) -> R<Vec<i64>> {
    state.read(|db| db.tag_group_items(id))
}

#[tauri::command]
pub(super) async fn paths_of(state: tauri::State<'_, AppState>, ids: Vec<i64>) -> R<Vec<String>> {
    state.read(|db| Ok(ids.iter().filter_map(|i| db.abs_path(*i)).map(|p| p.to_string_lossy().to_string()).collect()))
}

#[tauri::command]
pub(super) async fn selection_summary(state: tauri::State<'_, AppState>, ids: Vec<i64>) -> R<serde_json::Value> {
    state.read(|db| db.selection_summary(&ids))
}

/// How many of the selected items `selection.changed` names.
const SELECTION_TOLD: usize = 1000;

#[tauri::command]
pub(super) async fn set_selection(window: tauri::WebviewWindow, state: tauri::State<'_, AppState>, ids: Vec<i64>) -> R<()> {
    if window.label() != "main" {
        return Ok(());
    }
    let first: Vec<i64> = ids.iter().take(SELECTION_TOLD).copied().collect();
    let count = ids.len();
    {
        let mut sel = state.selection.lock();
        if *sel == ids {
            return Ok(());
        }
        *sel = ids;
    }
    if let Some(lib) = state.library_id() {
        let uids: Vec<String> = state.read(|db| db.uids(&first)).unwrap_or_default().into_iter().map(|n| n.1).collect();
        let w = state.me.lock().window.clone();
        state.hub.publish("selection.changed", Some(&lib), serde_json::json!({"window": w, "count": count, "ids": uids}));
    }
    Ok(())
}

/// The items of a folder and its subfolders, for exporting or sending elsewhere.
#[tauri::command]
pub(super) async fn folder_items(state: tauri::State<'_, AppState>, id: i64) -> R<Vec<i64>> {
    state.read(|db| {
        let cat = db.field_catalog();
        Ok(db.search(&Scope::Folder(id), "", &Sort::default(), true, &cat)?.into_iter().map(|(i, _)| i).collect())
    })
}
