//! The duplicate finder: identical files by their full hashes, and pictures, videos, sounds and archives that look or sound alike by the prints of `prints.rs`, grouped so that every member of a group matches every other one.
//! A search runs on a thread of its own with a read connection of its own; the hashes and prints it works out are written back through the writer for the next one.

mod archives;
pub mod fingerprints;
mod identical;
mod looks;
mod media;
pub mod resolution;
mod scope;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rusqlite::params_from_iter;

use crate::core::db::{Db, DbResult};
use crate::core::library::{changes::Changed, Ctx};
use crate::core::model::{AssetId, AssetKind};
use crate::core::duplicates::archives::archives;
use crate::core::duplicates::media::media;
use crate::core::duplicates::looks::looks;
use crate::core::duplicates::identical::identical;
use crate::core::duplicates::scope::Universe;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Way {
    Content,
    Looks,
    Video,
    Sound,
    Archive,
}

impl Way {
    pub const ALL: [Way; 5] = [Way::Content, Way::Looks, Way::Video, Way::Sound, Way::Archive];

    pub fn name(self) -> &'static str {
        match self {
            Way::Content => "content",
            Way::Looks => "looks",
            Way::Video => "video",
            Way::Sound => "sound",
            Way::Archive => "archive",
        }
    }

    pub fn parse(s: &str) -> Option<Way> {
        Way::ALL.into_iter().find(|w| w.name() == s)
    }

    /// The similarity unless asked for another.
    pub fn default_threshold(self) -> f64 {
        match self {
            Way::Content => 1.0,
            Way::Looks | Way::Video => crate::core::consts::SIMILAR_THRESHOLD,
            Way::Sound => DEFAULT_SOUND_THRESHOLD,
            Way::Archive => DEFAULT_ARCHIVE_THRESHOLD,
        }
    }

    /// The loosest similarity a search takes: looser looks would read thousands of buckets a picture.
    pub fn loosest(self) -> f64 {
        match self {
            Way::Content => 1.0,
            Way::Looks => 0.8,
            Way::Video => 0.7,
            Way::Sound => 0.4,
            Way::Archive => 0.5,
        }
    }
}

/// 1 − 2 × the share of bits that differ: copies at another bit rate come out from 0.7 to 0.95, unrelated sounds near 0.
const DEFAULT_SOUND_THRESHOLD: f64 = 0.7;
const DEFAULT_ARCHIVE_THRESHOLD: f64 = 0.9;
/// Pictures sharing one DCT hash past this many are too plain to tell apart (blank pages, one colour); they are counted, not compared.
const PLAIN_BUCKET: usize = 1000;
/// Entries found in more archives than this (a scanner's credits page) don't make candidates.
const COMMON_ENTRY: usize = 200;
/// A group stops taking members at this size.
const MAX_GROUP: usize = 500;
/// Problems listed by name; past this they are only counted.
const LISTED_PROBLEMS: usize = 100;
const WRITE_BATCH: usize = 500;
const HASH_THREADS: usize = 4;
const FFMPEG_THREADS: usize = 3;

#[derive(Clone, Debug)]
pub struct Search {
    /// The ways to compare, each with its similarity (1 for identical files).
    pub ways: Vec<(Way, f64)>,
    /// The kinds whose thumbnails are compared by their looks.
    pub look_kinds: Vec<AssetKind>,
    /// The items compared; every item when None.
    pub scope: Option<Vec<AssetId>>,
    /// The scope compared with every item, not only with itself.
    pub with_library: bool,
    /// Files smaller than this are left out.
    pub min_size: i64,
}

impl Search {
    fn threshold(&self, w: Way) -> Option<f64> {
        self.ways.iter().find(|x| x.0 == w).map(|x| x.1.clamp(w.loosest(), 1.0))
    }
}

/// A member of a group: `same` the same number for identical files, `file` for hard links of one file (removing one frees nothing), `store` a file in the library's own folder.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub id: AssetId,
    pub same: Option<usize>,
    pub file: Option<usize>,
    pub store: bool,
    pub size: i64,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub members: Vec<Member>,
    /// How two members that aren't identical match: their places in `members`, the way and the similarity.
    pub links: Vec<(usize, usize, Way, f64)>,
}

#[derive(Clone, Debug, Default)]
pub struct Found {
    pub groups: Vec<Group>,
    /// How many items each way compared.
    pub checked: Vec<(Way, usize)>,
    /// Items in scope whose file is missing or offline.
    pub missing: usize,
    /// Items in scope whose thumbnail isn't made yet, so their looks weren't compared.
    pub no_thumbnail: usize,
    /// Videos and sounds in scope left out for want of FFmpeg.
    pub no_ffmpeg: usize,
    /// Pictures too plain to tell apart.
    pub plain: usize,
    /// Files that couldn't be read, the first of them with why.
    pub unreadable: Vec<(AssetId, String)>,
    pub unreadable_count: usize,
    pub stopped: bool,
    pub took_ms: u64,
}

pub type Progress = dyn Fn(f64, f64, &str) + Send + Sync;

/// Runs a search on a thread of its own; a search already running in this library stops. `done` hears what was found, or why nothing could be.
pub fn spawn(ctx: Arc<Ctx>, search: Search, stop: Arc<AtomicBool>, progress: Box<Progress>, done: Box<dyn FnOnce(Option<&Db>, Result<Found, String>) + Send>) {
    if let Some(old) = ctx.dup_search.lock().replace(stop.clone()) {
        old.store(true, Ordering::Relaxed);
    }
    let run = move || {
        let db = Db::open(&ctx.paths.db);
        let r = db.as_ref().map_err(|e| e.clone()).and_then(|db| find(db, &*ctx, &search, &stop, &*progress));
        let mut held = ctx.dup_search.lock();
        if held.as_ref().is_some_and(|s| Arc::ptr_eq(s, &stop)) {
            *held = None;
        }
        drop(held);
        done(db.as_ref().ok(), r);
    };
    if let Err(e) = std::thread::Builder::new().name("duplicates".into()).spawn(run) {
        crate::core::system::logging::error(&format!("couldn't start the duplicate search: {e}"));
    }
}

/// What a search found. Stopping it answers what it had.
pub fn find(db: &Db, ctx: &dyn Store, s: &Search, stop: &AtomicBool, progress: &Progress) -> DbResult<Found> {
    let started = Instant::now();
    let u = Universe::new(db, s)?;
    let mut found = Found { missing: u.count(db, " AND a.status <> 0")?, ..Default::default() };
    let mut graph = Graph::default();
    let mut problems = Problems::default();
    if s.threshold(Way::Content).is_some() && !stop.load(Ordering::Relaxed) {
        let n = identical(db, ctx, s, &u, &mut graph, &mut problems, stop, progress)?;
        found.checked.push((Way::Content, n));
    }
    if let Some(t) = s.threshold(Way::Looks).filter(|_| !stop.load(Ordering::Relaxed)) {
        let n = looks(db, ctx, s, &u, t, &mut graph, &mut found, stop, progress)?;
        found.checked.push((Way::Looks, n));
    }
    for (way, kind) in [(Way::Video, "video"), (Way::Sound, "audio")] {
        if let Some(t) = s.threshold(way).filter(|_| !stop.load(Ordering::Relaxed)) {
            let n = media(db, ctx, &u, way, kind, t, &mut graph, &mut found, &mut problems, stop, progress)?;
            found.checked.push((way, n));
        }
    }
    if let Some(t) = s.threshold(Way::Archive).filter(|_| !stop.load(Ordering::Relaxed)) {
        let n = archives(db, ctx, &u, t, &mut graph, &mut problems, stop, progress)?;
        found.checked.push((Way::Archive, n));
    }
    found.stopped = stop.load(Ordering::Relaxed);
    progress(0.0, 0.0, "Grouping");
    graph.ignored = ignored(db, &graph)?;
    found.groups = graph.groups(db, &u)?;
    found.unreadable_count = problems.count;
    found.unreadable = problems.listed;
    found.took_ms = started.elapsed().as_millis() as u64;
    Ok(found)
}

// ------------------------------------------------------------------ scope

#[derive(Default)]
struct Problems {
    listed: Vec<(AssetId, String)>,
    count: usize,
}

impl Problems {
    fn add(&mut self, id: AssetId, why: String) {
        self.count += 1;
        if self.listed.len() < LISTED_PROBLEMS {
            self.listed.push((id, why));
        }
    }
}

/// Works through `items` on `threads` threads, stopping early when asked; `f` answers each item's result, `weight` what it counts for in `progress`.
fn in_parallel<T: Sync, R: Send>(items: &[T], threads: usize, stop: &AtomicBool, weight: impl Fn(&T) -> u64 + Sync, progress: &(dyn Fn(u64) + Sync), f: impl Fn(&T) -> R + Sync) -> Vec<(usize, R)> {
    let next = AtomicUsize::new(0);
    let done = AtomicU64::new(0);
    let out = Mutex::new(Vec::with_capacity(items.len()));
    std::thread::scope(|sc| {
        for _ in 0..threads.clamp(1, items.len().max(1)) {
            sc.spawn(|| loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(i) else { break };
                let r = f(item);
                let now = done.fetch_add(weight(item), Ordering::Relaxed) + weight(item);
                out.lock().unwrap_or_else(|e| e.into_inner()).push((i, r));
                progress(now);
            });
        }
    });
    out.into_inner().unwrap_or_else(|e| e.into_inner())
}

/// Writes in batches through the writer, so the search never holds the library's write lock.
fn write_back<T: Send + 'static>(ctx: &dyn Store, rows: Vec<T>, f: fn(&Db, &T) -> DbResult<()>) {
    let mut rows = rows;
    while !rows.is_empty() {
        let batch: Vec<T> = rows.drain(..rows.len().min(WRITE_BATCH)).collect();
        ctx.keep(Box::new(move |db| {
            for r in &batch {
                f(db, r)?;
            }
            Ok(())
        }));
    }
}

/// Where a search finds thumbnails and keeps what it works out: the library's writer, or nothing in a benchmark.
pub trait Store: Sync {
    fn thumbs(&self) -> &Path;
    fn keep(&self, f: Box<dyn FnOnce(&Db) -> DbResult<()> + Send>);
}

impl Store for Ctx {
    fn thumbs(&self) -> &Path {
        &self.paths.thumbs
    }
    fn keep(&self, f: Box<dyn FnOnce(&Db) -> DbResult<()> + Send>) {
        self.write(move |db, _| f(db).map(|_| Changed::default()));
    }
}

fn abs_paths(db: &Db, ids: &[AssetId]) -> HashMap<AssetId, PathBuf> {
    ids.iter().filter_map(|i| db.abs_path(*i).map(|p| (*i, p))).collect()
}

// ------------------------------------------------------------------ groups

/// Items found alike: classes of identical files, and links between two items by another way.
#[derive(Default)]
struct Graph {
    class_of: HashMap<AssetId, usize>,
    classes: Vec<Vec<AssetId>>,
    links: HashMap<(AssetId, AssetId), (Way, f64)>,
    ignored: HashSet<(AssetId, AssetId)>,
}

fn pair(a: AssetId, b: AssetId) -> (AssetId, AssetId) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

impl Graph {
    fn add_class(&mut self, ids: Vec<AssetId>) {
        let n = self.classes.len();
        for id in &ids {
            self.class_of.insert(*id, n);
        }
        self.classes.push(ids);
    }

    /// The better of two links between the same items stays.
    fn add_link(&mut self, a: AssetId, b: AssetId, way: Way, score: f64) {
        if a == b {
            return;
        }
        let e = self.links.entry(pair(a, b)).or_insert((way, score));
        if score > e.1 {
            *e = (way, score);
        }
    }

    fn same(&self, a: AssetId, b: AssetId) -> bool {
        matches!((self.class_of.get(&a), self.class_of.get(&b)), (Some(x), Some(y)) if x == y)
    }

    fn linked(&self, a: AssetId, b: AssetId) -> bool {
        !self.ignored.contains(&pair(a, b)) && (self.same(a, b) || self.links.contains_key(&pair(a, b)))
    }

    /// Complete linkage: two groups join only when every member of one matches every member of the other, identical classes first, then links from the most alike, so a chain of near pictures never becomes one group.
    fn groups(&self, db: &Db, u: &Universe) -> DbResult<Vec<Group>> {
        let mut nodes: Vec<AssetId> = self.class_of.keys().copied().chain(self.links.keys().flat_map(|(a, b)| [*a, *b])).collect();
        nodes.sort_unstable();
        nodes.dedup();
        let mut cluster: HashMap<AssetId, usize> = nodes.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        let mut members: Vec<Vec<AssetId>> = nodes.iter().map(|id| vec![*id]).collect();
        let join = |a: AssetId, b: AssetId, cluster: &mut HashMap<AssetId, usize>, members: &mut Vec<Vec<AssetId>>| {
            let (x, y) = (cluster[&a], cluster[&b]);
            if x == y || members[x].len() + members[y].len() > MAX_GROUP || !members[x].iter().all(|p| members[y].iter().all(|q| self.linked(*p, *q))) {
                return;
            }
            let (keep, gone) = if members[x].len() >= members[y].len() { (x, y) } else { (y, x) };
            let moved = std::mem::take(&mut members[gone]);
            for m in &moved {
                cluster.insert(*m, keep);
            }
            members[keep].extend(moved);
        };
        for class in &self.classes {
            let blocked = class.iter().any(|a| class.iter().any(|b| self.ignored.contains(&pair(*a, *b))));
            if blocked {
                for x in 0..class.len() {
                    for y in x + 1..class.len() {
                        join(class[x], class[y], &mut cluster, &mut members);
                    }
                }
            } else {
                for m in &class[1..] {
                    join(class[0], *m, &mut cluster, &mut members);
                }
            }
        }
        let mut order: Vec<(&(AssetId, AssetId), &(Way, f64))> = self.links.iter().collect();
        order.sort_by(|a, b| b.1 .1.total_cmp(&a.1 .1).then(a.0.cmp(b.0)));
        for ((a, b), _) in order {
            join(*a, *b, &mut cluster, &mut members);
        }
        let groups: Vec<Vec<AssetId>> = members.into_iter().filter(|m| m.len() > 1).collect();
        let all: Vec<AssetId> = groups.iter().flatten().copied().collect();
        let facts = facts(db, &all)?;
        let mine: HashSet<AssetId> = if u.scoped && u.with_library { scope_ids(db)? } else { HashSet::new() };
        let mut out = Vec::new();
        for mut ids in groups {
            if u.scoped && u.with_library && !ids.iter().any(|i| mine.contains(i)) {
                continue;
            }
            ids.sort_unstable();
            let mut same_of: HashMap<usize, usize> = HashMap::new();
            let mut file_of: HashMap<&str, usize> = HashMap::new();
            let file_counts: HashMap<&str, usize> = ids.iter().filter_map(|i| facts.get(i).and_then(|f| f.1.as_deref())).fold(HashMap::new(), |mut m, k| {
                *m.entry(k).or_default() += 1;
                m
            });
            let class_counts: HashMap<usize, usize> = ids.iter().filter_map(|i| self.class_of.get(i)).fold(HashMap::new(), |mut m, c| {
                *m.entry(*c).or_default() += 1;
                m
            });
            let members: Vec<Member> = ids
                .iter()
                .map(|id| {
                    let f = facts.get(id);
                    let key: Option<&str> = f.and_then(|f| f.1.as_deref());
                    let same = self.class_of.get(id).filter(|c| class_counts[c] > 1).map(|c| {
                        let n = same_of.len();
                        *same_of.entry(*c).or_insert(n)
                    });
                    let file = key.filter(|k| file_counts.get(k).copied().unwrap_or(0) > 1).map(|k| {
                        let n = file_of.len();
                        *file_of.entry(k).or_insert(n)
                    });
                    Member { id: *id, same, file, store: f.is_some_and(|f| f.2), size: f.map_or(0, |f| f.0) }
                })
                .collect();
            let mut links = Vec::new();
            for x in 0..ids.len() {
                for y in x + 1..ids.len() {
                    if members[x].same.is_some() && members[x].same == members[y].same {
                        continue;
                    }
                    if let Some((w, s)) = self.links.get(&pair(ids[x], ids[y])) {
                        links.push((x, y, *w, (*s * 1000.0).round() / 1000.0));
                    }
                }
            }
            out.push(Group { members, links });
        }
        // the most space to free first
        out.sort_by_key(|g| std::cmp::Reverse(g.members.iter().map(|m| m.size).sum::<i64>() - g.members.iter().map(|m| m.size).max().unwrap_or(0)));
        Ok(out)
    }
}

/// Size, file identity and whether the file is in the library's own folder, for each of these items.
fn facts(db: &Db, ids: &[AssetId]) -> DbResult<HashMap<AssetId, (i64, Option<String>, bool)>> {
    let mut out = HashMap::new();
    for chunk in ids.chunks(500) {
        let marks = vec!["?"; chunk.len()].join(",");
        let mut st = db.conn.prepare(&format!("SELECT a.id, a.size, a.file_key, r.kind = 'managed' FROM assets a JOIN roots r ON r.id = a.root_id WHERE a.id IN ({marks})")).map_err(|e| e.to_string())?;
        let rows = st.query_map(params_from_iter(chunk.iter()), |r| Ok((r.get::<_, AssetId>(0)?, (r.get::<_, i64>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, bool>(3)?)))).map_err(|e| e.to_string())?;
        out.extend(rows.filter_map(|r| r.ok()));
    }
    Ok(out)
}

fn scope_ids(db: &Db) -> DbResult<HashSet<AssetId>> {
    let mut st = db.conn.prepare("SELECT id FROM temp.dup_scope").map_err(|e| e.to_string())?;
    let rows = st.query_map([], |r| r.get(0)).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// The pairs marked as not duplicates among the items found.
fn ignored(db: &Db, g: &Graph) -> DbResult<HashSet<(AssetId, AssetId)>> {
    let mut st = db.conn.prepare("SELECT a, b FROM dup_ignored").map_err(|e| e.to_string())?;
    let rows = st.query_map([], |r| Ok((r.get::<_, AssetId>(0)?, r.get::<_, AssetId>(1)?))).map_err(|e| e.to_string())?;
    let touched: HashSet<AssetId> = g.class_of.keys().copied().chain(g.links.keys().flat_map(|(a, b)| [*a, *b])).collect();
    Ok(rows.filter_map(|r| r.ok()).filter(|(a, b)| touched.contains(a) && touched.contains(b)).collect())
}

