//! The managed store's layout and the journaled file work behind it.
//!
//! Each library folder is a directory and each item has one placement (file entry) per folder it is in, hard links to one set of data where the filesystem allows. An item indexed in place has its file in a location; its placements are links to that file (copies when the location is on another drive), one per folder and none outside them.
//! Folder memberships and the trash flag are the truth: triggers queue changed items in `store_dirty`, and `sync` moves their files to match.
//! Every operation records its intent in `file_ops` before touching the disk and finishes the database in one transaction afterwards, so `recover` can complete or drop it after a crash.
//! They must run outside any transaction (the intent has to be committed first), which in the app means on the scanner thread.

pub mod disk;
pub mod journal;
pub mod orphans;
pub mod outside;
pub mod placing;
pub mod recovery;
pub mod sync;
pub mod transfer;

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::db::{fold_path, Db, DbResult};
use crate::core::system::filesystem;
use crate::core::import::managed_root;
use crate::core::library::paths::LibraryPaths;
use crate::core::system::logging;
use crate::core::model::{AssetId, Placement};
use crate::core::store::disk::link_or_copy;
use crate::core::store::disk::rename_no_clobber;
use crate::core::store::placing::move_dir;
use crate::core::store::disk::exists;

/// Hidden folder of the store that holds trashed items' files.
pub const TRASH_DIR: &str = ".trash";

/// Longest file or folder name written, in UTF-8 bytes: ext4 allows 255, and a clash suffix needs room.
const MAX_NAME_BYTES: usize = 200;

const RESERVED: &[&str] = &["con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9"];

/// How a second file of the same name in one folder is told apart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuffixStyle {
    /// `name (2).ext`
    #[default]
    Paren,
    /// `name #2.ext`
    Hash,
    /// `name - 2.ext`
    Dash,
}

/// What an incoming file does when its folder already has an item of that name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnClash {
    /// Leave the existing item alone and drop the incoming file.
    Skip,
    /// The existing item keeps its metadata and folders and takes the incoming file's content.
    Replace,
    /// Both stay; the incoming file gets a numbered name.
    #[default]
    Keep,
}

impl SuffixStyle {
    pub fn apply(self, name: &str, n: usize) -> String {
        let (stem, ext) = split_ext(name);
        let tag = self.tag(n);
        match ext {
            Some(ext) => format!("{stem}{tag}.{ext}"),
            None => format!("{stem}{tag}"),
        }
    }

    /// The number alone, as it follows a name: ` (2)`, ` #2`, ` - 2`.
    pub fn tag(self, n: usize) -> String {
        match self {
            SuffixStyle::Paren => format!(" ({n})"),
            SuffixStyle::Hash => format!(" #{n}"),
            SuffixStyle::Dash => format!(" - {n}"),
        }
    }
}

/// `("photo", Some("png"))`; a leading dot alone (`.gitignore`) is not an extension.
fn split_ext(name: &str) -> (&str, Option<&str>) {
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => (&name[..i], Some(&name[i + 1..])),
        _ => (name, None),
    }
}

/// A file or folder name that every supported filesystem accepts, kept as close to `name` as possible.
pub fn clean_name(name: &str, is_dir: bool) -> String {
    let mut s: String = name.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }).collect();
    s = s.trim_end_matches([' ', '.']).to_string();
    // walkers skip hidden entries, so a folder must not start with a dot
    if is_dir && s.starts_with('.') {
        s.replace_range(..1, "_");
    }
    if s.len() > MAX_NAME_BYTES {
        // an extension too long to be one is cut with the rest of the name
        let (stem, ext) = match split_ext(&s) {
            (stem, Some(e)) if e.len() < 16 => (stem, format!(".{e}")),
            _ => (s.as_str(), String::new()),
        };
        let mut cut = MAX_NAME_BYTES - ext.len();
        while !stem.is_char_boundary(cut) {
            cut -= 1;
        }
        s = format!("{}{ext}", stem[..cut].trim_end_matches([' ', '.']));
    }
    let base = split_ext(&s).0.to_ascii_lowercase();
    if RESERVED.contains(&base.as_str()) {
        let (stem, ext) = split_ext(&s);
        s = match ext {
            Some(e) => format!("{stem}_.{e}"),
            None => format!("{stem}_"),
        };
    }
    if s.is_empty() {
        "_".into()
    } else {
        s
    }
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

fn base_of(rel: &str) -> &str {
    rel.rsplit_once('/').map(|(_, n)| n).unwrap_or(rel)
}

fn parent_of(rel: &str) -> &str {
    rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

/// Whether `base` is `clean` itself or `clean` with a clash suffix of any style.
pub fn is_variant(base: &str, clean: &str) -> bool {
    if base == clean {
        return true;
    }
    let (stem, ext) = split_ext(clean);
    let tail = ext.map(|e| format!(".{e}")).unwrap_or_default();
    let Some(mid) = base.strip_prefix(stem).and_then(|r| r.strip_suffix(tail.as_str())) else { return false };
    let num = |n: &str| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) && n.parse::<u64>().is_ok_and(|n| n >= 2);
    mid.strip_prefix(" (").and_then(|m| m.strip_suffix(')')).is_some_and(num) || mid.strip_prefix(" #").is_some_and(num) || mid.strip_prefix(" - ").is_some_and(num)
}

/// The directory of a library folder in the store (`""` for the store root) as it is on disk. A folder without one yet is given its place first.
pub fn folder_dir(db: &mut Db, paths: &LibraryPaths, folder: Option<i64>) -> DbResult<String> {
    let Some(id) = folder else { return Ok(String::new()) };
    let read = |db: &Db| db.conn.query_row("SELECT dir FROM folders WHERE id = ?", [id], |r| r.get::<_, Option<String>>(0)).map_err(|_| format!("no folder {id}"));
    if let Some(d) = read(db)? {
        return Ok(d);
    }
    for p in sync_folders(db, paths)? {
        logging::warn(&p);
    }
    read(db)?.ok_or_else(|| format!("folder {id} has no directory in the library"))
}

struct FolderRow {
    id: i64,
    parent: Option<i64>,
    name: String,
    dir: Option<String>,
}

/// The library folder whose directory is `dir`. A folder of that name without a directory yet takes it; missing ones are made, named after their directories and keeping them.
pub fn folder_for_dir(db: &Db, dir: &str) -> DbResult<i64> {
    let found: Option<i64> = db
        .conn
        .query_row("SELECT id FROM folders WHERE dir IS NOT NULL AND fold_path(dir) = ?", [fold_path(dir)], |r| r.get(0))
        .ok();
    if let Some(id) = found {
        return Ok(id);
    }
    let parent = match dir.rsplit_once('/') {
        Some((up, _)) => Some(folder_for_dir(db, up)?),
        None => None,
    };
    let unplaced: Vec<(i64, String)> = {
        let mut st = db.conn.prepare("SELECT id, name FROM folders WHERE parent_id IS ? AND dir IS NULL ORDER BY sort_order, id").map_err(|e| e.to_string())?;
        let q = st.query_map([parent], |r| Ok((r.get(0)?, r.get(1)?))).map_err(|e| e.to_string())?;
        q.filter_map(|r| r.ok()).collect()
    };
    if let Some((id, _)) = unplaced.iter().find(|(_, n)| fold_path(&clean_name(n, true)) == fold_path(base_of(dir))) {
        db.conn.execute("UPDATE folders SET dir = ? WHERE id = ?", rusqlite::params![dir, id]).map_err(|e| e.to_string())?;
        return Ok(*id);
    }
    let id = db.create_folder(base_of(dir), parent)?;
    db.conn.execute("UPDATE folders SET dir = ? WHERE id = ?", rusqlite::params![dir, id]).map_err(|e| e.to_string())?;
    Ok(id)
}

/// Where every folder's directory belongs, parents before children: under the parent's, named after the folder, numbered when a sibling or something else on disk has the name. A folder keeps its current name while that still fits.
pub fn plan_folder_dirs(db: &Db, paths: &LibraryPaths) -> DbResult<Vec<(i64, String)>> {
    let rows: Vec<FolderRow> = {
        let mut st = db.conn.prepare("SELECT id, parent_id, name, dir FROM folders ORDER BY sort_order, id").map_err(|e| e.to_string())?;
        let q = st.query_map([], |r| Ok(FolderRow { id: r.get(0)?, parent: r.get(1)?, name: r.get(2)?, dir: r.get(3)? })).map_err(|e| e.to_string())?;
        q.filter_map(|r| r.ok()).collect()
    };
    let mut kids: HashMap<Option<i64>, Vec<&FolderRow>> = HashMap::new();
    for r in &rows {
        kids.entry(r.parent).or_default().push(r);
    }
    let mut out = vec![];
    let mut queue: std::collections::VecDeque<(Option<i64>, String)> = [(None, String::new())].into();
    while let Some((parent, up)) = queue.pop_front() {
        let Some(group) = kids.get(&parent) else { continue };
        let mut taken: HashSet<String> = HashSet::new();
        if parent.is_none() {
            taken.insert(TRASH_DIR.to_string());
        }
        let mut want: HashMap<i64, String> = HashMap::new();
        for f in group {
            let Some(d) = &f.dir else { continue };
            let b = base_of(d);
            if !is_variant(b, &clean_name(&f.name, true)) || taken.contains(&fold_path(b)) {
                continue;
            }
            let w = join(&up, b);
            if fold_path(parent_of(d)) != fold_path(&up) && exists(&abs(paths, &w)) {
                continue;
            }
            taken.insert(fold_path(b));
            want.insert(f.id, w);
        }
        // directories siblings are leaving are free for the others
        let leaving: HashSet<String> = group.iter().filter(|f| !want.contains_key(&f.id)).filter_map(|f| f.dir.as_deref()).map(fold_path).collect();
        for f in group {
            if want.contains_key(&f.id) {
                continue;
            }
            let base = clean_name(&f.name, true);
            let mut cand = base.clone();
            let mut n = 1;
            loop {
                let w = join(&up, &cand);
                let mine = leaving.contains(&fold_path(&w));
                if !taken.contains(&fold_path(&cand)) && (mine || !exists(&abs(paths, &w))) {
                    break;
                }
                n += 1;
                cand = SuffixStyle::Paren.apply(&base, n);
            }
            taken.insert(fold_path(&cand));
            want.insert(f.id, join(&up, &cand));
        }
        for f in group {
            let w = want.remove(&f.id).unwrap_or_default();
            queue.push_back((Some(f.id), w.clone()));
            out.push((f.id, w));
        }
    }
    Ok(out)
}

/// Move every folder's directory to where `plan_folder_dirs` puts it, parents first, and make the ones not on disk yet. Returns the moves that failed; they are tried again on the next pass.
pub fn sync_folders(db: &mut Db, paths: &LibraryPaths) -> DbResult<Vec<String>> {
    let mut problems = vec![];
    let mut todo = plan_folder_dirs(db, paths)?;
    // a directory still held by a sibling that is moving away is tried again once that one has moved
    loop {
        let count = todo.len();
        let mut waiting = vec![];
        for (id, want) in todo {
            let cur: Option<String> = db.conn.query_row("SELECT dir FROM folders WHERE id = ?", [id], |r| r.get(0)).map_err(|e| e.to_string())?;
            match cur {
                None => {
                    db.conn.execute("UPDATE folders SET dir = ? WHERE id = ?", rusqlite::params![want, id]).map_err(|e| e.to_string())?;
                }
                Some(c) if c == want => {}
                Some(c) if exists(&abs(paths, &c)) => {
                    if fold_path(&c) != fold_path(&want) && exists(&abs(paths, &want)) {
                        waiting.push((id, want));
                        continue;
                    }
                    if let Err(e) = move_dir(db, paths, &c, &want) {
                        problems.push(e);
                        continue;
                    }
                }
                Some(c) => db.tx(|db| {
                    db.move_placement_dir(&c, &want)?;
                    db.move_folder_dirs(&c, &want)
                })?,
            }
            if let Err(e) = std::fs::create_dir_all(abs(paths, &want)) {
                problems.push(format!("making the folder {want}: {e}"));
            }
        }
        if waiting.is_empty() {
            break;
        }
        if waiting.len() == count {
            // break a sibling name swap (e.g. A <-> B) by moving one waiting directory aside to a temporary name
            let held: HashSet<String> = waiting
                .iter()
                .filter_map(|(id, _)| db.conn.query_row("SELECT dir FROM folders WHERE id = ?", [id], |r| r.get::<_, Option<String>>(0)).ok().flatten())
                .map(|d| fold_path(&d))
                .collect();
            let cycle = waiting.iter().find_map(|(id, want)| {
                held.contains(&fold_path(want)).then(|| {
                    db.conn.query_row("SELECT dir FROM folders WHERE id = ?", [id], |r| r.get::<_, Option<String>>(0)).ok().flatten()
                }).flatten()
            });
            if let Some(c) = cycle {
                let tmp = format!("{c}.swap-{}", fastrand::u32(..));
                match move_dir(db, paths, &c, &tmp) {
                    Ok(()) => {
                        todo = waiting;
                        continue;
                    }
                    Err(e) => problems.push(e),
                }
            } else {
                problems.extend(waiting.iter().map(|(_, w)| format!("moving a folder to {w}: something else is there")));
            }
            break;
        }
        todo = waiting;
    }
    drop_gone_dirs(db, paths)?;
    Ok(problems)
}

/// Remove the directories of deleted folders that are empty, deepest first. One still holding files is left to the item passes, which take it away once they have moved them out.
fn drop_gone_dirs(db: &Db, paths: &LibraryPaths) -> DbResult<()> {
    let mut dirs: Vec<String> = {
        let mut st = db.conn.prepare("SELECT dir FROM store_gone_dirs").map_err(|e| e.to_string())?;
        let q = st.query_map([], |r| r.get(0)).map_err(|e| e.to_string())?;
        q.filter_map(|r| r.ok()).collect()
    };
    dirs.sort_by_key(|d| std::cmp::Reverse(d.len()));
    for d in dirs {
        if !is_folder_dir(db, &d) {
            let _ = std::fs::remove_dir(abs(paths, &d));
        }
        db.conn.execute("DELETE FROM store_gone_dirs WHERE dir = ?", [&d]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Whether a folder of the library keeps its directory at `dir`.
fn is_folder_dir(db: &Db, dir: &str) -> bool {
    db.conn.query_row("SELECT EXISTS (SELECT 1 FROM folders WHERE dir IS NOT NULL AND fold_path(dir) = fold_path(?))", [dir], |r| r.get(0)).unwrap_or(false)
}

/// What a store path may already hold and still be free for an item: the entry being moved, and the entries of the files grouped into it, whose names (its stem and their suffixes) must be free too.
#[derive(Clone, Debug, Default)]
pub struct Own {
    pub placement: Option<i64>,
    pub members: Vec<AssetId>,
    pub suffixes: Vec<String>,
}

/// What a path may hold for `asset`, moving its entry `placement`.
pub fn own_of(db: &Db, asset: AssetId, placement: Option<i64>) -> Own {
    let parts = db.members_of(asset).unwrap_or_default();
    Own { placement, members: parts.iter().map(|p| p.id).collect(), suffixes: parts.into_iter().filter_map(|p| p.suffix).collect() }
}

/// Whether `rel` is free in the store: no placement claims it (in any case) but `own`'s, and nothing else is on disk there.
fn is_free_for(db: &Db, paths: &LibraryPaths, rel: &str, own: &Own) -> bool {
    match db.placement_at(rel) {
        Some(p) => Some(p.id) == own.placement || own.members.contains(&p.asset_id),
        None => std::fs::symlink_metadata(abs(paths, rel)).is_err(),
    }
}

/// The name of a grouped file beside its item's entry called `item_base`.
fn member_name(item_base: &str, suffix: &str) -> String {
    format!("{}{suffix}", crate::core::grouping::companions::stem(item_base))
}

/// A free store path for a file called `name` in `dir`, numbered with `style` from 2 when the name is taken, or when a name the files grouped into it would take beside it is.
pub fn free_for(db: &Db, paths: &LibraryPaths, dir: &str, name: &str, style: SuffixStyle, own: &Own) -> String {
    let base = clean_name(name, false);
    let fits = |cand: &str| is_free_for(db, paths, cand, own) && own.suffixes.iter().all(|s| is_free_for(db, paths, &join(dir, &member_name(base_of(cand), s)), own));
    let mut cand = join(dir, &base);
    let mut n = 1;
    while !fits(&cand) {
        n += 1;
        cand = join(dir, &style.apply(&base, n));
    }
    cand
}

/// A free store path for a file called `name` in `dir`, numbered with `style` from 2 when the name is taken.
pub fn free_name(db: &Db, paths: &LibraryPaths, dir: &str, name: &str, style: SuffixStyle) -> String {
    free_for(db, paths, dir, name, style, &Own::default())
}

/// Where an incoming file goes.
#[derive(Debug)]
pub enum Target {
    /// A free store path.
    New(String),
    /// The item already holding that name, for `Skip` or `Replace`.
    Existing(Placement),
}

/// Where a file called `name` coming into `folder` (`None` = store root) goes, following `clash` when the name is taken by an item listed there. A name taken by anything else on disk, such as a file still waiting to follow its item elsewhere, always gets a number.
pub fn target_for(db: &mut Db, paths: &LibraryPaths, folder: Option<i64>, name: &str, clash: OnClash, style: SuffixStyle) -> DbResult<Target> {
    let dir = folder_dir(db, paths, folder)?;
    if clash != OnClash::Keep {
        if let Some(p) = db.member_placement_at(&join(&dir, &clean_name(name, false)), folder) {
            return Ok(Target::Existing(p));
        }
    }
    Ok(Target::New(free_name(db, paths, &dir, name, style)))
}

/// The store path a file called `name` would clash with in the directory of `folder`, without creating anything: `None` when the folder has no directory yet.
pub fn clash_path(db: &Db, folder: Option<i64>, name: &str) -> Option<String> {
    let dir = match folder {
        None => String::new(),
        Some(f) => db.conn.query_row("SELECT dir FROM folders WHERE id = ?", [f], |r| r.get::<_, Option<String>>(0)).ok().flatten()?,
    };
    Some(join(&dir, &clean_name(name, false)))
}

/// Write `src` to `dest` through `<dest>.partial`, never replacing what is at `dest`.
pub fn copy_in(src: &Path, dest: &Path) -> io::Result<()> {
    if let Some(p) = dest.parent() {
        std::fs::create_dir_all(p)?;
    }
    let tmp = PathBuf::from(format!("{}.partial", dest.display()));
    let res = std::fs::copy(src, &tmp).and_then(|_| rename_no_clobber(&tmp, dest));
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

/// Move `src` to `dest`, never replacing what is at `dest`: renamed when both are on one drive, else copied and verified before the original is removed.
pub fn move_in(src: &Path, dest: &Path) -> io::Result<()> {
    match rename_no_clobber(src, dest) {
        Ok(()) => return Ok(()),
        Err(e) if transfer::crosses_devices(&e) => {}
        Err(e) => return Err(e),
    }
    disk::copy_verified(src, dest)?;
    if let Err(e) = std::fs::remove_file(src) {
        crate::core::system::logging::warn(&format!("{} was copied into the library but couldn't be removed: {e}", src.display()));
    }
    Ok(())
}

/// Write `bytes` to `dest` through `<dest>.partial`, never replacing what is at `dest`.
pub fn write_in(bytes: &[u8], dest: &Path) -> io::Result<()> {
    if let Some(p) = dest.parent() {
        std::fs::create_dir_all(p)?;
    }
    let tmp = PathBuf::from(format!("{}.partial", dest.display()));
    let res = std::fs::write(&tmp, bytes).and_then(|_| rename_no_clobber(&tmp, dest));
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

/// After an item's file was rewritten under its own path (replace, turn, saved frame), make its other placements entries of the new file again: each is swapped for a fresh link (or copy) in one rename, so it is never missing. Returns how many were renewed.
pub fn relink(db: &Db, paths: &LibraryPaths, asset: AssetId) -> DbResult<usize> {
    let primary: String = db.conn.query_row("SELECT rel_path FROM assets WHERE id = ?", [asset], |r| r.get(0)).map_err(|_| format!("no item {asset}"))?;
    let main = abs(paths, &primary);
    let Some(key) = filesystem::file_key(&main) else { return Ok(0) };
    let mut n = 0;
    for p in db.placements_of(asset)?.into_iter().filter(|p| p.rel_path != primary) {
        let at = abs(paths, &p.rel_path);
        if filesystem::file_key(&at).as_deref() == Some(key.as_str()) {
            continue;
        }
        let tmp = PathBuf::from(format!("{}.relinking", at.display()));
        let _ = std::fs::remove_file(&tmp);
        let res = link_or_copy(&main, &tmp, p.linked).and_then(|_| std::fs::rename(&tmp, &at));
        if let Err(e) = res {
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("renewing {}: {e}", p.rel_path));
        }
        n += 1;
    }
    Ok(n)
}

pub fn abs(paths: &LibraryPaths, rel: &str) -> PathBuf {
    filesystem::db_to_abs(&paths.managed.to_string_lossy(), rel)
}

