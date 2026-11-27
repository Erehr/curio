//! Auto-import with several watched folders and rules, per library.
//!
//! Each watch has a default destination and tags, a way of treating subfolders, and an ordered
//! list of rules evaluated against the file on disk: a later match replaces the destination
//! folder, tags from every match add up, and a rule can keep a file out altogether.
//!
//! Settings live in the library and travel with it, so they are not trusted on their own: a
//! watch only runs on a computer where it was allowed. Creating or changing one in this app
//! allows it; one that arrives any other way waits for the person to allow it.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::db::{Db, DbResult};
use crate::core::system::filesystem;
use crate::core::import::import_managed;
use crate::core::library::paths::LibraryPaths;
use crate::core::model::AssetId;
use crate::core::said::Said;

/// A file must stay unchanged this long, as seen by us, before it is imported. Its own
/// timestamps aren't trusted: copy tools preserve them.
pub const STABLE_MS: i64 = 2500;
const AFTER_FOLDER: &str = ".imported";

fn yes() -> bool {
    true
}
fn flat() -> String {
    "flat".into()
}
fn keep() -> String {
    "move".into()
}
fn all() -> String {
    "all".into()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AutoImportConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub watches: Vec<Watch>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Watch {
    pub id: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    /// off: only files directly inside; flat: everything into one destination; mirror: recreate
    /// the source tree as folders; match: only subfolders that already exist as folders.
    #[serde(default = "flat")]
    pub subfolders: String,
    /// Comma separated, blank means every file.
    #[serde(default)]
    pub extensions: String,
    /// Comma separated patterns such as `*.txt, temp_*, raw/**`.
    #[serde(default)]
    pub ignore: String,
    #[serde(default)]
    pub folder: Option<i64>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// move: into `.imported` beside the files; delete: to the Recycle Bin, after verifying the
    /// copy; keep: leave it.
    #[serde(default = "keep")]
    pub after: String,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Rule {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub name: String,
    /// all or any
    #[serde(default = "all")]
    pub matching: String,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    #[serde(default)]
    pub folder: Option<i64>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub rating: i64,
    #[serde(default)]
    pub note: String,
    /// Do not import at all.
    #[serde(default)]
    pub skip: bool,
    /// Stop evaluating later rules once this one matches.
    #[serde(default)]
    pub stop: bool,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Condition {
    pub field: String,
    pub op: String,
    #[serde(default)]
    pub value: String,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

// ------------------------------------------------------------------ what never gets imported

const JUNK_NAMES: &[&str] = &["desktop.ini", "thumbs.db", "ehthumbs.db", "ehthumbs_vista.db", ".ds_store", ".localized", "picasa.ini"];
const JUNK_PREFIXES: &[&str] = &["~$", "._", ".~lock."];
const PARTIAL_EXTS: &[&str] = &[
    "tmp", "temp", "part", "partial", "crdownload", "download", "filepart", "opdownload", "crswap", "aria2", "!qb", "!ut", "bc!", "swp", "swx", "lock", "lck", "lnk", "url", "webloc",
    "icloud",
];
/// Files never imported on their own, but brought in with the file a grouping rule gives them to.
const SIDECAR_EXTS: &[&str] = &["xmp"];
const SKIP_DIRS: &[&str] = &["@eadir", "#recycle", "#snapshot", "@recycle", "$recycle.bin", "recycler", "system volume information", AFTER_FOLDER, "lost+found"];
const SKIP_DIR_EXTS: &[&str] = &["library", "curio", "photoslibrary", "aplibrary", "lrlibrary", "app"];

pub fn file_is_junk(name: &str) -> bool {
    let lower = name.to_lowercase();
    if name.starts_with('.') || JUNK_NAMES.contains(&lower.as_str()) || JUNK_PREFIXES.iter().any(|p| lower.starts_with(p)) {
        return true;
    }
    let ext = lower.rsplit_once('.').map(|x| x.1).unwrap_or("");
    !ext.is_empty() && (PARTIAL_EXTS.contains(&ext) || SIDECAR_EXTS.contains(&ext) || ext.starts_with('~') || ext.starts_with("sb-"))
}

fn is_sidecar(name: &str) -> bool {
    !name.starts_with('.') && name.rsplit_once('.').is_some_and(|(_, e)| SIDECAR_EXTS.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

pub fn dir_is_skipped(name: &str) -> bool {
    let lower = name.to_lowercase();
    if name.starts_with('.') || SKIP_DIRS.contains(&lower.as_str()) {
        return true;
    }
    lower.rsplit_once('.').map(|x| SKIP_DIR_EXTS.contains(&x.1)).unwrap_or(false)
}

/// Why this folder can't be watched, if it can't.
pub fn watch_problem(path: &str, library: &Path) -> Option<Said> {
    let p = path.trim();
    if p.is_empty() {
        return Some(Said::new("errors.autoImport.choose", serde_json::Value::Null, "Choose a folder to watch"));
    }
    let pb = Path::new(p);
    if !pb.is_absolute() {
        return Some(Said::new("errors.autoImport.invalidPath", serde_json::Value::Null, "This path isn't valid on this computer"));
    }
    if !pb.exists() {
        return Some(Said::new("errors.autoImport.missing", serde_json::Value::Null, "This folder doesn't exist right now"));
    }
    let real = filesystem::canonical(pb);
    if real.parent().is_none() || real.parent().map(|x| x.parent().is_none() && cfg!(windows) && real.components().count() < 2).unwrap_or(false) {
        return Some(Said::new("errors.autoImport.drive", serde_json::Value::Null, "A whole drive can't be watched"));
    }
    let lib = filesystem::canonical(library);
    if real.starts_with(&lib) || lib.starts_with(&real) {
        return Some(Said::new("errors.autoImport.library", serde_json::Value::Null, "A folder containing this library, or inside it, can't be watched"));
    }
    if crate::core::library::instance::in_library_store(&real) {
        return Some(Said::new("errors.autoImport.otherLibrary", serde_json::Value::Null, "A folder inside another library can't be watched"));
    }
    if let Some(home) = dirs::home_dir() {
        let home = filesystem::canonical(&home);
        if real == home || home.starts_with(&real) {
            return Some(Said::new("errors.autoImport.home", serde_json::Value::Null, "Your home folder can't be watched; choose a folder inside it"));
        }
    }
    let system: Vec<PathBuf> = if cfg!(windows) {
        ["SystemRoot", "ProgramFiles", "ProgramFiles(x86)", "ProgramData"].iter().filter_map(|v| std::env::var_os(v)).map(PathBuf::from).collect()
    } else {
        ["/bin", "/boot", "/dev", "/etc", "/lib", "/proc", "/run", "/sbin", "/sys", "/usr", "/var"].iter().map(PathBuf::from).collect()
    };
    if system.iter().any(|s| real.starts_with(filesystem::canonical(s))) {
        return Some(Said::new("errors.autoImport.system", serde_json::Value::Null, "A system folder can't be watched"));
    }
    None
}

// ------------------------------------------------------------------ rules

/// What a rule is tested against.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    pub fullpath: String,
    pub filename: String,
    pub name: String,
    pub ext: String,
    /// Relative to the watched folder, `/` separated, empty when directly inside it.
    pub relpath: String,
    pub size: i64,
    pub age_days: f64,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

pub fn facts_of(file: &Path, root: &Path, size: i64, age_days: f64, needs_dims: bool) -> Facts {
    let rel = file.strip_prefix(root).unwrap_or(file);
    let relpath = rel.parent().map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
    let dims = if needs_dims { imagesize::size(file).ok() } else { None };
    Facts {
        fullpath: file.to_string_lossy().to_string(),
        filename: filesystem::name_of(file),
        name: filesystem::stem_of(file),
        ext: filesystem::ext_of(file),
        relpath,
        size,
        age_days,
        width: dims.as_ref().map(|d| d.width as i64),
        height: dims.as_ref().map(|d| d.height as i64),
    }
}

/// "50", "50KB", "2.5 MB", "1gb" → bytes. Bare numbers are bytes.
pub fn parse_size(v: &str) -> Option<f64> {
    let t = v.trim().to_lowercase();
    let split = t.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(t.len());
    let n: f64 = t[..split].parse().ok()?;
    let mult = match t[split..].trim() {
        "" | "b" => 1.0,
        "k" | "kb" => 1024.0,
        "m" | "mb" => 1024.0 * 1024.0,
        "g" | "gb" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some(n * mult)
}

/// `/pattern/flags` is honoured; anything else is a case-insensitive pattern. None when invalid.
pub fn compile_regex(src: &str) -> Option<regex::Regex> {
    let (body, flags) = match src.strip_prefix('/').and_then(|r| r.rsplit_once('/')) {
        Some((b, f)) if f.chars().all(|c| "gimsuy".contains(c)) => (b.to_string(), f.to_string()),
        _ => (src.to_string(), "i".to_string()),
    };
    regex::RegexBuilder::new(&body)
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'))
        .build()
        .ok()
}

const TEXT_FIELDS: &[&str] = &["name", "filename", "ext", "relpath", "fullpath"];

fn complete(c: &Condition) -> bool {
    if c.value.trim().is_empty() {
        return c.field == "relpath" && (c.op == "is" || c.op == "isnot");
    }
    true
}

fn test(c: &Condition, f: &Facts) -> bool {
    if TEXT_FIELDS.contains(&c.field.as_str()) {
        let actual = match c.field.as_str() {
            "name" => &f.name,
            "filename" => &f.filename,
            "ext" => &f.ext,
            "relpath" => &f.relpath,
            _ => &f.fullpath,
        };
        let a = actual.to_lowercase();
        let e = c.value.to_lowercase();
        return match c.op.as_str() {
            "contains" => a.contains(&e),
            "notcontains" => !a.contains(&e),
            "is" => a == e,
            "isnot" => a != e,
            "startswith" => a.starts_with(&e),
            "endswith" => a.ends_with(&e),
            "regex" => compile_regex(&c.value).map(|r| r.is_match(actual)).unwrap_or(false),
            "notregex" => compile_regex(&c.value).map(|r| !r.is_match(actual)).unwrap_or(false),
            _ => false,
        };
    }
    let (actual, expected) = match c.field.as_str() {
        "size" => (Some(f.size as f64), parse_size(&c.value)),
        "width" => (f.width.map(|v| v as f64), c.value.trim().parse().ok()),
        "height" => (f.height.map(|v| v as f64), c.value.trim().parse().ok()),
        "ageDays" => (Some(f.age_days), c.value.trim().parse().ok()),
        _ => (None, None),
    };
    let (Some(a), Some(e)) = (actual, expected) else { return false };
    match c.op.as_str() {
        "gt" => a > e,
        "gte" => a >= e,
        "lt" => a < e,
        "lte" => a <= e,
        "eq" => (a - e).abs() < 1e-9,
        _ => false,
    }
}

pub fn rule_matches(rule: &Rule, f: &Facts) -> bool {
    let usable: Vec<&Condition> = rule.conditions.iter().filter(|c| complete(c)).collect();
    if usable.is_empty() {
        return false;
    }
    if rule.matching == "any" {
        usable.iter().any(|c| test(c, f))
    } else {
        usable.iter().all(|c| test(c, f))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    pub skip: bool,
    pub folder: Option<i64>,
    pub tags: Vec<String>,
    pub rating: i64,
    pub note: String,
}

/// Rules run top to bottom: a later match replaces the destination; tags add up.
pub fn decide(w: &Watch, f: &Facts) -> Outcome {
    let mut o = Outcome { folder: w.folder, tags: w.tags.clone(), ..Default::default() };
    for r in w.rules.iter().filter(|r| r.enabled) {
        if !rule_matches(r, f) {
            continue;
        }
        if r.skip {
            o.skip = true;
            return o;
        }
        if r.folder.is_some() {
            o.folder = r.folder;
        }
        for t in &r.tags {
            if !o.tags.contains(t) {
                o.tags.push(t.clone());
            }
        }
        if r.rating > 0 {
            o.rating = r.rating;
        }
        if !r.note.is_empty() {
            o.note = r.note.clone();
        }
        if r.stop {
            break;
        }
    }
    o
}

// ------------------------------------------------------------------ file selection

/// `*` within a name, `**` across folders, `?` one character; case-insensitive.
fn glob_regex(pattern: &str) -> Option<regex::Regex> {
    let mut re = String::from("(?i)^");
    let mut it = pattern.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '*' if it.peek() == Some(&'*') => {
                it.next();
                re.push_str(".*");
            }
            '*' => re.push_str("[^/]*"),
            '?' => re.push_str("[^/]"),
            c => re.push_str(&regex::escape(&c.to_string())),
        }
    }
    re.push('$');
    regex::Regex::new(&re).ok()
}

fn list_of(s: &str) -> Vec<String> {
    s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

pub fn selected(w: &Watch, f: &Facts) -> bool {
    let exts = list_of(&w.extensions);
    if !exts.is_empty() && !exts.iter().any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(&f.ext)) {
        return false;
    }
    let rel = if f.relpath.is_empty() { f.filename.clone() } else { format!("{}/{}", f.relpath, f.filename) };
    !list_of(&w.ignore).iter().filter_map(|p| glob_regex(p)).any(|re| re.is_match(&f.filename) || re.is_match(&rel))
}

fn walk(dir: &Path, recursive: bool, out: &mut Vec<(PathBuf, std::fs::Metadata)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Ok(md) = e.metadata() else { continue };
        if md.is_dir() {
            if recursive && !dir_is_skipped(&name) {
                walk(&e.path(), true, out);
            }
        } else if md.is_file() && (!file_is_junk(&name) || is_sidecar(&name)) {
            out.push((e.path(), md));
        }
    }
}

// ------------------------------------------------------------------ approval (per computer)

/// Changing any of these needs a new yes.
pub fn fingerprint(w: &Watch) -> String {
    let s = format!("{}|{}|{}|{}|{}|{}", w.id, w.path.trim(), w.subfolders, w.extensions, w.ignore, w.after);
    blake3::hash(s.as_bytes()).to_hex().to_string()
}

fn approvals_file() -> PathBuf {
    crate::core::system::app_dirs::config_dir().join("auto-import-approvals.json")
}

pub fn load_approvals() -> HashSet<String> {
    std::fs::read_to_string(approvals_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn allow(prints: &[String]) {
    let mut set = load_approvals();
    set.extend(prints.iter().cloned());
    let p = approvals_file();
    if let Err(e) = crate::core::system::filesystem::write_whole(&p, &serde_json::to_vec(&set).unwrap_or_default()) {
        crate::core::system::logging::warn(&format!("Couldn't keep the approved scripts: {e}"));
    }
}

// ------------------------------------------------------------------ running

fn file_hash(p: &Path) -> Option<blake3::Hash> {
    let mut f = std::fs::File::open(p).ok()?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            return Some(h.finalize());
        }
        h.update(&buf[..n]);
    }
}

fn mtime_ms(md: &std::fs::Metadata) -> i64 {
    md.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Another program still writing it holds it open; Windows refuses us then.
pub fn free_to_read(p: &Path) -> bool {
    if cfg!(windows) {
        std::fs::OpenOptions::new().write(true).open(p).is_ok()
    } else {
        true
    }
}

/// The folder for a file under `base`, one level per part of `rel`, made if missing.
fn mirror(db: &Db, base: Option<i64>, rel: &str) -> DbResult<Option<i64>> {
    let mut cur = base;
    for part in rel.split('/').filter(|p| !p.is_empty()) {
        cur = Some(match db.child_folder(cur, part)? {
            Some(id) => id,
            None => db.create_folder(part, cur)?,
        });
    }
    Ok(cur)
}

/// Deal with the original now that its copy is safely in the library.
fn after_import(w: &Watch, file: &Path, root: &Path, md: &std::fs::Metadata, copy: Option<&Path>) -> Result<bool, String> {
    match w.after.as_str() {
        "keep" => return Ok(false),
        "delete" => {
            // still the version imported, and the copy really is the same bytes
            let now = std::fs::metadata(file).map_err(|e| e.to_string())?;
            if now.len() != md.len() || mtime_ms(&now) != mtime_ms(md) {
                return Err("changed since it was imported, so it was kept".into());
            }
            let copy = copy.ok_or("the library's copy couldn't be found, so the original was kept")?;
            let original = file_hash(file);
            if original.is_none() || original != file_hash(copy) {
                return Err("the library's copy doesn't match, so the original was kept".into());
            }
            filesystem::to_recycle_bin(file)?;
        }
        _ => {
            let rel = file.strip_prefix(root).map_err(|e| e.to_string())?;
            let dest = root.join(AFTER_FOLDER).join(rel);
            if let Some(d) = dest.parent() {
                std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            let mut target = dest.clone();
            let mut n = 1;
            while target.exists() {
                let stem = filesystem::stem_of(&dest);
                let ext = filesystem::ext_of(&dest);
                let name = if ext.is_empty() { format!("{stem} ({n})") } else { format!("{stem} ({n}).{ext}") };
                target = dest.with_file_name(name);
                n += 1;
            }
            std::fs::rename(file, &target).map_err(|e| e.to_string())?;
        }
    }
    // a source subfolder left empty goes too; the watched folder itself stays
    let mut dir = file.parent().map(Path::to_path_buf);
    while let Some(d) = dir {
        if d == root || std::fs::remove_dir(&d).is_err() {
            break;
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    Ok(true)
}

#[derive(Debug, Default)]
pub struct RunResult {
    pub ids: Vec<AssetId>,
    pub messages: Vec<String>,
}

/// As `run`, with the rules grouping files into one item: the files a rule gives an imported one come with it, whatever the watch takes, and go the way of its original.
pub fn run_with(db: &mut Db, paths: &LibraryPaths, cfg: &AutoImportConfig, approved: &HashSet<String>, style: crate::core::store::SuffixStyle, now: i64, rules: &[crate::core::grouping::companions::Rule]) -> DbResult<RunResult> {
    let mut out = RunResult::default();
    if !cfg.enabled {
        return Ok(out);
    }
    for w in cfg.watches.iter().filter(|w| w.enabled) {
        if !approved.contains(&fingerprint(w)) || watch_problem(&w.path, &paths.root).is_some() {
            continue;
        }
        if let Err(e) = run_watch(db, paths, w, style, now, rules, &mut out) {
            out.messages.push(format!("{}: {e}", w.path.trim()));
        }
    }
    Ok(out)
}

/// Where a watched file stands: seen whole for long enough, still settling, or imported before and left in place.
#[derive(PartialEq)]
enum Settled {
    Yes,
    NotYet,
    Imported,
}

fn settled(db: &Db, file: &Path, md: &std::fs::Metadata, now: i64) -> Settled {
    let state: Option<(i64, i64, i64, i64)> = db.conn.query_row("SELECT size, mtime, stable_since, imported FROM auto_import_state WHERE path = ?", [file.to_string_lossy()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).ok();
    match state {
        Some((s, m, _, 1)) if s == md.len() as i64 && m == mtime_ms(md) => Settled::Imported,
        Some((s, m, since, 0)) if s == md.len() as i64 && m == mtime_ms(md) && now - since >= STABLE_MS => Settled::Yes,
        _ => Settled::NotYet,
    }
}

fn run_watch(db: &mut Db, paths: &LibraryPaths, w: &Watch, style: crate::core::store::SuffixStyle, now: i64, rules: &[crate::core::grouping::companions::Rule], out: &mut RunResult) -> DbResult<()> {
    let root = PathBuf::from(w.path.trim());
    let needs_dims = w.rules.iter().any(|r| r.conditions.iter().any(|c| c.field == "width" || c.field == "height"));
    let mut files = Vec::new();
    walk(&root, w.subfolders != "off", &mut files);
    let seen: HashMap<PathBuf, std::fs::Metadata> = files.iter().cloned().collect();
    // the pairs the rules make in a folder, planned when one of its files is about to come in
    let mut plans: HashMap<PathBuf, Vec<(PathBuf, PathBuf, crate::core::grouping::companions::Pair)>> = HashMap::new();
    let mut plan_of = |dir: &Path| -> Vec<(PathBuf, PathBuf, crate::core::grouping::companions::Pair)> {
        if rules.is_empty() {
            return vec![];
        }
        plans.entry(dir.to_path_buf()).or_insert_with(|| crate::core::grouping::companions::plan_copies(&seen.keys().filter(|f| f.parent() == Some(dir)).cloned().collect::<Vec<_>>(), rules)).clone()
    };
    // whether the watch takes this file in, now or once it has settled
    let taken = |file: &Path, md: &std::fs::Metadata| {
        let facts = facts_of(file, &root, md.len() as i64, (now - mtime_ms(md)).max(0) as f64 / 86_400_000.0, needs_dims);
        selected(w, &facts) && !decide(w, &facts).skip
    };
    for (file, md) in files {
        let size = md.len() as i64;
        if size == 0 {
            continue;
        }
        let mtime = mtime_ms(&md);
        let key = file.to_string_lossy().to_string();
        let state: Option<(i64, i64, i64, i64)> = db
            .conn
            .query_row("SELECT size, mtime, stable_since, imported FROM auto_import_state WHERE path = ?", [&key], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .ok();
        match state {
            Some((s, m, _, 1)) if s == size && m == mtime => continue, // imported once, left in place
            Some((s, m, since, 0)) if s == size && m == mtime => {
                if now - since < STABLE_MS {
                    continue;
                }
            }
            _ => {
                db.conn
                    .execute("INSERT INTO auto_import_state(path, size, mtime, stable_since, imported) VALUES(?, ?, ?, ?, 0) ON CONFLICT(path) DO UPDATE SET size = excluded.size, mtime = excluded.mtime, stable_since = excluded.stable_since, imported = excluded.imported", rusqlite::params![key, size, mtime, now])
                    .map_err(|e| e.to_string())?;
                continue;
            }
        }
        // a file a rule gives another in its folder comes in with that one, when the watch takes that one
        let plan = plan_of(file.parent().unwrap_or(&root));
        if is_sidecar(&filesystem::name_of(&file)) || plan.iter().any(|(item, member, _)| *member == file && seen.get(item).is_some_and(|imd| settled(db, item, imd, now) != Settled::Imported && taken(item, imd))) {
            continue;
        }
        let age = (now - mtime).max(0) as f64 / 86_400_000.0;
        let facts = facts_of(&file, &root, size, age, needs_dims);
        if !selected(w, &facts) || !free_to_read(&file) {
            continue;
        }
        let o = decide(w, &facts);
        if o.skip {
            continue;
        }
        // the files a rule gives this one come with it: one still being written holds it back, one imported before stays where it is
        let mut bring = vec![];
        let mut wait = false;
        for (_, member, pair) in plan.iter().filter(|(item, _, _)| *item == file) {
            // an empty one is never imported, so it holds nothing back
            let Some(mmd) = seen.get(member).filter(|m| m.len() > 0) else { continue };
            match settled(db, member, mmd, now) {
                Settled::Imported => {}
                Settled::Yes if free_to_read(member) => bring.push((member.clone(), pair.clone(), mmd.clone())),
                _ => wait = true,
            }
        }
        if wait {
            continue;
        }
        let folder = match w.subfolders.as_str() {
            "match" => {
                let top = facts.relpath.split('/').next().unwrap_or("");
                if top.is_empty() || db.child_folder(o.folder, top)?.is_none() {
                    continue; // nothing to match: left where it is
                }
                mirror(db, o.folder, &facts.relpath)?
            }
            "mirror" => mirror(db, o.folder, &facts.relpath)?,
            _ => o.folder,
        };
        // one file that can't be read mustn't hold back the rest
        let ids = match import_managed(db, paths, std::slice::from_ref(&file), folder, crate::core::store::OnClash::Keep, style) {
            Ok(ids) => ids,
            Err(e) => {
                out.messages.push(format!("{}: {e}", facts.filename));
                continue;
            }
        };
        // marked imported with its metadata in one go: nothing that goes wrong afterwards can import it twice
        db.tx(|db| {
            db.conn.execute("UPDATE auto_import_state SET imported = 1 WHERE path = ?", [&key]).map_err(|e| e.to_string())?;
            if let Some(f) = folder {
                db.add_to_folder(&ids, f)?;
            }
            if !o.tags.is_empty() {
                db.add_tags(&ids, &o.tags)?;
            }
            if o.rating > 0 {
                db.set_rating(&ids, o.rating)?;
            }
            if !o.note.is_empty() {
                db.set_annotation(&ids, &o.note)?;
            }
            Ok(())
        })?;
        let copy = ids.first().and_then(|id| db.abs_path(*id));
        let done = match db.require_writable_path(&file) {
            Err(why) if w.after != "keep" => Err(format!("the original was kept: {why}")),
            _ => after_import(w, &file, &root, &md, copy.as_deref()),
        };
        match done {
            Ok(true) => {
                let _ = db.conn.execute("DELETE FROM auto_import_state WHERE path = ?", [&key]);
            }
            Ok(false) => {}
            Err(e) => out.messages.push(format!("{}: {e}", facts.filename)),
        }
        if let Some(item) = ids.first().copied() {
            for (member, pair, mmd) in bring {
                let name = filesystem::name_of(&member);
                let mkey = member.to_string_lossy().to_string();
                let mid = match import_managed(db, paths, std::slice::from_ref(&member), folder, crate::core::store::OnClash::Keep, style) {
                    Ok(m) => m.first().copied(),
                    Err(e) => {
                        out.messages.push(format!("{name}: {e}"));
                        continue;
                    }
                };
                let Some(mid) = mid else { continue };
                db.conn.execute("UPDATE auto_import_state SET imported = 1 WHERE path = ?", [&mkey]).map_err(|e| e.to_string())?;
                // one that can't join stays an item of its own
                if let Err(e) = db.group(item, mid, &pair.role, &pair.rule, pair.suffix.as_deref(), pair.cover) {
                    out.messages.push(format!("{name}: {e}"));
                    out.ids.push(mid);
                }
                let copy = db.abs_path(mid);
                let done = match db.require_writable_path(&member) {
                    Err(why) if w.after != "keep" => Err(format!("the original was kept: {why}")),
                    _ => after_import(w, &member, &root, &mmd, copy.as_deref()),
                };
                match done {
                    Ok(true) => {
                        let _ = db.conn.execute("DELETE FROM auto_import_state WHERE path = ?", [&mkey]);
                    }
                    Ok(false) => {}
                    Err(e) => out.messages.push(format!("{name}: {e}")),
                }
            }
        }
        out.ids.extend(ids);
    }
    Ok(())
}
