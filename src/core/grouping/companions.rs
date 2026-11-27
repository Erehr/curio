//! Which files in one directory belong to an item: the rules formats declare (`formats[].companions`), matched by name.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::core::db::{Db, DbResult};
use crate::core::system::filesystem;
use crate::core::library::paths::LibraryPaths;
use crate::core::model::{AssetId, Root, RootKind};
use crate::core::plugins::manifest::CompanionDecl;

/// A name template: the primary's stem or whole name, then literal text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Template {
    Stem(String),
    Name(String),
}

impl Template {
    pub fn parse(t: &str) -> Option<Template> {
        let (whole, rest) = if let Some(r) = t.strip_prefix("{stem}") { (false, r) } else { (true, t.strip_prefix("{name}")?) };
        let bad = rest.is_empty() || rest.contains(['/', '\\', '{', '}', '*', '?', ':', '<', '>', '|', '"']) || rest.chars().any(|c| c.is_control());
        if bad {
            return None;
        }
        Some(if whole { Template::Name(rest.into()) } else { Template::Stem(rest.into()) })
    }

    fn rest(&self) -> &str {
        match self {
            Template::Stem(r) | Template::Name(r) => r,
        }
    }

    /// The name this template gives a primary named `name`.
    pub fn name_for(&self, name: &str) -> String {
        match self {
            Template::Stem(r) => format!("{}{r}", stem(name)),
            Template::Name(r) => format!("{name}{r}"),
        }
    }

    /// The extension a file it names has, lowercase.
    pub fn ext(&self) -> Option<String> {
        let r = self.rest();
        r.rfind('.').map(|i| r[i + 1..].to_lowercase()).filter(|e| !e.is_empty())
    }

    pub fn text(&self) -> String {
        match self {
            Template::Stem(r) => format!("{{stem}}{r}"),
            Template::Name(r) => format!("{{name}}{r}"),
        }
    }
}

/// Checks Curio knows; a rule naming another is left out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verify {
    LivePhoto,
}

impl Verify {
    pub fn parse(s: &str) -> Option<Verify> {
        (s == "live-photo").then_some(Verify::LivePhoto)
    }
}

#[derive(Clone, Debug)]
pub struct Rule {
    /// `<plugin id>/<role>`, or `library/raw-pairs`
    pub id: String,
    /// extensions of the primaries, lowercase, in the format's order
    pub exts: Vec<String>,
    pub role: String,
    pub templates: Vec<Template>,
    pub cover: bool,
    pub verify: Option<Verify>,
}

pub fn valid_role(r: &str) -> bool {
    !r.is_empty() && r.len() <= 40 && r.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

impl Rule {
    /// A declared rule, checked: `Err` says why it is left out. Bad templates are dropped, a rule left with none is too.
    pub fn from_decl(owner: &str, exts: &[String], d: &CompanionDecl) -> Result<Rule, String> {
        if !valid_role(&d.role) {
            return Err(format!("companion role `{}` must be letters, digits, - or _", d.role));
        }
        let verify = match &d.verify {
            None => None,
            Some(v) => Some(Verify::parse(v).ok_or_else(|| format!("companion `{}`: Curio has no check `{v}`", d.role))?),
        };
        let templates: Vec<Template> = d.files.iter().filter_map(|t| Template::parse(t)).collect();
        if templates.is_empty() {
            return Err(format!("companion `{}` has no file template starting with {{stem}} or {{name}}", d.role));
        }
        let mut seen = HashSet::new();
        let exts = exts.iter().map(|e| e.trim_start_matches('.').to_lowercase()).filter(|e| seen.insert(e.clone())).collect();
        Ok(Rule { id: format!("{owner}/{}", d.role), exts, role: d.role.clone(), templates, cover: d.cover, verify })
    }
}

/// A name without its last extension.
pub fn stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if i > 0 => &name[..i],
        _ => name,
    }
}

fn ext_of(name: &str) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 => name[i + 1..].to_lowercase(),
        _ => String::new(),
    }
}

/// What the library knows of a file already in it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Known {
    pub id: AssetId,
    pub part_of: Option<AssetId>,
    pub solo: bool,
    pub has_members: bool,
    /// not in the trash and available
    pub usable: bool,
}

#[derive(Clone, Debug)]
pub struct DirFile {
    pub name: String,
    pub known: Option<Known>,
}

/// `member` becomes a member of `item` (indices into the files given).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    pub member: usize,
    pub item: usize,
    pub rule: String,
    pub role: String,
    /// what follows the item's stem in the member's name; none when the item's stem doesn't start it
    pub suffix: Option<String>,
    /// the rule's check, run before the pair is kept
    pub verify: Option<Verify>,
    pub cover: bool,
}

/// As `plan_dir`, keeping only pairs `keep` accepts; one it refuses leaves both files free for other pairs.
pub fn plan_dir_checked(files: &[DirFile], rules: &[Rule], keep: &dyn Fn(&Pair) -> bool) -> Vec<Pair> {
    let mut by_name: HashMap<String, usize> = HashMap::new();
    for (i, f) in files.iter().enumerate() {
        by_name.entry(f.name.to_lowercase()).or_insert(i);
    }
    let by_id: HashMap<AssetId, usize> = files.iter().enumerate().filter_map(|(i, f)| f.known.map(|k| (k.id, i))).collect();
    let mut item_of: HashMap<usize, usize> = HashMap::new();
    let mut has_members: HashSet<usize> = HashSet::new();
    let mut out = Vec::new();
    let usable = |i: usize| files[i].known.map(|k| k.usable).unwrap_or(true);
    let solo = |i: usize| files[i].known.is_some_and(|k| k.solo);
    let exts: Vec<String> = files.iter().map(|f| ext_of(&f.name)).collect();
    for rule in rules {
        // (member, primary, {name} first, the primary's extension's place, the template's place)
        let mut cands: Vec<(usize, usize, u8, usize, usize)> = Vec::new();
        for (f, file) in files.iter().enumerate() {
            let Some(rank) = rule.exts.iter().position(|e| *e == exts[f]) else { continue };
            for (t, tpl) in rule.templates.iter().enumerate() {
                if let Some(&g) = by_name.get(&tpl.name_for(&file.name).to_lowercase()) {
                    if g != f {
                        cands.push((g, f, matches!(tpl, Template::Stem(_)) as u8, rank, t));
                    }
                }
            }
        }
        cands.sort_by_key(|&(g, f, kind, rank, t)| (g, kind, rank, t, f));
        let from = out.len();
        let mut places = Vec::new();
        for (g, f, _, _, t) in cands {
            if item_of.contains_key(&g) || has_members.contains(&g) || solo(g) || solo(f) || !usable(g) || !usable(f) {
                continue;
            }
            if files[g].known.is_some_and(|k| k.part_of.is_some() || k.has_members) {
                continue;
            }
            let item = match (item_of.get(&f), files[f].known.and_then(|k| k.part_of)) {
                (Some(&i), _) => i,
                (None, Some(p)) => match by_id.get(&p) {
                    Some(&i) => i,
                    None => continue,
                },
                (None, None) => f,
            };
            if item == g || solo(item) || !usable(item) {
                continue;
            }
            let st = stem(&files[item].name);
            let name = &files[g].name;
            let suffix = (name.len() > st.len() && name.is_char_boundary(st.len()) && name[..st.len()].eq_ignore_ascii_case(st)).then(|| name[st.len()..].to_string());
            let pair = Pair { member: g, item, rule: rule.id.clone(), role: rule.role.clone(), suffix, verify: rule.verify, cover: rule.cover };
            if !keep(&pair) {
                continue;
            }
            item_of.insert(g, item);
            has_members.insert(item);
            out.push(pair);
            places.push(t);
        }
        // of several covers one rule finds for an item, the one its earliest template names draws it
        if rule.cover {
            let mut best: HashMap<usize, usize> = HashMap::new();
            for (i, p) in out[from..].iter().enumerate() {
                let b = best.entry(p.item).or_insert(i);
                if places[i] < places[*b] {
                    *b = i;
                }
            }
            for (i, p) in out[from..].iter_mut().enumerate() {
                p.cover = best.get(&p.item) == Some(&i);
            }
        }
    }
    out
}

/// The library's own rule pairing camera RAW files with the JPEG or HEIC the camera wrote beside them: `jpeg` keeps the picture as the item, `raw` the RAW (with the picture as its cover, so thumbnails stay fast).
pub fn raw_rule(setting: &str) -> Option<Rule> {
    let raw: Vec<String> = crate::core::formats::image::RAW.iter().map(|e| e.to_string()).collect();
    let pictures = ["jpg", "jpeg", "heic", "heif", "hif"];
    let (exts, role, templates, cover): (Vec<String>, &str, Vec<Template>, bool) = match setting {
        "jpeg" => (pictures.iter().map(|e| e.to_string()).collect(), "raw", raw.iter().map(|e| Template::Stem(format!(".{e}"))).collect(), false),
        "raw" => (raw, "jpeg", pictures.iter().map(|e| Template::Stem(format!(".{e}"))).collect(), true),
        _ => return None,
    };
    Some(Rule { id: "library/raw-pairs".into(), exts, role: role.into(), templates, cover, verify: None })
}

/// The rules of Curio's own kinds of files, for the kinds switched on (by family id), in the order they are tried: a Live Photo's clip goes to its still before any rule can take the clip as an item.
pub fn builtin_rules(on: &HashSet<String>) -> Vec<Rule> {
    let raw = crate::core::formats::image::RAW;
    let photos = ["heic", "heif", "hif", "jpg", "jpeg"];
    let pictures: Vec<&str> = raw.iter().chain(photos.iter()).chain(["tif", "tiff", "png", "psd"].iter()).copied().collect();
    let clips = ["mov", "mp4", "m4v", "mts", "m2ts", "avi", "mxf"];
    let specs: [(&str, &str, Vec<&str>, &[&str], Option<&str>, bool); 5] = [
        ("image", "motion", photos.to_vec(), &["{stem}.mov"], Some("live-photo"), false),
        ("image", "edits", pictures.clone(), &["{stem}.aae"], None, false),
        ("video", "edits", clips.to_vec(), &["{stem}.aae"], None, false),
        ("image", "metadata", pictures, &["{name}.xmp", "{stem}.xmp"], None, false),
        ("video", "metadata", clips.to_vec(), &["{name}.xmp", "{stem}.xmp"], None, false),
    ];
    let mut out = vec![];
    for (fam, role, exts, files, verify, cover) in specs {
        let Some(f) = crate::core::formats::FAMILIES.iter().find(|f| f.id == fam).filter(|f| on.contains(f.id)) else { continue };
        let d = CompanionDecl { role: role.into(), files: files.iter().map(|s| s.to_string()).collect(), cover, verify: verify.map(String::from) };
        let exts: Vec<String> = exts.iter().map(|e| e.to_string()).collect();
        if let Ok(r) = Rule::from_decl(&crate::core::formats::plugin_id(f), &exts, &d) {
            out.push(r);
        }
    }
    out
}

/// Whether a planned pair passes its rule's check, reading both files.
pub fn passes(p: &Pair, item: &Path, member: &Path) -> bool {
    match p.verify {
        None => true,
        Some(Verify::LivePhoto) => crate::core::grouping::live_photo::is_live_photo(item, member),
    }
}

fn known_row(db: &Db, filter: &str, params: impl rusqlite::Params) -> DbResult<Option<Known>> {
    use rusqlite::OptionalExtension;
    db.conn
        .prepare_cached(&format!("SELECT a.id, a.part_of, a.solo, EXISTS (SELECT 1 FROM assets m WHERE m.part_of = a.id), a.deleted = 0 AND a.status = 0 AND NOT EXISTS (SELECT 1 FROM trashed_entries t WHERE t.asset_id = a.id) FROM assets a WHERE {filter}"))
        .and_then(|mut st| st.query_row(params, |r| Ok(Known { id: r.get(0)?, part_of: r.get(1)?, solo: r.get(2)?, has_members: r.get(3)?, usable: r.get(4)? })).optional())
        .map_err(|e| e.to_string())
}

/// The extensions files under these rules can have, as primaries or members: no other file can be part of a pair.
pub fn exts_of(rules: &[Rule]) -> HashSet<String> {
    rules.iter().flat_map(|r| r.exts.iter().cloned().chain(r.templates.iter().filter_map(|t| t.ext()))).collect()
}

/// A directory of a location as `plan_dir` takes it: the names on disk that may be in a pair, each with its row when it has one, and its path in the location.
/// Whether a file only rules take in has a file beside it that a rule would give it to: a few checks of names, before the file itself is read.
pub fn may_pair(path: &Path, rules: &[Rule], dirs: &mut DirNames) -> bool {
    let (Some(name), Some(dir)) = (path.file_name().map(|n| n.to_string_lossy().to_string()), path.parent()) else { return false };
    if !rules.iter().any(|r| r.templates.iter().any(|t| head_of(&name, t).is_some())) {
        return false;
    }
    may_pair_named(&name, dirs.of(dir), rules)
}

/// The name a template would give a primary for this companion's name, compared without case.
fn head_of<'a>(name: &'a str, t: &Template) -> Option<&'a str> {
    let rest = t.rest();
    let cut = name.len().wrapping_sub(rest.len());
    (name.len() > rest.len() && name.is_char_boundary(cut) && name[cut..].eq_ignore_ascii_case(rest)).then(|| &name[..cut])
}

/// `may_pair` over the lower-case names of the companion's directory: case on disk doesn't keep a pair apart.
pub fn may_pair_named(name: &str, names: &HashSet<String>, rules: &[Rule]) -> bool {
    rules.iter().any(|r| {
        r.templates.iter().any(|t| {
            let Some(head) = head_of(name, t) else { return false };
            let head = head.to_lowercase();
            match t {
                Template::Name(_) => r.exts.contains(&ext_of(&head)) && names.contains(&head),
                Template::Stem(_) => r.exts.iter().any(|e| names.contains(&format!("{head}.{}", e.to_lowercase()))),
            }
        })
    })
}

/// The lower-case file names of the directories `may_pair` looked into, each listed once.
#[derive(Default)]
pub struct DirNames(HashMap<std::path::PathBuf, HashSet<String>>);

impl DirNames {
    pub fn of(&mut self, dir: &Path) -> &HashSet<String> {
        if self.0.len() >= 256 && !self.0.contains_key(dir) {
            self.0.clear();
        }
        self.0.entry(dir.to_path_buf()).or_insert_with(|| std::fs::read_dir(dir).map(|rd| rd.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_file())).map(|e| e.file_name().to_string_lossy().to_lowercase()).collect()).unwrap_or_default())
    }
}

/// Whether a file's name and a new file's could belong together: one starts with the other's stem.
fn near(name: &str, new: &[String]) -> bool {
    let n = name.to_lowercase();
    new.iter().any(|m| n.starts_with(stem(m)) || m.starts_with(stem(&n)))
}

/// Directories with more new files than this are listed whole.
const NEAR_NAMES: usize = 64;

fn location_dir(db: &Db, root: &Root, dir: &str, exts: &HashSet<String>) -> DbResult<Vec<(DirFile, String)>> {
    location_dir_near(db, root, dir, exts, None)
}

/// As `location_dir`; with `only`, the names near those (lower case) alone, so a large directory costs a lookup per related file.
fn location_dir_near(db: &Db, root: &Root, dir: &str, exts: &HashSet<String>, only: Option<&[String]>) -> DbResult<Vec<(DirFile, String)>> {
    let abs = filesystem::db_to_abs(&root.path, dir);
    let Ok(rd) = std::fs::read_dir(&abs) else { return Ok(vec![]) };
    let prefix = if dir.is_empty() { String::new() } else { format!("{dir}/") };
    let mut out = vec![];
    for e in rd.flatten() {
        if !e.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if filesystem::is_ignored_name(&name) || !exts.contains(&ext_of(&name)) || only.is_some_and(|o| !near(&name, o)) {
            continue;
        }
        let rel = format!("{prefix}{name}");
        let known = known_row(db, "a.root_id = ?1 AND a.rel_path = ?2", rusqlite::params![root.id, rel])?;
        out.push((DirFile { name, known }, rel));
    }
    out.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    Ok(out)
}

/// After a scan or an add in these directories of a location: the files the rules give an item become its members, with rows made for those that had none. `new`: only pairs with one of these files (by path in the location) are made, so files already in the library are grouped only when asked. Answers each pair made, as (item, member).
pub fn after(db: &mut Db, root: &Root, dirs: &[String], rules: &[Rule], new: Option<&HashSet<String>>) -> DbResult<Vec<(AssetId, AssetId)>> {
    if rules.is_empty() || root.kind == RootKind::Managed {
        return Ok(vec![]);
    }
    let exts = exts_of(rules);
    let mut grouped = vec![];
    let mut seen = HashSet::new();
    // the new files' names by directory, gathered once: filtering every new path for each directory cost directories times files
    let mut new_by_dir: HashMap<String, Vec<String>> = HashMap::new();
    for r in new.into_iter().flatten() {
        new_by_dir.entry(dir_of(r)).or_default().push(r.rsplit('/').next().unwrap_or(r).to_lowercase());
    }
    for dir in dirs.iter().filter(|d| seen.insert(d.as_str())) {
        let here: &[String] = new_by_dir.get(dir.as_str()).map(|v| v.as_slice()).unwrap_or(&[]);
        let only = (new.is_some() && here.len() <= NEAR_NAMES).then_some(here);
        let listed = location_dir_near(db, root, dir, &exts, only)?;
        let files: Vec<DirFile> = listed.iter().map(|(f, _)| f.clone()).collect();
        let abs = |i: usize| filesystem::db_to_abs(&root.path, &listed[i].1);
        let is_new = |i: usize| new.is_some_and(|n| n.contains(&listed[i].1));
        // a location that only keeps what it is given takes a file in only beside one given now
        let may_add = |p: &Pair| files[p.member].known.is_some() || !root.explicit_only || is_new(p.item);
        // checks read the files: before the transaction
        let keep = |p: &Pair| (new.is_none() || is_new(p.item) || is_new(p.member) || files[p.member].known.is_none()) && files[p.item].known.is_some() && may_add(p) && passes(p, &abs(p.item), &abs(p.member));
        let pairs = plan_dir_checked(&files, rules, &keep);
        if pairs.is_empty() {
            continue;
        }
        let mut fresh = HashMap::new();
        for p in &pairs {
            if files[p.member].known.is_none() {
                let path = abs(p.member);
                let Ok(st) = filesystem::stat(&path) else { continue };
                if !crate::core::scan::is_forgotten(db.forgotten_one(root.id, &listed[p.member].1), &st) {
                    fresh.insert(p.member, crate::core::scan::scanned(Path::new(&root.path), &path, &st));
                }
            }
        }
        let made = db.tx(|db| {
            let mut made = vec![];
            for p in &pairs {
                let Some(item) = files[p.item].known.map(|k| k.id) else { continue };
                let row = match files[p.member].known {
                    Some(_) => None,
                    None => match fresh.get(&p.member) {
                        Some(f) => Some(f),
                        None => continue,
                    },
                };
                // a row made for a member that can't join goes again: such a file is never an item of its own
                db.conn.execute_batch("SAVEPOINT pair").map_err(|e| e.to_string())?;
                let member = match row {
                    Some(f) => db.insert_scanned(root.id, f)?,
                    None => files[p.member].known.map(|k| k.id).unwrap_or_default(),
                };
                if db.group(item, member, &p.role, &p.rule, p.suffix.as_deref(), p.cover).is_ok() {
                    db.conn.execute_batch("RELEASE pair").map_err(|e| e.to_string())?;
                    made.push((item, member));
                } else {
                    db.conn.execute_batch("ROLLBACK TO pair; RELEASE pair").map_err(|e| e.to_string())?;
                }
            }
            Ok(made)
        })?;
        grouped.extend(made);
    }
    Ok(grouped)
}

/// A directory of the library's store as `plan_dir` takes it: only the files that are entries of items, each standing for its item's row.
fn store_dir(db: &Db, paths: &LibraryPaths, dir: &str, exts: &HashSet<String>) -> DbResult<Vec<(DirFile, String)>> {
    let abs = crate::core::store::abs(paths, dir);
    let Ok(rd) = std::fs::read_dir(&abs) else { return Ok(vec![]) };
    let prefix = if dir.is_empty() { String::new() } else { format!("{dir}/") };
    let mut out = vec![];
    for e in rd.flatten() {
        if !e.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if !exts.contains(&ext_of(&name)) {
            continue;
        }
        let rel = format!("{prefix}{name}");
        let Some(p) = db.placement_at(&rel) else { continue };
        let known = known_row(db, "a.id = ?1", [p.asset_id])?;
        out.push((DirFile { name, known }, rel));
    }
    out.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    Ok(out)
}

/// As `after`, for directories of the library's store, where each file is an entry of an item: rows are never made, and both files of a pair are entries in that directory. `new`: as in `after`, by item.
pub fn after_store(db: &mut Db, paths: &LibraryPaths, dirs: &[String], rules: &[Rule], new: Option<&HashSet<AssetId>>) -> DbResult<Vec<(AssetId, AssetId)>> {
    if rules.is_empty() {
        return Ok(vec![]);
    }
    let exts = exts_of(rules);
    let mut grouped = vec![];
    let mut seen = HashSet::new();
    for dir in dirs.iter().filter(|d| seen.insert(d.as_str()) && !d.starts_with(crate::core::store::TRASH_DIR)) {
        let listed = store_dir(db, paths, dir, &exts)?;
        let files: Vec<DirFile> = listed.iter().map(|(f, _)| f.clone()).collect();
        let abs = |i: usize| crate::core::store::abs(paths, &listed[i].1);
        let is_new = |i: usize| new.is_some_and(|n| files[i].known.is_some_and(|k| n.contains(&k.id)));
        let keep = |p: &Pair| (new.is_none() || is_new(p.item) || is_new(p.member)) && passes(p, &abs(p.item), &abs(p.member));
        let pairs = plan_dir_checked(&files, rules, &keep);
        if pairs.is_empty() {
            continue;
        }
        let made = db.tx(|db| {
            let mut made = vec![];
            for p in &pairs {
                let (Some(i), Some(m)) = (files[p.item].known, files[p.member].known) else { continue };
                if db.group(i.id, m.id, &p.role, &p.rule, p.suffix.as_deref(), p.cover).is_ok() {
                    made.push((i.id, m.id));
                }
            }
            Ok(made)
        })?;
        grouped.extend(made);
    }
    Ok(grouped)
}

/// After these items came into the library's store: each takes the members the rules give it beside it, or becomes a member of an item there.
pub fn after_store_items(db: &mut Db, paths: &LibraryPaths, ids: &[AssetId], rules: &[Rule]) -> DbResult<Vec<(AssetId, AssetId)>> {
    if rules.is_empty() || ids.is_empty() {
        return Ok(vec![]);
    }
    let mut dirs = vec![];
    for id in ids {
        dirs.extend(db.placements_of(*id)?.into_iter().map(|p| dir_of(&p.rel_path)));
    }
    let new: HashSet<AssetId> = ids.iter().copied().collect();
    after_store(db, paths, &dirs, rules, Some(&new))
}

/// Pairs among files about to be copied into the library, planned in their own folders: `files` may take members, and `Pair`'s indices are into the returned paths. Members not among `files` are brought along.
pub fn plan_copies(files: &[std::path::PathBuf], rules: &[Rule]) -> Vec<(std::path::PathBuf, std::path::PathBuf, Pair)> {
    if rules.is_empty() {
        return vec![];
    }
    let exts = exts_of(rules);
    let given: HashSet<&std::path::PathBuf> = files.iter().collect();
    let mut by_dir: HashMap<std::path::PathBuf, Vec<&std::path::PathBuf>> = HashMap::new();
    for f in files {
        if let Some(d) = f.parent() {
            by_dir.entry(d.to_path_buf()).or_default().push(f);
        }
    }
    let mut out = vec![];
    for dir in by_dir.keys() {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        let mut names: Vec<String> = rd.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_file())).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| !filesystem::is_ignored_name(n) && exts.contains(&ext_of(n))).collect();
        names.sort();
        let listed: Vec<DirFile> = names.iter().map(|n| DirFile { name: n.clone(), known: None }).collect();
        let keep = |p: &Pair| given.contains(&dir.join(&names[p.item])) && passes(p, &dir.join(&names[p.item]), &dir.join(&names[p.member]));
        for p in plan_dir_checked(&listed, rules, &keep) {
            out.push((dir.join(&names[p.item]), dir.join(&names[p.member]), p));
        }
    }
    out
}

/// The directory part of a path in a location or the store.
pub fn dir_of(rel: &str) -> String {
    rel.rfind('/').map(|i| rel[..i].to_string()).unwrap_or_default()
}

/// Every directory holding items or members: `(root, dir)`, the store as root `-1`.
fn all_dirs(db: &Db) -> DbResult<Vec<(i64, String)>> {
    let mut out: HashSet<(i64, String)> = HashSet::new();
    let mut st = db.conn.prepare("SELECT a.root_id, a.rel_path FROM assets a JOIN roots r ON r.id = a.root_id WHERE a.deleted = 0 AND r.kind <> 'managed'").map_err(|e| e.to_string())?;
    for r in st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))).map_err(|e| e.to_string())? {
        let (root, rel) = r.map_err(|e| e.to_string())?;
        out.insert((root, dir_of(&rel)));
    }
    let mut st = db.conn.prepare("SELECT rel_path FROM placements").map_err(|e| e.to_string())?;
    for r in st.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())? {
        out.insert((-1, dir_of(&r.map_err(|e| e.to_string())?)));
    }
    let mut v: Vec<_> = out.into_iter().collect();
    v.sort();
    Ok(v)
}

/// What grouping the files already in the library would do: for each rule, how many files it would group. Nothing is written.
pub fn survey(db: &Db, paths: &LibraryPaths, rules: &[Rule]) -> DbResult<Vec<(String, usize)>> {
    let mut counts: Vec<(String, usize)> = rules.iter().map(|r| (r.id.clone(), 0)).collect();
    let exts = exts_of(rules);
    let roots: HashMap<i64, Root> = db.roots()?.into_iter().map(|r| (r.id, r)).collect();
    for (root, dir) in all_dirs(db)? {
        let listed = match roots.get(&root) {
            _ if root == -1 => store_dir(db, paths, &dir, &exts)?,
            Some(r) => location_dir(db, r, &dir, &exts)?,
            None => continue,
        };
        let may_add = root == -1 || roots.get(&root).is_some_and(|r| !r.explicit_only);
        let abs = |i: usize| match roots.get(&root) {
            Some(r) if root != -1 => filesystem::db_to_abs(&r.path, &listed[i].1),
            _ => crate::core::store::abs(paths, &listed[i].1),
        };
        let files: Vec<DirFile> = listed.iter().map(|(f, _)| f.clone()).collect();
        let keep = |p: &Pair| files[p.item].known.is_some() && (may_add || files[p.member].known.is_some()) && passes(p, &abs(p.item), &abs(p.member));
        for p in plan_dir_checked(&files, rules, &keep) {
            if let Some(c) = counts.iter_mut().find(|c| c.0 == p.rule) {
                c.1 += 1;
            }
        }
    }
    Ok(counts)
}

/// Group the files already in the library as the rules say, directory by directory; `stop` ends it between directories. Answers each pair made, as (item, member).
pub fn group_existing(db: &mut Db, paths: &LibraryPaths, rules: &[Rule], progress: &dyn Fn(usize, usize), stop: &std::sync::atomic::AtomicBool) -> DbResult<Vec<(AssetId, AssetId)>> {
    let dirs = all_dirs(db)?;
    let roots: HashMap<i64, Root> = db.roots()?.into_iter().map(|r| (r.id, r)).collect();
    let mut out = vec![];
    for (i, (root, dir)) in dirs.iter().enumerate() {
        if stop.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        if *root == -1 {
            out.extend(after_store(db, paths, std::slice::from_ref(dir), rules, None)?);
        } else if let Some(r) = roots.get(root) {
            out.extend(after(db, r, std::slice::from_ref(dir), rules, None)?);
        }
        progress(i + 1, dirs.len());
    }
    Ok(out)
}
