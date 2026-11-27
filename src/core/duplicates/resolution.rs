//! Groups settled: marked not duplicates, or one item kept and the others put away.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use rusqlite::{params, OptionalExtension};
use crate::core::db::{Db, DbResult};
use crate::core::library::{changes::Changed, changes::Parts};
use crate::core::model::AssetId;
use crate::core::duplicates::pair;

/// Marks every pair among `ids` as not duplicates; answers how many pairs were new.
pub fn ignore(db: &Db, ids: &[AssetId]) -> DbResult<usize> {
    let mut st = db.conn.prepare_cached("INSERT OR IGNORE INTO dup_ignored(a, b) VALUES (?, ?)").map_err(|e| e.to_string())?;
    let mut n = 0;
    for x in 0..ids.len() {
        for y in x + 1..ids.len() {
            if ids[x] != ids[y] {
                let (a, b) = pair(ids[x], ids[y]);
                n += st.execute(params![a, b]).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(n)
}

/// Takes back pairs marked as not duplicates: those with any of `ids`, or all; answers how many.
pub fn unignore(db: &Db, ids: Option<&[AssetId]>) -> DbResult<usize> {
    match ids {
        None => db.conn.execute("DELETE FROM dup_ignored", []).map_err(|e| e.to_string()),
        Some(ids) => {
            let mut st = db.conn.prepare_cached("DELETE FROM dup_ignored WHERE a = ?1 OR b = ?1").map_err(|e| e.to_string())?;
            let mut n = 0;
            for id in ids {
                n += st.execute([id]).map_err(|e| e.to_string())?;
            }
            Ok(n)
        }
    }
}

/// One group to settle: the item kept and the copies removed.
#[derive(Clone, Debug)]
pub struct Settle {
    pub keep: AssetId,
    pub remove: Vec<AssetId>,
}

/// Why an item of a settled group was left as it was.
pub type Skipped = Vec<(AssetId, String)>;

/// Checks a group against the files as they are now, before anything is written: the kept item exists, isn't in the trash and has its file; the removed ones exist; those said to be identical still hold the kept file's bytes.
pub fn check(db: &Db, keep: AssetId, remove: &[AssetId], identical: &[AssetId]) -> DbResult<(Option<Settle>, Skipped)> {
    let mut skipped = Vec::new();
    let all: Vec<AssetId> = std::iter::once(keep).chain(remove.iter().copied()).collect();
    let rows: HashMap<AssetId, crate::core::model::Asset> = db.assets(&all)?.into_iter().map(|a| (a.id, a)).collect();
    let Some(k) = rows.get(&keep) else { return Ok((None, vec![(keep, "It's no longer in the library".into())])) };
    if k.deleted || k.part_of.is_some() {
        return Ok((None, vec![(keep, "It's in the trash".into())]));
    }
    if k.status != crate::core::model::Availability::Ok {
        return Ok((None, vec![(keep, "Its file is missing or offline".into())]));
    }
    let mut out = Vec::new();
    for id in remove {
        if *id == keep || out.contains(id) {
            continue;
        }
        match rows.get(id) {
            None => skipped.push((*id, "It's no longer in the library".into())),
            Some(a) if a.deleted => skipped.push((*id, "It's in the trash already".into())),
            Some(a) if a.part_of.is_some() => skipped.push((*id, "It's a file of another item now".into())),
            Some(a) if identical.contains(id) && (!as_read(k) || !as_read(a) || !same_bytes(Path::new(&k.abs_path), Path::new(&a.abs_path)).unwrap_or(false)) => skipped.push((*id, "It changed since the search".into())),
            Some(_) => out.push(*id),
        }
    }
    Ok(((!out.is_empty()).then_some(Settle { keep, remove: out }), skipped))
}

/// Whether a file is as the library last read it, by its size and time: what tells a large file changed where `same_bytes` doesn't look.
fn as_read(a: &crate::core::model::Asset) -> bool {
    std::fs::metadata(&a.abs_path).is_ok_and(|md| md.len() as i64 == a.size && (a.mtime.is_none() || crate::core::system::filesystem::listed(&md).mtime == a.mtime))
}

/// Files compared in full up to this size; larger ones at their start, middle and end.
const COMPARED_WHOLE: u64 = 8 << 20;

const COMPARED_PART: u64 = 1 << 20;

/// Whether two files still hold the same bytes: all of them for files up to 8 MiB, a mebibyte at the start, the middle and the end of larger ones.
fn same_bytes(a: &Path, b: &Path) -> std::io::Result<bool> {
    use std::io::{Read, Seek, SeekFrom};
    let (mut fa, mut fb) = (std::fs::File::open(a)?, std::fs::File::open(b)?);
    let len = fa.metadata()?.len();
    if fb.metadata()?.len() != len {
        return Ok(false);
    }
    let spans: Vec<(u64, u64)> = if len <= COMPARED_WHOLE { vec![(0, len)] } else { vec![(0, COMPARED_PART), (len / 2 - COMPARED_PART / 2, COMPARED_PART), (len - COMPARED_PART, COMPARED_PART)] };
    let (mut xa, mut xb) = (vec![0u8; 1 << 16], vec![0u8; 1 << 16]);
    for (start, n) in spans {
        fa.seek(SeekFrom::Start(start))?;
        fb.seek(SeekFrom::Start(start))?;
        let mut left = n;
        while left > 0 {
            let k = left.min(xa.len() as u64) as usize;
            fa.read_exact(&mut xa[..k])?;
            fb.read_exact(&mut xb[..k])?;
            if xa[..k] != xb[..k] {
                return Ok(false);
            }
            left -= k as u64;
        }
    }
    Ok(true)
}

/// Settles groups in the writer: with `combine`, each kept item takes its copies' tags, folders, best rating, notes, source address and user fields it hasn't any of; then the copies go to the trash. Answers how many were moved to the trash.
pub fn settle(db: &Db, groups: &[Settle], combine: bool) -> DbResult<(usize, Changed)> {
    let before = db.list_sizes()?;
    let mut kept = Vec::new();
    let mut gone = Vec::new();
    for g in groups {
        // checked again here, where nothing else can change them in between
        let alive: HashSet<AssetId> = db.conn.prepare_cached("SELECT id FROM assets WHERE id = ? AND deleted = 0 AND part_of IS NULL").map_err(|e| e.to_string()).and_then(|mut st| {
            let mut s = HashSet::new();
            for id in std::iter::once(&g.keep).chain(g.remove.iter()) {
                if st.query_row([id], |r| r.get::<_, AssetId>(0)).optional().map_err(|e| e.to_string())?.is_some() {
                    s.insert(*id);
                }
            }
            Ok(s)
        })?;
        if !alive.contains(&g.keep) {
            continue;
        }
        let remove: Vec<AssetId> = g.remove.iter().copied().filter(|i| alive.contains(i) && *i != g.keep).collect();
        if remove.is_empty() {
            continue;
        }
        if combine {
            combine_into(db, g.keep, &remove)?;
        }
        db.set_deleted(&remove, true)?;
        kept.push(g.keep);
        gone.extend(remove);
    }
    let n = gone.len();
    Ok((n, Changed::items(&kept).and(Changed::removed(&gone, false)).with(crate::core::library::edits::new_lists(db, before)? | Parts::TAGS | Parts::FOLDERS)))
}

/// What the removed copies had that the kept item takes: no value of its own is replaced.
fn combine_into(db: &Db, keep: AssetId, from: &[AssetId]) -> DbResult<()> {
    let k = db.asset(keep)?.ok_or("The kept item is gone")?;
    let mut tags: Vec<String> = Vec::new();
    let mut folders: Vec<i64> = Vec::new();
    let mut rating = k.rating;
    let mut notes = k.annotation.clone();
    let mut url = k.source_url.clone();
    let mine: HashSet<i64> = db.asset_folders(keep)?.into_iter().map(|f| f.id).collect();
    for id in from {
        let Some(a) = db.asset(*id)? else { continue };
        tags.extend(db.asset_tags(*id)?.into_iter().map(|t| t.name));
        folders.extend(db.asset_folders(*id)?.into_iter().map(|f| f.id).filter(|f| !mine.contains(f)));
        rating = rating.max(a.rating);
        let note = a.annotation.trim();
        if !note.is_empty() && !notes.contains(note) {
            notes = if notes.trim().is_empty() { note.to_string() } else { format!("{}\n\n{note}", notes.trim_end()) };
        }
        if url.is_empty() && !a.source_url.is_empty() {
            url = a.source_url.clone();
        }
        db.fill_user_fields(keep, *id)?;
    }
    tags.sort();
    tags.dedup();
    if !tags.is_empty() {
        db.add_tags(&[keep], &tags)?;
    }
    folders.sort_unstable();
    folders.dedup();
    for f in folders {
        db.add_to_folder(&[keep], f)?;
    }
    if rating != k.rating {
        db.set_rating(&[keep], rating)?;
    }
    if notes != k.annotation {
        db.set_annotation(&[keep], &notes)?;
    }
    if url != k.source_url {
        db.set_source_url(&[keep], &url)?;
    }
    db.refresh_fts(&[keep])
}
