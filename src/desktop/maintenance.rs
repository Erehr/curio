//! Export, backups and their restore, and a library's maintenance.

use crate::desktop::AppState;
use crate::desktop::R;
use std::path::{Path, PathBuf};
use crate::core::library::{changes::Changed, LibEvent, changes::Parts};
use crate::core::said::Said;
use crate::desktop::config::save_config;
use crate::desktop::config::load_config;

#[tauri::command]
pub(super) async fn export(state: tauri::State<'_, AppState>, kind: String, ids: Vec<i64>) -> R<bool> {
    let target: Option<PathBuf> = match kind.as_str() {
        "json" => rfd::FileDialog::new().set_file_name("metadata.json").save_file(),
        "csv" => rfd::FileDialog::new().set_file_name("metadata.csv").save_file(),
        "sidecar-folder" => rfd::FileDialog::new().pick_folder(),
        "sidecar-beside" => Some(PathBuf::new()),
        _ => None,
    };
    let Some(t) = target else { return Ok(false) };
    state.with_lib(|lib| {
        lib.job(move |db, ctx| {
            let (n, refused) = match kind.as_str() {
                "json" => (crate::core::export::export_json(db, &ids, &t)?, 0),
                "csv" => (crate::core::export::export_csv(db, &ids, &t)?, 0),
                "sidecar-folder" => crate::core::export::write_sidecars(db, &ids, Some(&t))?,
                _ => crate::core::export::write_sidecars(db, &ids, None)?,
            };
            let s = if n == 1 { "" } else { "s" };
            let msg = match refused {
                0 => Said::new("results.export.done", serde_json::json!({"count": n}), format!("Exported {n} item{s}")),
                r => Said::new("results.export.someSkipped", serde_json::json!({"count": n, "skipped": r}), format!("Exported {n} item{s}; skipped in read-only locations: {r}")),
            };
            let _ = ctx.events.send(LibEvent::Done(msg));
            Ok(Default::default())
        });
        Ok(true)
    })
}

/// A staged restore can only replace the database once this process's connections are gone, so a new process takes over. The library lock stays held until this one exits, which is what the new one waits for.
fn relaunch_on(app: &tauri::AppHandle, state: &AppState, lib: &Path) -> R<()> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(exe).arg("--relaunch").arg("--library").arg(lib).spawn().map_err(|e| format!("Restart Curio to finish restoring the backup: {e}"))?;
    if let Some(old) = state.lib.lock().take() {
        old.shutdown();
    }
    state.readers.lock().clear();
    app.exit(0);
    Ok(())
}

/// Backups of the open library: its backup folder, plus pre-migration copies and databases set
/// aside by earlier restores.
#[tauri::command]
pub(super) async fn backup_list(state: tauri::State<'_, AppState>) -> R<Vec<crate::core::library::backup::BackupEntry>> {
    let (dirs, id) = state.with_lib(|l| Ok((vec![l.ctx.backup_dest(), l.paths().backups.clone(), l.paths().root.clone()], l.ctx.library_id.clone())))?;
    Ok(crate::core::library::backup::list_backups(&dirs, &id))
}

/// Restore `path` (empty: pick a file). Returns `{confirm}` when the user must accept a warning
/// first (call again with `confirmed`), or `{message}` once the app is relaunching.
#[tauri::command]
pub(super) async fn backup_restore(app: tauri::AppHandle, state: tauri::State<'_, AppState>, path: String, confirmed: bool) -> R<serde_json::Value> {
    let (lp, dest, id) = state.with_lib(|l| Ok((l.paths().clone(), l.ctx.backup_dest(), l.ctx.library_id.clone())))?;
    let file = if path.is_empty() {
        let Some(f) = rfd::FileDialog::new().set_directory(&dest).add_filter(crate::desktop::locale::text("dialog.database", &[]), &["db"]).pick_file() else { return Ok(serde_json::json!({})) };
        f
    } else {
        PathBuf::from(path)
    };
    match crate::core::library::backup::stage_restore(&lp, &file, &id, confirmed)? {
        crate::core::library::backup::Staged::NeedsConfirm(w) => Ok(serde_json::json!({ "confirm": w, "path": file })),
        crate::core::library::backup::Staged::Ready => {
            relaunch_on(&app, &state, &lp.root)?;
            Ok(serde_json::json!({ "message": "Restarting to restore the backup" }))
        }
    }
}

#[tauri::command]
pub(super) async fn maintenance(state: tauri::State<'_, AppState>, op: String) -> R<String> {
    if op == "backup-dir" || op == "backup-default" {
        // the dialog runs without the library lock held
        let dir = if op == "backup-dir" {
            let Some(d) = rfd::FileDialog::new().pick_folder() else { return Ok(String::new()) };
            d.to_string_lossy().to_string()
        } else {
            String::new()
        };
        return state.with_lib(|lib| {
            let mut patch = serde_json::Map::new();
            patch.insert("backup_dir".to_string(), serde_json::Value::String(dir));
            lib.patch_settings(patch);
            Ok(format!("New backups go to {}", lib.ctx.backup_dest().display()))
        });
    }
    if op == "cache-dir" || op == "cache-default" {
        let mut cfg = load_config();
        cfg.cache_dir = if op == "cache-dir" { rfd::FileDialog::new().pick_folder() } else { None };
        save_config(&cfg);
        return Ok("Applies when the library reopens".into());
    }
    // on a pooled connection: the status poll waits on the library lock, so a long check under it would freeze the window
    if op == "integrity" {
        return state.read(|db| Ok(format!("Integrity check: {}", db.integrity_check(false)?.join(", "))));
    }
    if op == "paths" {
        let (root, thumbs, backups, id) = state.with_lib(|lib| Ok((lib.paths().root.clone(), lib.paths().thumbs.clone(), lib.ctx.backup_dest(), lib.ctx.library_id.clone())))?;
        let (schema, backup) = state.read(|db| Ok((db.schema_version(), crate::core::library::upkeep::backup_status(db))))?;
        return Ok(serde_json::json!({"root": root, "thumbs": thumbs, "backups": backups, "schema": schema, "library_id": id, "installation_id": crate::core::model::installation_id(), "backup": backup}).to_string());
    }
    state.with_lib(|lib| match op.as_str() {
        "backup" => {
            lib.backup_now();
            Ok("Backing up".into())
        }
        "optimize" => {
            lib.maintenance();
            Ok("Optimizing in the background".into())
        }
        "rescan" => {
            lib.scan_all();
            Ok("Rescanning all locations".into())
        }
        "clear-thumbs" => {
            let thumbs = lib.paths().thumbs.clone();
            lib.job(move |db, _| {
                let _ = std::fs::remove_dir_all(&thumbs);
                let _ = std::fs::create_dir_all(&thumbs);
                db.conn.execute("UPDATE assets SET thumb_state = 0, thumb_ver = thumb_ver + 1", []).map_err(|e| e.to_string())?;
                Ok(Changed::parts(Parts::SIDEBAR))
            });
            lib.kick_thumbs();
            Ok("Thumbnails will be rebuilt".into())
        }
        other => Err(format!("unknown op {other}")),
    })
}
