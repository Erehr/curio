//! Videos and sounds alike, by their fingerprints.

use crate::core::duplicates::fingerprints;
use crate::core::duplicates::Store;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::core::db::{Db, DbResult};
use crate::core::model::AssetId;
use crate::core::duplicates::fingerprints::Frame;
use crate::core::duplicates::write_back;
use crate::core::duplicates::FFMPEG_THREADS;
use crate::core::duplicates::in_parallel;
use crate::core::duplicates::abs_paths;
use crate::core::duplicates::Progress;
use crate::core::duplicates::Problems;
use crate::core::duplicates::Found;
use crate::core::duplicates::Graph;
use crate::core::duplicates::Way;
use crate::core::duplicates::scope::Universe;

struct MediaNode {
    id: AssetId,
    length: f64,
    print: MediaPrint,
}

enum MediaPrint {
    Frames(Vec<Frame>),
    Sound(Vec<u32>),
}

/// Videos by their frames, or sounds by their prints, made with FFmpeg where missing; candidates are those of about the same length. Answers how many were compared.
#[allow(clippy::too_many_arguments)]
pub(super) fn media(db: &Db, ctx: &dyn Store, u: &Universe, min_size: i64, way: Way, kind: &str, threshold: f64, graph: &mut Graph, found: &mut Found, problems: &mut Problems, stop: &AtomicBool, progress: &Progress) -> DbResult<usize> {
    let pk = if way == Way::Video { fingerprints::VIDEO } else { fingerprints::SOUND };
    let lengths_match = if way == Way::Video { fingerprints::video_lengths_match } else { fingerprints::sound_lengths_match };
    let sql = format!(
        "SELECT a.id, COALESCE(a.duration, 0), a.mtime, a.size, f.ver, f.src, f.size, f.data, {} FROM assets a LEFT JOIN fingerprints f ON f.asset_id = a.id AND f.kind = {pk} WHERE {} AND a.status = 0 AND a.size >= ? AND a.kind = '{kind}'",
        u.mine(),
        u.filter()
    );
    struct Row {
        id: AssetId,
        length: f64,
        mtime: i64,
        size: i64,
        data: Option<Vec<u8>>,
        mine: bool,
    }
    let mut st = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows: Vec<Row> = st
        .query_map([min_size], |r| {
            let (mtime, size): (Option<i64>, i64) = (r.get(2)?, r.get(3)?);
            let current = r.get::<_, Option<i64>>(4)? == Some(fingerprints::version(pk)) && r.get::<_, Option<i64>>(5)? == mtime && r.get::<_, Option<i64>>(6)? == Some(size);
            Ok(Row { id: r.get(0)?, length: r.get(1)?, mtime: mtime.unwrap_or(0), size, data: if current { r.get(7)? } else { None }, mine: r.get::<_, i64>(8)? != 0 })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    let mut rows: Vec<Row> = rows.into_iter().filter(|r| r.length > 0.0).collect();
    rows.sort_by(|a, b| a.length.total_cmp(&b.length));
    // a scope compared with the library reads only the library's files of about the length of one of the scope's
    if u.scoped && u.with_library {
        let mine: Vec<f64> = rows.iter().filter(|r| r.mine).map(|r| r.length).collect();
        rows.retain(|r| r.mine || mine.iter().any(|m| lengths_match(*m, r.length)));
    }
    let missing: Vec<usize> = (0..rows.len()).filter(|i| rows[*i].data.is_none()).collect();
    let mut made: HashMap<usize, Vec<u8>> = HashMap::new();
    if !missing.is_empty() {
        if crate::core::media::ffmpeg::ffmpeg().is_none() {
            found.no_ffmpeg += missing.iter().filter(|i| rows[**i].mine).count();
        } else {
            let paths = abs_paths(db, &missing.iter().map(|i| rows[*i].id).collect::<Vec<_>>());
            let total = missing.len() as u64;
            let what = if way == Way::Video { "videos" } else { "sounds" };
            let report = |done: u64| progress(done as f64, total as f64, &format!("Reading {total} {what}"));
            report(0);
            let got = in_parallel(&missing, FFMPEG_THREADS, stop, |_| 1, &report, |i| {
                let r = &rows[*i];
                let Some(p) = paths.get(&r.id) else { return Err("Its file isn't known".to_string()) };
                if way == Way::Video {
                    fingerprints::video_frames(p, r.length).map(|f| fingerprints::frames_data(&f))
                } else {
                    fingerprints::sound_of_file(p).map(|s| fingerprints::sound_data(&s))
                }
            });
            let mut keep = Vec::new();
            for (k, r) in got {
                let row = &rows[missing[k]];
                match r {
                    Ok(data) => {
                        keep.push((row.id, pk, row.mtime, row.size, (row.length * 1000.0) as i64, data.clone()));
                        made.insert(missing[k], data);
                    }
                    Err(e) => problems.add(row.id, e),
                }
            }
            write_back(ctx, keep, |db, (id, kind, mtime, size, key, data)| db.set_print(*id, *kind, *mtime, *size, *key, data));
        }
    }
    let nodes: Vec<MediaNode> = rows
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let data = r.data.as_ref().or(made.get(&i))?;
            let print = if way == Way::Video { MediaPrint::Frames(fingerprints::frames_from(data)) } else { MediaPrint::Sound(fingerprints::sound_from(data)) };
            Some(MediaNode { id: r.id, length: r.length, print })
        })
        .collect();
    if stop.load(Ordering::Relaxed) {
        return Ok(nodes.len());
    }
    progress(0.0, 0.0, &format!("Comparing {} {}", nodes.len(), if way == Way::Video { "videos" } else { "sounds" }));
    let max_bits = fingerprints::max_bits(threshold);
    for i in 0..nodes.len() {
        for j in i + 1..nodes.len() {
            if !lengths_match(nodes[i].length, nodes[j].length) {
                break;
            }
            let score = match (&nodes[i].print, &nodes[j].print) {
                (MediaPrint::Frames(a), MediaPrint::Frames(b)) => fingerprints::video_similarity(a, b, max_bits),
                (MediaPrint::Sound(a), MediaPrint::Sound(b)) => fingerprints::sound_similarity(a, b).filter(|s| *s >= threshold),
                _ => None,
            };
            if let Some(sc) = score {
                graph.add_link(nodes[i].id, nodes[j].id, way, sc);
            }
        }
    }
    Ok(nodes.len())
}
