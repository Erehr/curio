//! Identical files, by size and then hash.

use crate::core::duplicates::Store;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use crate::core::db::{Db, DbResult};
use crate::core::model::AssetId;
use crate::core::duplicates::write_back;
use crate::core::duplicates::HASH_THREADS;
use crate::core::duplicates::in_parallel;
use crate::core::duplicates::abs_paths;
use crate::core::duplicates::Progress;
use crate::core::duplicates::Problems;
use crate::core::duplicates::Graph;
use crate::core::duplicates::scope::Universe;
use crate::core::duplicates::Search;

/// Files of one size, split by their quick hash where every one has it, then by their full hash, worked out where missing. Answers how many files were compared.
#[allow(clippy::too_many_arguments)]
pub(super) fn identical(db: &Db, ctx: &dyn Store, s: &Search, u: &Universe, graph: &mut Graph, problems: &mut Problems, stop: &AtomicBool, progress: &Progress) -> DbResult<usize> {
    progress(0.0, 0.0, "Finding files of the same size");
    let filter = u.filter();
    let mine = u.mine();
    let sql = format!(
        "WITH shared AS (SELECT a.size FROM assets a WHERE {filter} AND a.status = 0 AND a.size > 0 AND a.size >= ?1 GROUP BY a.size HAVING count(*) > 1 AND max({mine}) = 1)
         SELECT a.id, a.size, a.mtime, a.quick_hash, a.content_hash, {mine} FROM assets a JOIN shared s ON s.size = a.size WHERE {filter} AND a.status = 0"
    );
    struct Row {
        id: AssetId,
        size: i64,
        mtime: Option<i64>,
        quick: Option<String>,
        hash: Option<String>,
        mine: bool,
    }
    let mut st = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows: Vec<Row> = st
        .query_map([s.min_size], |r| Ok(Row { id: r.get(0)?, size: r.get(1)?, mtime: r.get(2)?, quick: r.get(3)?, hash: r.get(4)?, mine: r.get::<_, i64>(5)? != 0 }))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    let mut by_size: HashMap<i64, Vec<usize>> = HashMap::new();
    for (i, r) in rows.iter().enumerate() {
        by_size.entry(r.size).or_default().push(i);
    }
    // a size's files split by quick hash when all of them have one: different first or last bytes can't be the same file
    let mut sets: Vec<Vec<usize>> = Vec::new();
    for (_, members) in by_size {
        if members.iter().all(|i| rows[*i].quick.is_some()) {
            let mut by_quick: HashMap<&str, Vec<usize>> = HashMap::new();
            for i in members {
                by_quick.entry(rows[i].quick.as_deref().unwrap_or_default()).or_default().push(i);
            }
            sets.extend(by_quick.into_values());
        } else {
            sets.push(members);
        }
    }
    sets.retain(|m| m.len() > 1 && m.iter().any(|i| rows[*i].mine));
    let to_hash: Vec<usize> = sets.iter().flatten().copied().filter(|i| rows[*i].hash.is_none()).collect();
    let compared = sets.iter().map(|m| m.len()).sum();
    let mut hashes: HashMap<usize, String> = HashMap::new();
    if !to_hash.is_empty() {
        let paths = abs_paths(db, &to_hash.iter().map(|i| rows[*i].id).collect::<Vec<_>>());
        let total: u64 = to_hash.iter().map(|i| rows[*i].size as u64).sum();
        let report = |done: u64| progress(done as f64, total as f64, &format!("Reading {} files that share a size", to_hash.len()));
        report(0);
        let got = in_parallel(&to_hash, HASH_THREADS, stop, |i| rows[*i].size as u64, &report, |i| match paths.get(&rows[*i].id) {
            Some(p) => crate::core::system::filesystem::content_hash(p).map_err(|e| e.to_string()),
            None => Err("Its file isn't known".into()),
        });
        let mut keep = Vec::new();
        for (k, r) in got {
            let row = &rows[to_hash[k]];
            match r {
                Ok(h) => {
                    keep.push((row.id, h.clone(), row.size, row.mtime));
                    hashes.insert(to_hash[k], h);
                }
                Err(e) => problems.add(row.id, e),
            }
        }
        write_back(ctx, keep, |db, (id, h, size, mtime)| db.set_content_hash_if(*id, h, *size, *mtime));
    }
    for set in sets {
        let mut by_hash: HashMap<&str, Vec<AssetId>> = HashMap::new();
        for i in &set {
            if let Some(h) = rows[*i].hash.as_deref().or(hashes.get(i).map(|s| s.as_str())) {
                by_hash.entry(h).or_default().push(rows[*i].id);
            }
        }
        for (_, ids) in by_hash {
            if ids.len() > 1 {
                graph.add_class(ids);
            }
        }
    }
    Ok(compared)
}
