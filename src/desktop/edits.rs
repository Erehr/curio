//! Folder, tag, smart folder and trash edits, the store's file operations and its orphans.

use crate::desktop::R;
use crate::desktop::AppState;
use std::path::PathBuf;
use crate::core::library::{changes::Changed, LibEvent, changes::Parts};
use crate::core::library::edits;
use crate::core::said::Said;

/// Delete in a folder view, and its undo (`deleted` false).
#[tauri::command]
pub(super) fn trash_in_folder(state: tauri::State<AppState>, ids: Vec<i64>, folder: i64, deleted: bool) -> R<()> {
    state.write(move |db| if deleted { edits::trash_in_folder(db, &ids, folder) } else { edits::restore_in_folder(db, &ids, folder) })
}

#[tauri::command]
pub(super) fn restore_to_folder(state: tauri::State<AppState>, ids: Vec<i64>, folder: i64) -> R<()> {
    state.write(move |db| edits::restore_to_folder(db, &ids, folder))
}

/// Files in the library's own folder that belong to no item: `list`, or `adopt` / `recycle` the given ones.
#[tauri::command]
pub(super) async fn managed_orphans(state: tauri::State<'_, AppState>, action: String, paths: Vec<String>) -> R<serde_json::Value> {
    let lp = state.with_lib(|l| Ok(l.paths().clone()))?;
    if action == "list" {
        let found = state.read(|db| crate::core::store::orphans::managed_orphans(db, &lp, false))?;
        let items: Vec<String> = found.iter().take(200).map(|p| p.to_string_lossy().to_string()).collect();
        return Ok(serde_json::json!({ "count": found.len(), "items": items }));
    }
    let files: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let adopt = match action.as_str() {
        "adopt" => true,
        "recycle" => false,
        other => return Err(format!("unknown action {other}")),
    };
    state.with_lib(|l| {
        l.job(move |db, ctx| {
            if adopt {
                let ids = crate::core::store::orphans::adopt_orphans(db, &lp, &files)?;
                ctx.send(LibEvent::Done(Said::new("results.orphans.adopted", serde_json::json!({"count": ids.len()}), format!("{} files added back to the library", ids.len()))));
                return Ok(Changed::added(&ids, "window"));
            }
            let n = crate::core::store::orphans::recycle_orphans(db, &lp, &files)?;
            ctx.send(LibEvent::Done(Said::new("results.orphans.recycled", serde_json::json!({"count": n}), format!("{n} files moved to the Recycle Bin"))));
            Ok(Changed::default())
        });
        if adopt {
            l.kick_thumbs();
        }
        Ok(serde_json::json!({}))
    })
}

/// File work that is waiting or failed (`list`), or act on failed ops: `retry` or `forget` (the file stays where it is).
#[tauri::command]
pub(super) async fn file_ops(state: tauri::State<'_, AppState>, action: String, ids: Vec<i64>) -> R<serde_json::Value> {
    if action == "list" {
        return state.read(|db| {
            let (pending, failed) = db.file_op_counts()?;
            let mut items = vec![];
            for op in db.file_ops(None, Some("failed"), 200)? {
                let path = db.file_op_path(&op).map(|p| p.to_string_lossy().to_string()).unwrap_or(op.rel_path.clone());
                items.push(serde_json::json!({ "id": op.id, "kind": op.kind, "path": path, "error": op.error, "at": op.created_at }));
            }
            Ok(serde_json::json!({ "pending": pending, "failed": failed, "items": items }))
        });
    }
    let retry = match action.as_str() {
        "retry" => true,
        "forget" => false,
        other => return Err(format!("unknown action {other}")),
    };
    state.write_wait(move |db| {
        db.tx(|db| {
            for id in &ids {
                if retry {
                    db.conn.execute("UPDATE file_ops SET state = 'pending', error = NULL WHERE id = ? AND state = 'failed' AND kind IN ('recycle', 'place', 'unplace', 'move', 'move_dir')", [id]).map_err(|e| e.to_string())?;
                } else {
                    db.conn.execute("DELETE FROM file_ops WHERE id = ? AND state = 'failed'", [id]).map_err(|e| e.to_string())?;
                }
            }
            Ok(())
        })
    })?;
    state.with_lib(|l| {
        l.kick_file_ops();
        Ok(())
    })?;
    Ok(serde_json::json!({}))
}

#[tauri::command]
pub(super) async fn folder_op(state: tauri::State<'_, AppState>, op: String, id: Option<i64>, ids: Option<Vec<i64>>, name: Option<String>, parent: Option<i64>, value: Option<String>, before: Option<i64>) -> R<()> {
    state.write_answered(move |db| edits::folder_op(db, &op, edits::FolderArgs { id, ids: ids.unwrap_or_default(), name, parent, value, before })).await
}

#[tauri::command]
pub(super) async fn tag_op(state: tauri::State<'_, AppState>, op: String, id: Option<i64>, name: Option<String>, group: Option<i64>, color: Option<String>) -> R<()> {
    state.write_answered(move |db| edits::tag_op(db, &op, edits::TagArgs { id, name, group, color })).await
}

#[tauri::command]
pub(super) async fn smart_op(state: tauri::State<'_, AppState>, op: String, id: Option<i64>, name: Option<String>, query: Option<String>, icon: Option<String>, parent: Option<i64>, before: Option<i64>) -> R<()> {
    state.write_answered(move |db| edits::smart_op(db, &op, edits::SmartArgs { id, name, query, icon, group_by: None, parent, before })).await
}

/// Add, remove or reorder favorites. `kind` is folder, smart, root or tag.
#[tauri::command]
pub(super) fn favorite_op(state: tauri::State<AppState>, op: String, kind: Option<String>, id: Option<i64>, list: Option<Vec<serde_json::Value>>) -> R<()> {
    state.write(move |db| {
        let mut favs = db.favorites();
        let same = |v: &serde_json::Value| v["kind"].as_str() == kind.as_deref() && v["id"].as_i64() == id;
        match op.as_str() {
            "add" => {
                if !favs.iter().any(same) {
                    favs.push(serde_json::json!({ "kind": kind, "id": id }));
                }
            }
            "remove" => favs.retain(|v| !same(v)),
            "set" => favs = list.unwrap_or_default(),
            other => return Err(format!("unknown favorite op {other}")),
        }
        db.set_favorites(&favs)?;
        Ok(Changed { parts: Parts::SIDEBAR, ..Default::default() })
    })
}
