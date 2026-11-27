//! A library's upkeep: backups when due, maintenance and old temporary files.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::core::db::{Db, DbResult};
use crate::core::system::logging;
use crate::core::model::*;
use crate::core::said::Said;
use crate::core::thumbs;
use crate::core::library::settings::HashPolicy;
use crate::core::library::LibEvent;
use crate::core::library::Ctx;

/// Last backup results, kept in `meta` for Settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BackupStatus {
    pub last_ok_at: Option<i64>,
    pub last_ok_dir: Option<String>,
    pub last_error_at: Option<i64>,
    pub last_error: Option<String>,
}

pub fn backup_status(db: &Db) -> BackupStatus {
    db.meta_get("backup_status").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub(super) fn backup_due(db: &Db, ctx: &Ctx) -> bool {
    let every = ctx.settings.read().backup_every_hours;
    if every == 0 || !ctx.backup_dirty.load(Ordering::Relaxed) {
        return false;
    }
    backup_status(db).last_ok_at.is_none_or(|t| now_ms() - t >= every as i64 * 3_600_000)
}

/// Back up, record the result and apply retention. Errors are also sent to the window.
pub(super) fn run_backup(db: &Db, ctx: &Ctx, kind: &str) -> DbResult<PathBuf> {
    *ctx.busy.lock() = Some("Backing up".into());
    // edits made while the copy runs mark the library changed again
    let was_dirty = ctx.backup_dirty.swap(false, Ordering::Relaxed);
    let dest = ctx.backup_dest();
    let res = crate::core::library::backup::backup(db, &dest, &ctx.library_id, kind);
    let mut st = backup_status(db);
    match &res {
        Ok(dir) => {
            st.last_ok_at = Some(now_ms());
            st.last_ok_dir = Some(dir.to_string_lossy().to_string());
            st.last_error = None;
            st.last_error_at = None;
            if !ctx.backup_dirty.load(Ordering::Relaxed) {
                let _ = db.meta_set("backup_dirty", "0");
            }
            let removed = crate::core::library::backup::prune_backups(&dest, ctx.settings.read().backup_keep, &ctx.library_id);
            logging::info(&format!("{kind} backup saved to {}; {removed} old backup(s) removed", dir.display()));
        }
        Err(e) => {
            if was_dirty {
                ctx.backup_dirty.store(true, Ordering::Relaxed);
            }
            st.last_error_at = Some(now_ms());
            st.last_error = Some(e.clone());
            logging::error(&format!("{kind} backup failed: {e}"));
            ctx.send(LibEvent::Error(Said::new("errors.backup.failed", json!({"error": e}), format!("Backup failed: {e}"))));
        }
    }
    let _ = db.meta_set("backup_status", &serde_json::to_string(&st).unwrap_or_default());
    *ctx.busy.lock() = None;
    res
}

pub(super) fn run_maintenance(db: &Db, ctx: &Ctx) {
    if let Err(e) = db.optimize() {
        logging::warn(&format!("optimize: {e}"));
    }
    let limit = ctx.settings.read().cache_limit_mb * 1024 * 1024;
    let size = thumbs::enforce_cache_limit(&[&ctx.paths.thumbs, &ctx.paths.previews], limit);
    clean_tmp(&ctx.paths.tmp);
    let stored = thumbs::remove_stored_previews(&ctx.paths.previews);
    if stored > 0 {
        logging::info(&format!("removed {stored} viewer previews kept on disk by earlier versions"));
    }
    logging::info(&format!("maintenance done; thumbnail cache {} MB", size / 1024 / 1024));
    match crate::core::store::orphans::managed_orphans(db, &ctx.paths, true) {
        Ok(o) if !o.is_empty() => logging::info(&format!("{} files in the library's own folder belong to no item; see Settings › Storage & backup", o.len())),
        Ok(_) => {}
        Err(e) => logging::warn(&format!("looking for stray files: {e}")),
    }
    // hashing policies
    let policy = ctx.settings.read().hash_policy;
    if policy != HashPolicy::OnDemand {
        let limit = if policy == HashPolicy::SmallFiles { ctx.settings.read().hash_small_limit_mb * 1024 * 1024 } else { u64::MAX };
        let ids: Vec<(AssetId, i64)> = {
            let Ok(mut st) = db.conn.prepare("SELECT id, size FROM assets WHERE content_hash IS NULL AND status = 0 AND part_of IS NULL LIMIT 5000") else { return };
            st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map(|r| r.filter_map(|x| x.ok()).collect()).unwrap_or_default()
        };
        for (id, size) in ids {
            if ctx.cancel.load(Ordering::Relaxed) {
                break;
            }
            if size as u64 > limit {
                continue;
            }
            if let Some(p) = db.abs_path(id) {
                if let Ok(h) = crate::core::system::filesystem::content_hash(&p) {
                    let _ = db.set_content_hash(id, &h);
                }
            }
        }
    }
}

/// Leftovers in the cache's tmp folder; recent ones may belong to work still under way.
fn clean_tmp(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let young = e.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_none_or(|age| age < Duration::from_secs(600));
        if young {
            continue;
        }
        let p = e.path();
        let _ = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
    }
}
