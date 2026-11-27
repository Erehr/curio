//! A library's backups: made, listed, pruned and restored on the next start.

use crate::core::library::LibraryPaths;
use crate::core::export::export_json;
use std::io::Read;
use std::path::{Path, PathBuf};
use crate::core::db::{Db, DbResult};
use crate::core::model::*;

pub const BACKUP_FORMAT: &str = "curio-backup/1";

const BACKUP_FILES: [&str; 3] = ["library.db", "metadata.json", "manifest.json"];

/// `manifest.json` of a finished backup. Only folders with one were checked when made.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BackupManifest {
    pub format: String,
    pub created_at: i64,
    pub schema: i64,
    pub app_version: String,
    pub library_id: String,
    /// the computer that made it
    #[serde(default)]
    pub installation_id: String,
    /// "manual" or "scheduled"
    pub kind: String,
    /// size and BLAKE3 of `library.db`
    pub size: u64,
    pub blake3: String,
}

pub fn read_manifest(dir: &Path) -> Option<BackupManifest> {
    let m: BackupManifest = serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).ok()?).ok()?;
    (m.format == BACKUP_FORMAT).then_some(m)
}

/// Full backup: consistent database copy + portable JSON next to it. Written to `<stamp>.tmp`,
/// checked, described in `manifest.json`, then renamed, so a folder named `<stamp>` is complete.
pub fn backup(db: &Db, dest: &Path, library_id: &str, kind: &str) -> DbResult<PathBuf> {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let mut name = stamp.clone();
    let mut n = 1;
    while dest.join(&name).exists() || dest.join(format!("{name}.tmp")).exists() {
        n += 1;
        name = format!("{stamp}-{n}");
    }
    let tmp = dest.join(format!("{name}.tmp"));
    let dir = dest.join(&name);
    std::fs::create_dir_all(&tmp).map_err(|e| format!("can't create {}: {e}", tmp.display()))?;
    let made = (|| {
        let file = tmp.join("library.db");
        db.backup_to(&file)?;
        let copy = Db::open_unmigrated(&file)?;
        let ok = copy.integrity_check(true)?;
        if ok != vec!["ok".to_string()] {
            return Err(format!("the copy failed its check: {}", ok.join("; ")));
        }
        let schema = copy.schema_version();
        drop(copy);
        let ids: Vec<AssetId> = {
            let mut st = db.conn.prepare("SELECT id FROM assets WHERE part_of IS NULL").map_err(|e| e.to_string())?;
            let rows = st.query_map([], |r| r.get(0)).map_err(|e| e.to_string())?;
            rows.filter_map(|r| r.ok()).collect()
        };
        export_json(db, &ids, &tmp.join("metadata.json"))?;
        let (size, blake3) = hash_file(&file)?;
        let m = BackupManifest {
            format: BACKUP_FORMAT.into(),
            created_at: crate::core::model::now_ms(),
            schema,
            app_version: env!("CURIO_VERSION").into(),
            library_id: library_id.into(),
            installation_id: crate::core::model::installation_id().into(),
            kind: kind.into(),
            size,
            blake3,
        };
        std::fs::write(tmp.join("manifest.json"), serde_json::to_vec_pretty(&m).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &dir).map_err(|e| e.to_string())
    })();
    if let Err(err) = made {
        remove_backup_folder(&tmp);
        return Err(err);
    }
    Ok(dir)
}

fn hash_file(p: &Path) -> Result<(u64, String), String> {
    let mut f = std::fs::File::open(p).map_err(|e| e.to_string())?;
    let mut h = blake3::Hasher::new();
    let size = std::io::copy(&mut f, &mut h).map_err(|e| e.to_string())?;
    Ok((size, h.finalize().to_hex().to_string()))
}

/// Remove only the files a backup consists of, then the folder if nothing else is in it.
fn remove_backup_folder(dir: &Path) {
    for f in BACKUP_FILES {
        let _ = std::fs::remove_file(dir.join(f));
    }
    let _ = std::fs::remove_dir(dir);
}

/// Keep the newest `keep` checked backups of this library (0 keeps all). Folders without a
/// manifest, another library's backups and pre-migration copies are never touched. Unfinished
/// `.tmp` folders older than an hour are removed. Returns how many backups were removed.
pub fn prune_backups(dest: &Path, keep: usize, library_id: &str) -> usize {
    let Ok(rd) = std::fs::read_dir(dest) else { return 0 };
    let mut ours: Vec<(i64, PathBuf)> = vec![];
    for d in rd.flatten() {
        let p = d.path();
        if !p.is_dir() {
            continue;
        }
        let name = d.file_name().to_string_lossy().to_string();
        if name.ends_with(".tmp") {
            let old = d.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age.as_secs() > 3600);
            if old {
                remove_backup_folder(&p);
            }
            continue;
        }
        if let Some(m) = read_manifest(&p) {
            let intact = std::fs::metadata(p.join("library.db")).is_ok_and(|f| f.len() == m.size);
            if intact && m.library_id == library_id {
                ours.push((m.created_at, p));
            }
        }
    }
    if keep == 0 || ours.len() <= keep {
        return 0;
    }
    ours.sort_by(|a, b| b.0.cmp(&a.0));
    let mut n = 0;
    for (_, p) in ours.into_iter().skip(keep) {
        remove_backup_folder(&p);
        n += 1;
    }
    n
}

/// `<db>-wal` / `<db>-shm` next to a database file.
fn side_file(db: &Path, suffix: &str) -> PathBuf {
    let mut s = db.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

fn remove_side_files(db: &Path) {
    for s in ["-wal", "-shm", "-journal"] {
        let _ = std::fs::remove_file(side_file(db, s));
    }
}

/// Is this file a Curio library database this build can open? Checked without migrating or
/// writing. `Ok(Some(..))` is a warning: it belongs to another library.
pub fn check_restore_file(file: &Path, library_id: Option<&str>) -> Result<Option<String>, String> {
    let mut head = [0u8; 16];
    std::fs::File::open(file)
        .and_then(|mut f| f.read_exact(&mut head))
        .map_err(|_| "Not a usable backup.".to_string())?;
    if &head != b"SQLite format 3\0" {
        return Err("The file isn't a database.".into());
    }
    let db = Db::open_unmigrated(file).map_err(|e| format!("The file can't be read as a database: {e}"))?;
    let ok = db.integrity_check(true).map_err(|e| format!("The backup can't be checked: {e}"))?;
    if ok != vec!["ok".to_string()] {
        return Err(format!("The backup is damaged: {}", ok.join("; ")));
    }
    let has_assets = db.conn.query_row("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'assets'", [], |_| Ok(())).is_ok();
    if db.schema_version() < 1 || !has_assets {
        return Err("The database isn't a Curio library.".into());
    }
    let theirs = db.meta_get("library_id");
    Ok(match (theirs, library_id) {
        (Some(t), Some(ours)) if t != ours => Some("This backup was made from a different library. Restoring it replaces everything in this one.".into()),
        _ => None,
    })
}

pub enum Staged {
    Ready,
    /// a warning the user has to accept first; nothing was staged
    NeedsConfirm(String),
}

/// Copy a backup next to the library as `restore-pending.db`, to replace the database on the
/// next open. The copy is checked before it gets that name, so a half-written or foreign file is
/// never staged, and the live database isn't touched here.
pub fn stage_restore(paths: &LibraryPaths, backup_db: &Path, library_id: &str, confirmed: bool) -> Result<Staged, String> {
    let tmp = paths.root.join("restore-pending.db.tmp");
    let _ = std::fs::remove_file(&tmp);
    remove_side_files(&tmp);
    let staged = (|| {
        std::fs::copy(backup_db, &tmp).map_err(|e| format!("Couldn't copy the backup: {e}"))?;
        // a database picked from a library folder may still have part of its content in its WAL
        let wal = side_file(backup_db, "-wal");
        if wal.is_file() {
            std::fs::copy(&wal, side_file(&tmp, "-wal")).map_err(|e| format!("Couldn't copy the backup: {e}"))?;
            let c = rusqlite::Connection::open(&tmp).map_err(|e| e.to_string())?;
            c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").map_err(|e| format!("Couldn't read the backup: {e}"))?;
        }
        remove_side_files(&tmp);
        // an older backup is migrated, with its own pre-migration copy, when the library opens
        let warning = check_restore_file(&tmp, Some(library_id))?;
        remove_side_files(&tmp);
        if let (Some(w), false) = (warning, confirmed) {
            return Ok(Staged::NeedsConfirm(w));
        }
        std::fs::rename(&tmp, paths.root.join("restore-pending.db")).map_err(|e| e.to_string())?;
        Ok(Staged::Ready)
    })();
    if !matches!(staged, Ok(Staged::Ready)) {
        let _ = std::fs::remove_file(&tmp);
        remove_side_files(&tmp);
    }
    staged
}

const RESTORE_RENAME_TRIES: u32 = 50;

pub enum Restore {
    Nothing,
    Applied,
    /// the staged file failed its check again and was set aside; the library is unchanged
    Rejected(String),
}

/// Rename, retrying while the process being replaced closes its handles.
fn rename_patiently(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut tries = 0;
    loop {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(err) if tries + 1 >= RESTORE_RENAME_TRIES => return Err(err),
            Err(_) => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}

/// Called before opening a library: apply a staged restore. The current database moves, with its
/// WAL and shared-memory files, into `before-restore-<stamp>/`, so a stale WAL never meets the
/// restored file.
pub fn apply_pending_restore(paths: &LibraryPaths) -> Result<Restore, String> {
    apply_restore_with(paths, rename_patiently)
}

fn apply_restore_with(paths: &LibraryPaths, put_in_place: impl Fn(&Path, &Path) -> std::io::Result<()>) -> Result<Restore, String> {
    let pending = paths.root.join("restore-pending.db");
    if !pending.exists() {
        return Ok(Restore::Nothing);
    }
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    if let Err(why) = check_restore_file(&pending, None) {
        remove_side_files(&pending);
        let aside = paths.root.join(format!("restore-pending.db.rejected-{stamp}"));
        let _ = std::fs::rename(&pending, &aside);
        return Ok(Restore::Rejected(why));
    }
    remove_side_files(&pending);
    if paths.db.exists() {
        let kept = paths.root.join(format!("before-restore-{stamp}"));
        std::fs::create_dir_all(&kept).map_err(|e| e.to_string())?;
        let mut moved: Vec<(PathBuf, PathBuf)> = vec![];
        let put_back = |moved: &[(PathBuf, PathBuf)]| {
            for (f, t) in moved.iter().rev() {
                let _ = rename_patiently(t, f);
            }
            let _ = std::fs::remove_dir(&kept);
        };
        for (from, name) in [(side_file(&paths.db, "-wal"), "library.db-wal"), (side_file(&paths.db, "-shm"), "library.db-shm"), (paths.db.clone(), "library.db")] {
            if !from.exists() {
                continue;
            }
            let to = kept.join(name);
            if let Err(err) = rename_patiently(&from, &to) {
                put_back(&moved);
                return Err(format!("Couldn't set the current database aside to restore the backup: {err}"));
            }
            moved.push((from, to));
        }
        if let Err(err) = put_in_place(&pending, &paths.db) {
            put_back(&moved);
            return Err(format!("Couldn't put the backup in place: {err}"));
        }
        return Ok(Restore::Applied);
    }
    put_in_place(&pending, &paths.db).map_err(|e| format!("Couldn't put the backup in place: {e}"))?;
    Ok(Restore::Applied)
}

/// One restorable database, for the backup list in Settings.
#[derive(Clone, Debug, serde::Serialize)]
pub struct BackupEntry {
    /// the `library.db` to restore
    pub path: PathBuf,
    pub created_at: i64,
    /// manual, scheduled, pre-migration, before-restore, or unknown
    pub kind: String,
    pub schema: Option<i64>,
    pub size: u64,
    /// made with a manifest and still the size it says
    pub checked: bool,
    pub other_library: bool,
}

/// Backups found in the given folders (each scanned one level deep), newest first.
pub fn list_backups(dirs: &[PathBuf], library_id: &str) -> Vec<BackupEntry> {
    let mut out: Vec<BackupEntry> = vec![];
    let mut seen = std::collections::HashSet::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        for d in rd.flatten() {
            let name = d.file_name().to_string_lossy().to_string();
            let db = d.path().join("library.db");
            if name.ends_with(".tmp") || !db.is_file() || !seen.insert(db.clone()) {
                continue;
            }
            let Ok(meta) = std::fs::metadata(&db) else { continue };
            let mtime = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0);
            let e = match read_manifest(&d.path()) {
                Some(m) => BackupEntry {
                    checked: m.size == meta.len(),
                    other_library: m.library_id != library_id,
                    created_at: m.created_at,
                    kind: m.kind,
                    schema: Some(m.schema),
                    size: meta.len(),
                    path: db,
                },
                None => BackupEntry {
                    kind: ["pre-migration", "before-restore"].into_iter().find(|k| name.starts_with(k)).unwrap_or("unknown").into(),
                    created_at: mtime,
                    schema: name.strip_prefix("pre-migration-v").and_then(|s| s.split('-').next()).and_then(|v| v.parse().ok()),
                    checked: false,
                    other_library: false,
                    size: meta.len(),
                    path: db,
                },
            };
            out.push(e);
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out
}
