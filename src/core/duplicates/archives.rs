//! Archives alike, by their entries.

use crate::core::duplicates::fingerprints;
use crate::core::duplicates::Store;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::core::db::{Db, DbResult};
use crate::core::model::AssetId;
use crate::core::duplicates::Way;
use crate::core::duplicates::COMMON_ENTRY;
use crate::core::duplicates::write_back;
use crate::core::duplicates::HASH_THREADS;
use crate::core::duplicates::in_parallel;
use crate::core::duplicates::abs_paths;
use crate::core::duplicates::Progress;
use crate::core::duplicates::Problems;
use crate::core::duplicates::Graph;
use crate::core::duplicates::scope::Universe;

/// Archives by their entries, read from their directories where missing: the same contents, or the share of entries in common. Answers how many were compared.
#[allow(clippy::too_many_arguments)]
pub(super) fn archives(db: &Db, ctx: &dyn Store, u: &Universe, min_size: i64, threshold: f64, graph: &mut Graph, problems: &mut Problems, stop: &AtomicBool, progress: &Progress) -> DbResult<usize> {
    let exts = crate::core::archive::EXTS.iter().map(|e| format!("'{e}'")).collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT a.id, a.mtime, a.size, f.ver, f.src, f.size, f.data, {} FROM assets a LEFT JOIN fingerprints f ON f.asset_id = a.id AND f.kind = {} WHERE {} AND a.status = 0 AND a.size >= ? AND a.ext IN ({exts})",
        u.mine(),
        fingerprints::ARCHIVE,
        u.filter()
    );
    struct Row {
        id: AssetId,
        mtime: i64,
        size: i64,
        data: Option<Vec<u8>>,
        mine: bool,
    }
    let mut st = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows: Vec<Row> = st
        .query_map([min_size], |r| {
            let (mtime, size): (Option<i64>, i64) = (r.get(1)?, r.get(2)?);
            let current = r.get::<_, Option<i64>>(3)? == Some(fingerprints::version(fingerprints::ARCHIVE)) && r.get::<_, Option<i64>>(4)? == mtime && r.get::<_, Option<i64>>(5)? == Some(size);
            Ok(Row { id: r.get(0)?, mtime: mtime.unwrap_or(0), size, data: if current { r.get(6)? } else { None }, mine: r.get::<_, i64>(7)? != 0 })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    let missing: Vec<usize> = (0..rows.len()).filter(|i| rows[*i].data.is_none()).collect();
    let mut made: HashMap<usize, Vec<(u32, u64)>> = HashMap::new();
    if !missing.is_empty() {
        let paths = abs_paths(db, &missing.iter().map(|i| rows[*i].id).collect::<Vec<_>>());
        let total = missing.len() as u64;
        let report = |done: u64| progress(done as f64, total as f64, &format!("Reading {total} archives"));
        let got = in_parallel(&missing, HASH_THREADS, stop, |_| 1, &report, |i| match paths.get(&rows[*i].id) {
            Some(p) => fingerprints::archive_entries(p),
            None => Err("Its file isn't known".into()),
        });
        let mut keep = Vec::new();
        for (k, r) in got {
            let row = &rows[missing[k]];
            match r {
                Ok(entries) => {
                    keep.push((row.id, row.mtime, row.size, fingerprints::entries_key(&entries), fingerprints::entries_data(&entries)));
                    made.insert(missing[k], entries);
                }
                Err(e) => problems.add(row.id, e),
            }
        }
        write_back(ctx, keep, |db, (id, mtime, size, key, data)| db.set_print(*id, fingerprints::ARCHIVE, *mtime, *size, *key, data));
    }
    let nodes: Vec<(AssetId, bool, Vec<(u32, u64)>)> = rows.iter().enumerate().filter_map(|(i, r)| Some((r.id, r.mine, r.data.as_ref().map(|d| fingerprints::entries_from(d)).or_else(|| made.remove(&i))?))).collect();
    if stop.load(Ordering::Relaxed) {
        return Ok(nodes.len());
    }
    progress(0.0, 0.0, &format!("Comparing {} archives", nodes.len()));
    // which archives hold each entry (with multiplicity, so archives with repeated entries prune accurately)
    let mut holders: HashMap<(u32, u64), Vec<(u32, u32)>> = HashMap::new();
    for (k, n) in nodes.iter().enumerate() {
        let mut i = 0;
        while i < n.2.len() {
            let e = n.2[i];
            let start = i;
            while i < n.2.len() && n.2[i] == e {
                i += 1;
            }
            holders.entry(e).or_default().push((k as u32, (i - start) as u32));
        }
    }
    let mut shared: HashMap<(u32, u32), u32> = HashMap::new();
    for list in holders.values().filter(|l| l.len() > 1 && l.len() <= COMMON_ENTRY) {
        for x in 0..list.len() {
            for y in x + 1..list.len() {
                *shared.entry((list[x].0, list[y].0)).or_default() += list[x].1.min(list[y].1);
            }
        }
    }
    for ((a, b), common) in shared {
        let (x, y) = (&nodes[a as usize], &nodes[b as usize]);
        // |A ∩ B| / |A ∪ B| can reach the threshold only with this many entries in common
        if (common as f64) < threshold * (x.2.len() + y.2.len()) as f64 / (1.0 + threshold) - 1.0 {
            continue;
        }
        let sim = fingerprints::archive_similarity(&x.2, &y.2);
        if sim >= threshold {
            graph.add_link(x.0, y.0, Way::Archive, sim);
        }
    }
    Ok(nodes.len())
}
