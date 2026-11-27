//! Pictures that look alike, by their fingerprints.

use crate::core::duplicates::fingerprints;
use crate::core::duplicates::Store;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::core::db::{Db, DbResult};
use crate::core::model::AssetId;
use crate::core::duplicates::fingerprints::Look;
use crate::core::duplicates::PLAIN_BUCKET;
use crate::core::duplicates::Way;
use crate::core::duplicates::write_back;
use crate::core::duplicates::in_parallel;
use crate::core::duplicates::Progress;
use crate::core::duplicates::Found;
use crate::core::duplicates::Graph;
use crate::core::duplicates::scope::Universe;
use crate::core::duplicates::Search;

struct LookNode {
    id: AssetId,
    aspect: f32,
    look: Look,
    mine: bool,
}

/// Thumbnails' looks, those older than their thumbnails made again from the thumbnail files; pairs found by multi-index hashing over the DCT hash's four 16-bit parts, then confirmed by the rest of the look. Answers how many looks were compared.
#[allow(clippy::too_many_arguments)]
pub(super) fn looks(db: &Db, ctx: &dyn Store, s: &Search, u: &Universe, threshold: f64, graph: &mut Graph, found: &mut Found, stop: &AtomicBool, progress: &Progress) -> DbResult<usize> {
    progress(0.0, 0.0, "Reading thumbnails' looks");
    if s.look_kinds.is_empty() {
        return Ok(0);
    }
    let kinds = s.look_kinds.iter().map(|k| format!("'{}'", k.as_str())).collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT a.id, a.aspect, a.thumb_state, a.thumb_ver, f.ver, f.src, f.key, f.data, {}, CASE WHEN f.src IS a.thumb_ver AND f.ver = {} THEN NULL ELSE uid_text(a.uid) END
         FROM assets a LEFT JOIN fingerprints f ON f.asset_id = a.id AND f.kind = {} WHERE {} AND a.kind IN ({kinds}) AND a.size >= ?",
        u.mine(),
        fingerprints::version(fingerprints::LOOK),
        fingerprints::LOOK,
        u.filter()
    );
    let mut nodes: Vec<LookNode> = Vec::new();
    let mut stale: Vec<(AssetId, f32, i64, String, bool)> = Vec::new();
    {
        let mut st = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut rows = st.query([s.min_size]).map_err(|e| e.to_string())?;
        while let Some(r) = rows.next().map_err(|e| e.to_string())? {
            let (id, aspect, state, ver): (AssetId, f64, i64, i64) = (r.get(0).map_err(|e| e.to_string())?, r.get(1).map_err(|e| e.to_string())?, r.get(2).map_err(|e| e.to_string())?, r.get(3).map_err(|e| e.to_string())?);
            let mine = r.get::<_, i64>(8).map_err(|e| e.to_string())? != 0;
            if state != 1 {
                // a thumbnail that failed for good is no thumbnail to wait for
                found.no_thumbnail += (mine && state != 2) as usize;
                continue;
            }
            match r.get::<_, Option<String>>(9).map_err(|e| e.to_string())? {
                None => {
                    let key: i64 = r.get(6).map_err(|e| e.to_string())?;
                    let data: Vec<u8> = r.get(7).map_err(|e| e.to_string())?;
                    if let Some(look) = Look::from_row(key, &data) {
                        nodes.push(LookNode { id, aspect: aspect as f32, look, mine });
                    }
                }
                Some(uid) => stale.push((id, aspect as f32, ver, uid, mine)),
            }
        }
    }
    if !stale.is_empty() {
        let total = stale.len() as u64;
        let report = |done: u64| progress(done as f64, total as f64, &format!("Reading {total} thumbnails"));
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8);
        let got = in_parallel(&stale, threads, stop, |_| 1, &report, |(_, _, ver, uid, _)| fingerprints::look_of_file(&crate::core::thumbs::thumb_path(ctx.thumbs(), uid, *ver)));
        let mut keep = Vec::new();
        for (k, r) in got {
            let (id, aspect, ver, _, mine) = &stale[k];
            match r {
                Ok(look) => {
                    keep.push((*id, look, *ver));
                    nodes.push(LookNode { id: *id, aspect: *aspect, look, mine: *mine });
                }
                // a thumbnail file gone is made again by the thumbnailer, not counted as the file's fault
                Err(_) => found.no_thumbnail += *mine as usize,
            }
        }
        write_back(ctx, keep, |db, (id, look, ver)| db.set_look_at(*id, look, *ver));
    }
    if stop.load(Ordering::Relaxed) {
        return Ok(nodes.len());
    }
    progress(0.0, 0.0, &format!("Comparing {} pictures", nodes.len()));
    let max_bits = fingerprints::max_bits(threshold);
    let pairs = near_pairs(&nodes, max_bits, u.scoped && u.with_library, &mut found.plain, stop);
    for (a, b) in pairs {
        let (x, y) = (&nodes[a], &nodes[b]);
        if fingerprints::looks_match(&x.look, &y.look, (x.aspect, y.aspect), max_bits) {
            graph.add_link(x.id, y.id, Way::Looks, x.look.similarity(&y.look));
        }
    }
    Ok(nodes.len())
}

/// Every pair of nodes whose DCT hashes are within `max_bits`, found through four tables of the hashes' 16-bit parts: two hashes within d bits share a part within d/4 bits. With `around_mine`, only pairs among the scope's pictures and their neighbours.
fn near_pairs(nodes: &[LookNode], max_bits: u32, around_mine: bool, plain: &mut usize, stop: &AtomicBool) -> Vec<(usize, usize)> {
    // pictures with one hash are one entry, so a thousand copies of a page are one lookup
    let mut distinct: Vec<u64> = Vec::new();
    let mut members: Vec<Vec<usize>> = Vec::new();
    let mut at: HashMap<u64, usize> = HashMap::new();
    for (i, n) in nodes.iter().enumerate() {
        let k = *at.entry(n.look.dct).or_insert_with(|| {
            distinct.push(n.look.dct);
            members.push(Vec::new());
            distinct.len() - 1
        });
        members[k].push(i);
    }
    for m in members.iter_mut().filter(|m| m.len() > PLAIN_BUCKET) {
        *plain += m.len();
        m.clear();
    }
    let tables: Vec<PartTable> = (0..4).map(|p| PartTable::new(&distinct, &members, p)).collect();
    let radius = max_bits / 4;
    let masks: Vec<u16> = (0u32..1 << 16).filter(|m| m.count_ones() <= radius).map(|m| m as u16).collect();
    // hashes after `from` only, each looked at once however many of its parts are near
    let neighbours = |k: usize, from: usize, seen: &mut Vec<u32>, stamp: u32, out: &mut Vec<usize>| {
        let v = distinct[k];
        for (p, t) in tables.iter().enumerate() {
            let part = (v >> (16 * p)) as u16;
            for m in &masks {
                for &j in t.holding(part ^ m) {
                    let j = j as usize;
                    if j < from || seen[j] == stamp {
                        continue;
                    }
                    seen[j] = stamp;
                    if fingerprints::bits_apart(v, distinct[j]) <= max_bits {
                        out.push(j);
                    }
                }
            }
        }
    };
    // which distinct hashes to ask about: all of them, or the scope's and those near them
    let asked: Vec<usize> = if around_mine {
        let mut seen = vec![0u32; distinct.len()];
        let mut set: HashSet<usize> = HashSet::new();
        let mut out = Vec::new();
        for (k, m) in members.iter().enumerate() {
            if m.iter().any(|i| nodes[*i].mine) {
                out.clear();
                neighbours(k, 0, &mut seen, k as u32 + 1, &mut out);
                set.extend(out.iter().copied());
            }
        }
        let mut v: Vec<usize> = set.into_iter().collect();
        v.sort_unstable();
        v
    } else {
        (0..distinct.len()).filter(|k| !members[*k].is_empty()).collect()
    };
    let within: Option<HashSet<usize>> = around_mine.then(|| asked.iter().copied().collect());
    // the tables are only read, so the lookups share them across threads, each with its own marks
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 8);
    let per = asked.len().div_ceil(threads).max(1);
    let pairs_of = |part: &[usize]| {
        let mut seen = vec![0u32; distinct.len()];
        let mut out = Vec::new();
        let mut pairs = Vec::new();
        for (n, &k) in part.iter().enumerate() {
            if n % 4096 == 0 && stop.load(Ordering::Relaxed) {
                break;
            }
            out.clear();
            neighbours(k, k, &mut seen, n as u32 + 1, &mut out);
            for &j in &out {
                if within.as_ref().is_some_and(|w| !w.contains(&j)) {
                    continue;
                }
                let (a, b) = (&members[k], &members[j]);
                if j == k {
                    for x in 0..a.len() {
                        for y in x + 1..a.len() {
                            pairs.push((a[x], a[y]));
                        }
                    }
                } else {
                    for &x in a {
                        for &y in b {
                            pairs.push((x, y));
                        }
                    }
                }
            }
        }
        pairs
    };
    std::thread::scope(|sc| {
        let running: Vec<_> = asked.chunks(per).map(|part| sc.spawn(|| pairs_of(part))).collect();
        running.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    })
}

/// The distinct hashes by one of their 16-bit parts, as one flat list: those whose part is `v` are `at[start[v]..start[v + 1]]`, so a lookup is two reads instead of a hash map's work, hundreds of millions of times at a loose threshold.
struct PartTable {
    start: Vec<u32>,
    at: Vec<u32>,
}

impl PartTable {
    fn new(distinct: &[u64], members: &[Vec<usize>], part: usize) -> PartTable {
        let value = |v: u64| ((v >> (16 * part)) & 0xFFFF) as usize;
        let mut start = vec![0u32; (1 << 16) + 1];
        for (_, v) in distinct.iter().enumerate().filter(|(k, _)| !members[*k].is_empty()) {
            start[value(*v) + 1] += 1;
        }
        for i in 1..start.len() {
            start[i] += start[i - 1];
        }
        let mut next = start.clone();
        let mut at = vec![0u32; start[1 << 16] as usize];
        for (k, v) in distinct.iter().enumerate().filter(|(k, _)| !members[*k].is_empty()) {
            let b = value(*v);
            at[next[b] as usize] = k as u32;
            next[b] += 1;
        }
        PartTable { start, at }
    }

    fn holding(&self, part: u16) -> &[u32] {
        let b = part as usize;
        &self.at[self.start[b] as usize..self.start[b + 1] as usize]
    }
}
