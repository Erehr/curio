//! Files in the managed store that no item has: found, adopted as items or recycled.

use crate::core::import::insert_managed_file;
use crate::core::store::managed_root;
use crate::core::store::LibraryPaths;
use std::collections::HashSet;
use std::path::PathBuf;
use crate::core::db::{fold_path, Db, DbResult};
use crate::core::system::filesystem;
use crate::core::model::*;
use crate::core::store::{self};

const TEMP_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(24 * 3600);

/// Files in the managed store that no item uses and no file op covers.
/// With `clean_temps`, our unfinished copies older than a day are removed; younger ones may still be in progress.
pub fn managed_orphans(db: &Db, paths: &LibraryPaths, clean_temps: bool) -> DbResult<Vec<PathBuf>> {
    let Some(root_id) = db.conn.query_row("SELECT id FROM roots WHERE kind = 'managed'", [], |r| r.get::<_, i64>(0)).ok() else { return Ok(vec![]) };
    let mut known: HashSet<String> = {
        let mut st = db.conn.prepare("SELECT rel_path FROM assets WHERE root_id = ?1 UNION SELECT rel_path FROM placements").map_err(|e| e.to_string())?;
        let rows = st.query_map([root_id], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).map(|r| fold_path(&r)).collect()
    };
    for op in db.file_ops(None, None, i64::MAX)?.into_iter().filter(|o| o.root_id == root_id) {
        if let Some(d) = op.dest {
            known.insert(fold_path(&d));
        }
        if let Some(t) = &op.tmp {
            let tmp_rel = match op.rel_path.rsplit_once('/') {
                Some((dir, _)) => format!("{dir}/{t}"),
                None => t.clone(),
            };
            known.insert(fold_path(&tmp_rel));
        }
        known.insert(fold_path(&op.rel_path));
    }
    let mut out = vec![];
    let walker = walkdir::WalkDir::new(&paths.managed).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        e.depth() == 0 || filesystem::is_temp_name(&name) || !filesystem::is_ignored_name(&name)
    });
    for entry in walker.filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {
        let rel = filesystem::rel_to_db(entry.path().strip_prefix(&paths.managed).unwrap_or(entry.path()));
        if known.contains(&fold_path(&rel)) {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if filesystem::is_temp_name(&name) {
            let old = entry.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|age| age > TEMP_MAX_AGE);
            if clean_temps && old {
                match std::fs::remove_file(entry.path()) {
                    Ok(()) => crate::core::system::logging::info(&format!("removed an unfinished copy: {}", entry.path().display())),
                    Err(e) => crate::core::system::logging::warn(&format!("couldn't remove {}: {e}", entry.path().display())),
                }
            }
            continue;
        }
        out.push(entry.into_path());
    }
    Ok(out)
}

/// The given files, if they are still orphans of the managed store. Anything else is dropped, so a caller can't reach outside it.
fn still_orphans(db: &Db, paths: &LibraryPaths, files: &[PathBuf]) -> DbResult<Vec<PathBuf>> {
    let orphans: HashSet<PathBuf> = managed_orphans(db, paths, false)?.into_iter().collect();
    Ok(files.iter().filter(|f| orphans.contains(*f)).cloned().collect())
}

/// Make items for orphaned files of the managed store, where they are. A file in a folder's directory joins that folder; a directory no folder has becomes one, named after it.
pub fn adopt_orphans(db: &mut Db, paths: &LibraryPaths, files: &[PathBuf]) -> DbResult<Vec<AssetId>> {
    let files = still_orphans(db, paths, files)?;
    let root_id = managed_root(db, paths)?;
    let mut ids = vec![];
    for chunk in files.chunks(500) {
        let mut folders = vec![];
        for f in chunk {
            let rel = filesystem::rel_to_db(f.strip_prefix(&paths.managed).unwrap_or(f));
            let dir = rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
            folders.push(if dir.is_empty() { None } else { Some(store::folder_for_dir(db, dir)?) });
        }
        ids.extend(db.tx(|db| chunk.iter().zip(&folders).map(|(f, folder)| insert_managed_file(db, paths, root_id, f, None, *folder)).collect::<DbResult<Vec<_>>>())?);
    }
    Ok(ids)
}

/// Send orphaned files of the managed store to the Recycle Bin. Returns how many went.
pub fn recycle_orphans(db: &Db, paths: &LibraryPaths, files: &[PathBuf]) -> DbResult<usize> {
    let mut n = 0;
    for f in still_orphans(db, paths, files)? {
        match filesystem::to_recycle_bin(&f) {
            Ok(()) => n += 1,
            Err(e) => crate::core::system::logging::warn(&format!("{}: {e}", f.display())),
        }
    }
    Ok(n)
}
