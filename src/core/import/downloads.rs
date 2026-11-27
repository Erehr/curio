//! Curio's own downloads: one connection per file, streamed to disk, resumed with `Range`, outside the library's job so its writes go on meanwhile.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::api::events::Hub;
use crate::core::api::{jobs, ApiError};
use crate::core::library::thumbnailing::Poster;
use crate::core::system::logging;
use crate::core::model::now_ms;

const MAX_HOPS: usize = 5;
/// Free space a download leaves on its drive.
const MARGIN: u64 = 512 << 20;
/// How often a download of unknown size looks at the free space again.
const ROOM_EVERY: u64 = 64 << 20;

/// What is asked for one address. `cookies` and `user_agent` live only in memory.
#[derive(Clone, Debug, Default)]
pub struct Ask {
    pub url: String,
    pub referer: Option<String>,
    pub cookies: Option<String>,
    pub user_agent: Option<String>,
}

/// How far a file got, kept between tries and across a restart.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Partial {
    pub received: u64,
    #[serde(default)]
    pub total: Option<u64>,
    /// The first answer's `ETag`, else its `Last-Modified`; a resume is asked only with one.
    #[serde(default)]
    pub validator: Option<String>,
    #[serde(default)]
    pub ranges: bool,
    #[serde(default)]
    pub name: Option<String>,
    /// The address the file is coming from: the one asked for, or an alternate of it.
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum Got {
    /// The file is complete; its name.
    File(String),
    /// The address answered with a page, and was asked as one that might be.
    Page,
}

#[derive(Debug, PartialEq)]
pub enum Failed {
    Status(u16),
    /// The connection failed or broke off; worth another try.
    Net(String),
    Cancelled,
    NoRoom,
    /// Not an address Curio fetches.
    Refused(String),
}

impl Failed {
    /// Whether another try may succeed.
    pub fn passing(&self) -> bool {
        match self {
            Failed::Net(_) => true,
            Failed::Status(s) => matches!(s, 408 | 425 | 429 | 500 | 502 | 503 | 504),
            _ => false,
        }
    }

    /// In words for a person, never a raw resolver or TLS error.
    pub fn words(&self) -> String {
        match self {
            Failed::Status(401 | 403) => "Access denied. Send it again from the browser.".into(),
            Failed::Status(404 | 410) => "Not found".into(),
            Failed::Status(429) => "Rate limited. Try again later.".into(),
            Failed::Status(s) if *s >= 500 => format!("Server error ({s})"),
            Failed::Status(s) => format!("HTTP error {s}"),
            Failed::Net(e) => net_words(e),
            Failed::Cancelled => "Cancelled".into(),
            Failed::NoRoom => "Not enough disk space".into(),
            Failed::Refused(e) => e.clone(),
        }
    }
}

fn net_words(e: &str) -> String {
    let l = e.to_ascii_lowercase();
    if l.contains("not a public") {
        "Local network addresses are blocked".into()
    } else if l.contains("dns") || l.contains("resolve") || l.contains("no such host") || l.contains("failed to lookup") {
        "Site not found".into()
    } else if l.contains("timed out") || l.contains("timeout") {
        "Timed out".into()
    } else if l.contains("broke off") || l.contains("redirect") {
        e.to_string()
    } else {
        "Connection failed".into()
    }
}

fn host_of(url: &str) -> Option<(String, String)> {
    url::Url::parse(url).ok().map(|u| (u.scheme().to_string(), u.host_str().unwrap_or("").to_ascii_lowercase()))
}

/// The ask for an alternate of `first`: the cookies were given for `first`'s host, so another host's alternate goes without them.
fn alternate(ask: &Ask, first: &str, alt: &str) -> Ask {
    let cookies = ask.cookies.clone().filter(|_| host_of(alt) == host_of(first));
    Ask { url: alt.to_string(), cookies, ..ask.clone() }
}

/// One request, redirects followed by hand: `Referer` goes with every hop, `Cookie` only while the host and scheme are still the first ones.
fn open(agent: &ureq::Agent, ask: &Ask, range: Option<(u64, &str)>) -> Result<(ureq::Response, String), Failed> {
    let mut url = ask.url.clone();
    let first = host_of(&url);
    let mut cookies = ask.cookies.as_deref();
    for _ in 0..=MAX_HOPS {
        crate::core::import::links::check_web_url(&url).map_err(Failed::Refused)?;
        if host_of(&url) != first {
            cookies = None;
        }
        let mut req = agent.get(&url).set("Accept-Encoding", "identity");
        if let Some(r) = ask.referer.as_deref().filter(|r| crate::core::import::links::referer_ok(r)) {
            req = req.set("Referer", r);
        }
        if let Some(ua) = ask.user_agent.as_deref() {
            req = req.set("User-Agent", ua);
        }
        if let Some(c) = cookies {
            req = req.set("Cookie", c);
        }
        if let Some((from, validator)) = range {
            req = req.set("Range", &format!("bytes={from}-")).set("If-Range", validator);
        }
        let resp = match req.call() {
            Ok(r) => r,
            Err(ureq::Error::Status(_, r)) => r,
            // no such host, or not one on the internet: another try would end the same
            Err(ureq::Error::Transport(t)) if t.kind() == ureq::ErrorKind::Dns => return Err(Failed::Refused(net_words(&t.to_string()))),
            Err(e) => return Err(Failed::Net(e.to_string())),
        };
        if matches!(resp.status(), 301 | 302 | 303 | 307 | 308) {
            let loc = resp.header("location").unwrap_or("").to_string();
            url = url::Url::parse(&url).and_then(|b| b.join(&loc)).map(|u| u.to_string()).map_err(|_| Failed::Net(format!("Invalid redirect: {loc}")))?;
            continue;
        }
        return Ok((resp, url));
    }
    Err(Failed::Net(format!("Too many redirects (over {MAX_HOPS})")))
}

/// The start of a `Content-Range: bytes <start>-<end>/<total>` answer, and its total when known.
fn content_range(h: &str) -> Option<(u64, Option<u64>)> {
    let rest = h.trim().strip_prefix("bytes ")?;
    let (span, total) = rest.split_once('/')?;
    let start = span.split_once('-')?.0.trim().parse().ok()?;
    Some((start, total.trim().parse().ok()))
}

fn room(dir: &Path, need: u64) -> Result<(), Failed> {
    match crate::core::system::filesystem::free_space(dir) {
        Some(free) if need.saturating_add(MARGIN) > free => Err(Failed::NoRoom),
        _ => Ok(()),
    }
}

/// Fetches `ask` into `file`, going on from `part` when the site allows it; `sniff` keeps a page's answer as a page.
pub fn transfer(agent: &ureq::Agent, ask: &Ask, file: &Path, part: &mut Partial, sniff: bool, cancel: &AtomicBool, progress: &mut dyn FnMut(&Partial)) -> Result<Got, Failed> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Failed::Cancelled);
    }
    // only this download writes the file, front to back, so its length is what arrived
    let on_disk = std::fs::metadata(file).map(|m| m.len()).unwrap_or(0);
    let resume = on_disk > 0 && part.ranges && part.validator.is_some() && part.url.as_deref().is_none_or(|u| u == ask.url);
    if resume {
        part.received = on_disk;
    } else {
        *part = Partial::default();
    }
    let (resp, url) = open(agent, ask, resume.then(|| (part.received, part.validator.as_deref().unwrap_or(""))))?;
    let status = resp.status();
    let append = match status {
        206 if resume => match resp.header("content-range").and_then(content_range) {
            Some((start, total)) if start == part.received => {
                if total.is_some() {
                    part.total = total;
                }
                true
            }
            _ => {
                *part = Partial::default();
                return Err(Failed::Net("Resume failed; restarting".into()));
            }
        },
        // the file changed or the site ignored the range: the whole file comes again
        200..=299 if status != 206 => false,
        416 => {
            *part = Partial::default();
            return Err(Failed::Net("Resume failed; restarting".into()));
        }
        s => return Err(Failed::Status(s)),
    };
    if !append {
        let ctype = resp.header("content-type").unwrap_or("").to_string();
        let disposition = resp.header("content-disposition").map(str::to_string);
        let attachment = disposition.as_deref().is_some_and(|d| d.trim().to_ascii_lowercase().starts_with("attachment"));
        // bytes of no stated kind are a download, never a page
        let binary = ctype.split(';').next().unwrap_or("").trim().eq_ignore_ascii_case("application/octet-stream");
        if sniff && !crate::core::import::links::is_file_type(&ctype) && !attachment && !binary {
            return Ok(Got::Page);
        }
        *part = Partial {
            received: 0,
            total: resp.header("content-length").and_then(|l| l.trim().parse().ok()),
            validator: resp.header("etag").or_else(|| resp.header("last-modified")).map(str::to_string),
            ranges: resp.header("accept-ranges").is_some_and(|r| r.trim().eq_ignore_ascii_case("bytes")),
            name: Some(crate::core::import::links::file_name(&url, disposition.as_deref().and_then(crate::core::import::links::disposition_name), &ctype)),
            url: Some(ask.url.clone()),
        };
    }
    let dir = file.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| Failed::Refused(e.to_string()))?;
    if let Some(t) = part.total {
        room(dir, t.saturating_sub(part.received))?;
    }
    let mut out = std::fs::OpenOptions::new().create(true).write(true).append(append).truncate(!append).open(file).map_err(|e| Failed::Refused(e.to_string()))?;
    let mut body = resp.into_reader();
    let mut buf = vec![0u8; 64 << 10];
    let mut since_room = 0u64;
    progress(part);
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(Failed::Cancelled);
        }
        let n = match body.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => return Err(Failed::Net(e.to_string())),
        };
        out.write_all(&buf[..n]).map_err(|e| Failed::Refused(e.to_string()))?;
        part.received += n as u64;
        since_room += n as u64;
        if since_room >= ROOM_EVERY {
            since_room = 0;
            room(dir, 0)?;
        }
        progress(part);
    }
    out.flush().map_err(|e| Failed::Refused(e.to_string()))?;
    if part.total.is_some_and(|t| t != part.received) {
        return Err(Failed::Net("Connection lost".into()));
    }
    Ok(Got::File(part.name.clone().unwrap_or_else(|| crate::core::import::links::file_name(&url, None, ""))))
}

// ------------------------------------------------------------------ the queue

/// How many files download at once in one library; never two from one host.
const AT_ONCE: usize = 3;
/// Tries after the first, for a failure that may pass.
const TRIES: u32 = 3;
const KEEP_MS: i64 = 24 * 3600 * 1000;
const KEEP_FINISHED: usize = 200;
const PROGRESS_EVERY: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Waiting,
    Running,
    /// Complete on disk and handed to the library's job.
    Importing,
    Done,
    Failed,
    Cancelled,
}

/// One address on its way into the library, as the queue file keeps it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub alternates: Vec<String>,
    /// Where the item goes and what it gets, as `Placing` takes it.
    pub place: Value,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub referer: Option<String>,
    /// `auto`, `file` or `link`.
    pub save_as: String,
    pub on_duplicate: String,
    /// `window` or `api`, for `items.added`.
    pub how: String,
    pub state: State,
    #[serde(default)]
    pub part: Partial,
    #[serde(default)]
    pub tries: u32,
    #[serde(default)]
    pub error: Option<String>,
    pub added: i64,
    #[serde(default)]
    pub ended: Option<i64>,
    /// The file it is written to, in the library's `downloading` folder.
    pub file: String,
    /// The address the file came from, once complete: this one or an alternate.
    #[serde(default)]
    pub from: Option<String>,
    /// The caller's `requestId`, for the events of what it adds.
    #[serde(default)]
    pub request_id: Option<String>,
    /// A hash of who asked with which `requestId`, so a repeat after a restart adds nothing.
    #[serde(default)]
    pub request_key: Option<String>,
    /// What it added, failed or found, once finished: `{ ids, names, errors, duplicates, folder }`.
    #[serde(default)]
    pub answer: Option<Value>,
    /// What fails of it, or was there already, is told in Curio's notice: asked from outside Curio, where nothing else would say so.
    #[serde(default)]
    pub notify: bool,
}

impl Record {
    fn open(&self) -> bool {
        matches!(self.state, State::Waiting | State::Running | State::Importing)
    }

    fn title(&self) -> String {
        self.name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| {
            let path = self.url.split(['?', '#']).next().unwrap_or(&self.url);
            path.trim_end_matches('/').rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(&self.url).to_string()
        })
    }
}

/// How a download ended, for the library's job to import.
pub enum Outcome {
    File { path: PathBuf, name: String, url: String },
    Link,
}

/// One address to add.
pub struct New {
    pub ask: Ask,
    pub alternates: Vec<String>,
    pub place: Value,
    pub name: Option<String>,
    pub save_as: &'static str,
    pub on_duplicate: String,
    pub how: &'static str,
    pub request_id: Option<String>,
    pub request_key: Option<String>,
    pub notify: bool,
}

/// Hears a call's answer once every address it gave has ended.
pub type Answer = Box<dyn FnOnce(Result<Value, ApiError>) + Send>;

/// The records of one call, answered and told of together.
struct Batch {
    pieces: Vec<Option<Value>>,
    records: Vec<String>,
    answer: Option<Answer>,
    notify: bool,
}

/// What a record has only in memory.
struct Live {
    cookies: Option<String>,
    user_agent: Option<String>,
    batch: Option<(Arc<Mutex<Batch>>, usize)>,
    job: String,
    cancel: Arc<AtomicBool>,
}

#[derive(Default, Serialize, Deserialize)]
struct Saved {
    library: PathBuf,
    records: Vec<Record>,
}

struct Inner {
    records: Vec<Record>,
    live: HashMap<String, Live>,
    hosts: Vec<String>,
}

/// One library's downloads: kept in a file of the app's data, since they are this computer's (its cookies, its partial files) and never the library's, run here, and imported by the library's job one file at a time.
pub struct Queue {
    library: PathBuf,
    file: PathBuf,
    dir: PathBuf,
    post: Poster,
    hub: Option<Arc<Hub>>,
    inner: Mutex<Inner>,
    closed: AtomicBool,
}

/// The queue file's name for a library: its folder's name and a hash of its path, as a shared cache folder names it.
fn key_of(root: &Path) -> String {
    let name: String = crate::core::system::filesystem::name_of(root).chars().map(|c| if c.is_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect();
    let id = blake3::hash(crate::core::system::filesystem::canonical(root).to_string_lossy().to_lowercase().as_bytes()).to_hex();
    format!("{name}-{}", &id[..8])
}

/// Where the app keeps the queue files.
pub fn queue_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("downloads")
}

/// The libraries whose queue file has downloads not yet finished.
pub fn unfinished(data_dir: &Path) -> Vec<PathBuf> {
    let Ok(list) = std::fs::read_dir(queue_dir(data_dir)) else { return vec![] };
    list.flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| serde_json::from_slice::<Saved>(&std::fs::read(e.path()).ok()?).ok())
        .filter(|s| s.records.iter().any(Record::open))
        .map(|s| s.library)
        .collect()
}

fn host_key(url: &str) -> String {
    url::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_ascii_lowercase)).unwrap_or_default()
}

/// A given name with the downloaded file's extension.
fn named(given: Option<&str>, fetched: &str) -> String {
    let Some(g) = given.map(str::trim).filter(|g| !g.is_empty()) else { return fetched.to_string() };
    let ext = crate::core::system::filesystem::ext_of(Path::new(fetched));
    if ext.is_empty() || crate::core::system::filesystem::ext_of(Path::new(g)).eq_ignore_ascii_case(&ext) {
        return crate::core::import::sanitize_name(g);
    }
    crate::core::import::sanitize_name(&format!("{g}.{ext}"))
}

fn failed_piece(url: &str, e: &str) -> Value {
    json!({"ids": [], "errors": [format!("{url}: {e}")]})
}

/// What a person cancelled is told of to nobody.
fn told(pieces: &[(Value, Option<String>)]) -> Vec<(Value, Option<String>)> {
    pieces.iter().filter(|(p, _)| p["cancelled"].as_bool() != Some(true)).cloned().collect()
}

/// The answer of several pieces, in the order they were asked: failed when nothing was added and something went wrong.
pub fn merge(pieces: &[Value]) -> Result<Value, ApiError> {
    let (mut ids, mut errors, mut dups) = (Vec::new(), Vec::new(), Vec::new());
    for p in pieces {
        ids.extend(p["ids"].as_array().cloned().unwrap_or_default());
        errors.extend(p["errors"].as_array().cloned().unwrap_or_default());
        dups.extend(p["duplicates"].as_array().cloned().unwrap_or_default());
    }
    if ids.is_empty() && !errors.is_empty() {
        let text: Vec<String> = errors.iter().filter_map(|e| e.as_str().map(String::from)).collect();
        return Err(ApiError::new("failed", text.join("; ")));
    }
    let mut v = json!({"success": true, "ids": ids, "errors": errors});
    if !dups.is_empty() {
        v["duplicates"] = json!(dups);
    }
    Ok(v)
}

impl Queue {
    /// Reads the library's queue file and goes on with what it left unfinished, without the cookies it had.
    pub fn open(data_dir: &Path, paths: &crate::core::library::paths::LibraryPaths, post: Poster, hub: Option<Arc<Hub>>) -> Arc<Queue> {
        let key = key_of(&paths.root);
        let file = queue_dir(data_dir).join(format!("{key}.json"));
        // beside the store, so a finished file goes in with a rename
        let dir = paths.root.join("downloading");
        let saved: Saved = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let mut records = saved.records;
        let mut live = HashMap::new();
        let mut import = Vec::new();
        for r in records.iter_mut().filter(|r| r.state == State::Importing) {
            // the import took the file: it is in the library, though its answer was lost
            if !dir.join(&r.file).is_file() || r.save_as == "link" {
                r.state = State::Done;
                r.ended = Some(now_ms());
                r.answer.get_or_insert_with(|| json!({"ids": [], "errors": []}));
            } else {
                import.push(r.id.clone());
            }
        }
        for r in records.iter_mut().filter(|r| r.open()) {
            if r.state != State::Importing {
                r.state = State::Waiting;
                // the file holds what reached the disk before the process ended
                r.part.received = std::fs::metadata(dir.join(&r.file)).map(|m| m.len()).unwrap_or(0);
            }
            let (job, cancel) = jobs::start(hub.clone(), None, &r.title(), r.part.total.unwrap_or(0) as f64);
            live.insert(r.id.clone(), Live { cookies: None, user_agent: None, batch: None, job, cancel });
        }
        if let Ok(list) = std::fs::read_dir(&dir) {
            for e in list.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if !records.iter().any(|r| r.open() && r.file == name) {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        let q = Arc::new(Queue { library: paths.root.clone(), file, dir, post, hub, inner: Mutex::new(Inner { records, live, hosts: vec![] }), closed: AtomicBool::new(false) });
        for id in import {
            if let Some(r) = q.record(&id) {
                let outcome = Outcome::File { path: q.dir.join(&r.file), name: named(r.name.as_deref(), r.part.name.as_deref().unwrap_or("download")), url: r.from.clone().unwrap_or_else(|| r.url.clone()) };
                q.import(r, outcome);
            }
        }
        q.pump();
        q
    }

    /// Queues `news`; `answer` hears once all of them ended, and so does Curio's notice when they asked for one.
    pub fn add(self: &Arc<Self>, news: Vec<New>, answer: Option<Answer>) {
        let ids: Vec<String> = news.iter().map(|_| format!("dl-{}", uuid::Uuid::new_v4().simple())).collect();
        let notify = news.iter().any(|n| n.notify);
        let batch = (answer.is_some() || notify).then(|| Arc::new(Mutex::new(Batch { pieces: vec![None; news.len()], records: ids.clone(), answer, notify })));
        {
            let mut inner = self.inner.lock();
            for (i, (n, id)) in news.into_iter().zip(ids).enumerate() {
                let r = Record {
                    file: format!("{id}.part"),
                    from: None,
                    id: id.clone(),
                    url: n.ask.url,
                    alternates: n.alternates,
                    place: n.place,
                    name: n.name,
                    referer: n.ask.referer,
                    save_as: n.save_as.into(),
                    on_duplicate: n.on_duplicate,
                    how: n.how.into(),
                    state: State::Waiting,
                    part: Partial::default(),
                    tries: 0,
                    error: None,
                    added: now_ms(),
                    ended: None,
                    request_id: n.request_id,
                    request_key: n.request_key,
                    answer: None,
                    notify: n.notify,
                };
                let (job, cancel) = jobs::start(self.hub.clone(), None, &r.title(), 0.0);
                let _ = jobs::progress(&job, None, None, None, Some(&host_key(&r.url)));
                inner.live.insert(id, Live { cookies: n.ask.cookies, user_agent: n.ask.user_agent, batch: batch.clone().map(|b| (b, i)), job, cancel });
                inner.records.push(r);
            }
            self.save(&mut inner);
        }
        self.pump();
    }

    /// The answer kept for a `requestId` seen before a restart: `queued` while any of its downloads runs.
    pub fn repeat(&self, request_key: &str) -> Option<Value> {
        let inner = self.inner.lock();
        let mine: Vec<&Record> = inner.records.iter().filter(|r| r.request_key.as_deref() == Some(request_key)).collect();
        if mine.is_empty() {
            return None;
        }
        let mut v = if mine.iter().any(|r| r.open()) {
            json!({"success": true, "queued": true})
        } else {
            let pieces: Vec<Value> = mine.iter().filter_map(|r| r.answer.clone()).collect();
            // an add that failed whole frees its id for a retry
            merge(&pieces).ok()?
        };
        v["repeat"] = json!(true);
        Some(v)
    }

    /// Sends finished downloads again, without the cookies they had; `keep` adds them even when the library holds the same file.
    pub fn again(self: &Arc<Self>, ids: &[String], keep: bool) -> usize {
        let news: Vec<New> = {
            let inner = self.inner.lock();
            inner
                .records
                .iter()
                .filter(|r| ids.contains(&r.id) && !r.open())
                .map(|r| New {
                    ask: Ask { url: r.url.clone(), referer: r.referer.clone(), cookies: None, user_agent: None },
                    alternates: r.alternates.clone(),
                    place: r.place.clone(),
                    name: r.name.clone(),
                    save_as: if r.save_as == "link" { "link" } else if r.save_as == "file" { "file" } else { "auto" },
                    on_duplicate: if keep { "keep".into() } else { r.on_duplicate.clone() },
                    how: if r.how == "window" { "window" } else { "api" },
                    request_id: None,
                    request_key: None,
                    notify: true,
                })
                .collect()
        };
        let n = news.len();
        if n > 0 {
            self.add(news, None);
        }
        n
    }

    /// Downloads waiting or running, which keep a background holder open.
    pub fn busy(&self) -> bool {
        self.inner.lock().records.iter().any(Record::open)
    }

    /// Stops the transfers for the library's closing; they go on when it opens again.
    pub fn close(&self) {
        if self.closed.swap(true, Ordering::Relaxed) {
            return;
        }
        let mut inner = self.inner.lock();
        for l in inner.live.values() {
            l.cancel.store(true, Ordering::Relaxed);
            let _ = jobs::finish(&l.job, None, None, Some("Paused until the library reopens".into()));
        }
        for r in inner.records.iter_mut().filter(|r| r.state == State::Running) {
            r.state = State::Waiting;
        }
        self.save(&mut inner);
    }

    /// Marks the record as importing before the library's job takes the file, so a process ending in between neither loses it nor adds it twice.
    fn import(self: &Arc<Self>, mut rec: Record, outcome: Outcome) {
        {
            let mut inner = self.inner.lock();
            if let Some(r) = inner.records.iter_mut().find(|r| r.id == rec.id) {
                r.state = State::Importing;
                if let Outcome::File { url, name, .. } = &outcome {
                    r.from = Some(url.clone());
                    r.part.name.get_or_insert_with(|| name.clone());
                }
                rec = r.clone();
            }
            self.save(&mut inner);
        }
        let q = self.clone();
        self.post.job(move |db, ctx| {
            let (piece, changed) = crate::core::api::operations::item_adds::import_download(db, ctx, &rec, outcome);
            let failed = piece["ids"].as_array().is_some_and(|a| a.is_empty()) && piece["errors"].as_array().is_some_and(|a| !a.is_empty());
            q.finish(&rec.id, piece, if failed { State::Failed } else { State::Done });
            q.post.kick_thumbs();
            Ok(changed)
        });
    }

    /// Writes the queue file whole, through a temporary file; finished records stay a day, at most 200 of them.
    fn save(&self, inner: &mut Inner) {
        let now = now_ms();
        inner.records.retain(|r| r.open() || r.ended.is_some_and(|e| now - e < KEEP_MS));
        let mut surplus = inner.records.iter().filter(|r| !r.open()).count().saturating_sub(KEEP_FINISHED);
        inner.records.retain(|r| {
            let drop = surplus > 0 && !r.open();
            surplus -= drop as usize;
            !drop
        });
        let body = serde_json::to_vec_pretty(&json!({"library": self.library, "records": inner.records})).unwrap_or_default();
        let tmp = self.file.with_extension(format!("json.{}.tmp", std::process::id()));
        let res = self.file.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&tmp, &body)).and_then(|_| std::fs::rename(&tmp, &self.file));
        if let Err(e) = res {
            let _ = std::fs::remove_file(&tmp);
            logging::warn(&format!("couldn't save the downloads of {}: {e}", self.library.display()));
        }
    }

    /// Starts what may start: links at once, files up to `AT_ONCE`, one per host.
    fn pump(self: &Arc<Self>) {
        if self.closed.load(Ordering::Relaxed) {
            return;
        }
        let mut cancelled = Vec::new();
        let mut start = Vec::new();
        {
            let mut inner = self.inner.lock();
            let mut running = inner.records.iter().filter(|r| r.state == State::Running && r.save_as != "link").count();
            let ids: Vec<String> = inner.records.iter().filter(|r| r.state == State::Waiting).map(|r| r.id.clone()).collect();
            for id in ids {
                if inner.live.get(&id).is_some_and(|l| l.cancel.load(Ordering::Relaxed)) {
                    cancelled.push(id);
                    continue;
                }
                let Some(r) = inner.records.iter().find(|r| r.id == id) else { continue };
                let (link, host) = (r.save_as == "link", host_key(&r.url));
                if !link && (running >= AT_ONCE || inner.hosts.contains(&host)) {
                    continue;
                }
                if !link {
                    running += 1;
                    inner.hosts.push(host);
                }
                if let Some(r) = inner.records.iter_mut().find(|r| r.id == id) {
                    r.state = State::Running;
                }
                start.push(id);
            }
        }
        for id in cancelled {
            let url = self.record(&id).map(|r| r.url).unwrap_or_default();
            let mut piece = failed_piece(&url, &Failed::Cancelled.words());
            piece["cancelled"] = json!(true);
            self.finish(&id, piece, State::Cancelled);
        }
        for id in start {
            let q = self.clone();
            let _ = std::thread::Builder::new().name("download".into()).spawn(move || q.run(&id));
        }
    }

    fn run(self: &Arc<Self>, id: &str) {
        let snapshot = {
            let inner = self.inner.lock();
            inner.records.iter().find(|r| r.id == id).cloned().zip(inner.live.get(id).map(|l| (l.cookies.clone(), l.user_agent.clone(), l.job.clone(), l.cancel.clone())))
        };
        let Some((rec, (cookies, user_agent, job, cancel))) = snapshot else { return };
        let outcome = if rec.save_as == "link" { Ok(Outcome::Link) } else { self.fetch(&rec, Ask { url: rec.url.clone(), referer: rec.referer.clone(), cookies, user_agent }, &job, &cancel) };
        if self.closed.load(Ordering::Relaxed) {
            return;
        }
        match outcome {
            Err(e) => {
                let _ = std::fs::remove_file(self.dir.join(&rec.file));
                let state = if cancel.load(Ordering::Relaxed) { State::Cancelled } else { State::Failed };
                let mut piece = failed_piece(&rec.url, &e);
                if state == State::Cancelled {
                    piece["cancelled"] = json!(true);
                }
                self.finish(id, piece, state);
            }
            Ok(outcome) => self.import(self.record(id).unwrap_or(rec), outcome),
        }
    }

    fn record(&self, id: &str) -> Option<Record> {
        self.inner.lock().records.iter().find(|r| r.id == id).cloned()
    }

    /// The file, from an alternate address first, tried again while a failure may pass; the error in words.
    fn fetch(&self, rec: &Record, ask: Ask, job: &str, cancel: &AtomicBool) -> Result<Outcome, String> {
        let agent = crate::core::system::net::download_agent();
        let file = self.dir.join(&rec.file);
        let mut last = Instant::now() - PROGRESS_EVERY;
        let mut kept: Option<String> = None;
        let mut progress = |p: &Partial| {
            // once per address, as its answer's head comes: what a restart needs to go on with this file
            if p.url != kept {
                kept = p.url.clone();
                self.keep(&rec.id, p);
            }
            if last.elapsed() >= PROGRESS_EVERY || p.total == Some(p.received) {
                last = Instant::now();
                let _ = jobs::progress(job, None, Some(p.received as f64), p.total.map(|t| t as f64), None);
            }
        };
        let mut part = rec.part.clone();
        // a file begun before a restart goes on from the alternate it was coming from
        let begun = part.url.clone().filter(|u| part.received > 0 && rec.alternates.contains(u));
        let alternates: Vec<&String> = match &begun {
            Some(u) => vec![u],
            None if part.received == 0 => rec.alternates.iter().collect(),
            None => vec![],
        };
        for alt in alternates {
            let a = alternate(&ask, &rec.url, alt);
            match transfer(&agent, &a, &file, &mut part, true, cancel, &mut progress) {
                Ok(Got::File(name)) => return Ok(Outcome::File { path: file, name: named(rec.name.as_deref(), &name), url: alt.clone() }),
                Err(Failed::Cancelled) => return Err(Failed::Cancelled.words()),
                _ => part = Partial::default(),
            }
        }
        let sniff = rec.save_as == "auto" && crate::core::import::links::is_page_url(&rec.url);
        let mut tries = rec.tries;
        loop {
            match transfer(&agent, &ask, &file, &mut part, sniff, cancel, &mut progress) {
                Ok(Got::File(name)) => return Ok(Outcome::File { path: file, name: named(rec.name.as_deref(), &name), url: rec.url.clone() }),
                Ok(Got::Page) => return Ok(Outcome::Link),
                // whatever goes wrong while asking what a page address holds leaves it a link
                Err(f) if sniff && f != Failed::Cancelled => return Ok(Outcome::Link),
                Err(f) if f.passing() && tries < TRIES && !cancel.load(Ordering::Relaxed) && !self.closed.load(Ordering::Relaxed) => {
                    tries += 1;
                    self.note(&rec.id, &part, tries, &f.words());
                    let until = Instant::now() + Duration::from_secs([2, 5, 10][(tries as usize - 1).min(2)]);
                    while Instant::now() < until && !cancel.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
                Err(f) => {
                    self.note(&rec.id, &part, tries, &f.words());
                    return Err(f.words());
                }
            }
        }
    }

    /// Keeps what a download's answer said of its file, so a restart resumes it.
    fn keep(&self, id: &str, part: &Partial) {
        let mut inner = self.inner.lock();
        if let Some(r) = inner.records.iter_mut().find(|r| r.id == id) {
            r.part = part.clone();
        }
        self.save(&mut inner);
    }

    /// Keeps how far a download got and why its last try failed.
    fn note(&self, id: &str, part: &Partial, tries: u32, error: &str) {
        let mut inner = self.inner.lock();
        if let Some(r) = inner.records.iter_mut().find(|r| r.id == id) {
            r.part = part.clone();
            r.tries = tries;
            r.error = Some(error.to_string());
        }
        self.save(&mut inner);
    }

    /// A record ended: its job, its call's answer once all of the call's records ended, and what may start next.
    fn finish(self: &Arc<Self>, id: &str, piece: Value, state: State) {
        let error = piece["errors"].as_array().and_then(|e| e.first()).and_then(|e| e.as_str()).map(String::from);
        let live = {
            let mut inner = self.inner.lock();
            let host = inner.records.iter().find(|r| r.id == id).filter(|r| r.save_as != "link").map(|r| host_key(&r.url));
            if let Some(i) = host.and_then(|h| inner.hosts.iter().position(|x| *x == h)) {
                inner.hosts.remove(i);
            }
            if let Some(r) = inner.records.iter_mut().find(|r| r.id == id) {
                r.state = state;
                r.ended = Some(now_ms());
                r.answer = Some(piece.clone());
                if state != State::Done {
                    r.error = error.clone();
                }
            }
            let live = inner.live.remove(id);
            self.save(&mut inner);
            live
        };
        let notify = self.record(id).is_some_and(|r| r.notify);
        let Some(live) = live else { return self.pump() };
        match state {
            State::Done => {
                let _ = jobs::finish(&live.job, None, Some(piece.clone()), None);
            }
            // job.cancel ended it already
            State::Cancelled => {}
            _ => {
                let _ = jobs::finish(&live.job, None, None, error.or_else(|| Some("Failed".into())));
            }
        }
        match live.batch {
            Some((b, i)) => {
                let ready = {
                    let mut b = b.lock();
                    b.pieces[i] = Some(piece);
                    if b.pieces.iter().all(Option::is_some) {
                        let pieces: Vec<Value> = b.pieces.iter().flatten().cloned().collect();
                        let told: Vec<(Value, Option<String>)> = pieces.iter().cloned().zip(b.records.iter().cloned().map(Some)).collect();
                        Some((b.answer.take(), pieces, b.notify.then_some(told)))
                    } else {
                        None
                    }
                };
                if let Some((answer, pieces, told_of)) = ready {
                    if let Some(a) = answer {
                        a(merge(&pieces));
                    }
                    if let Some(n) = told_of.and_then(|t| crate::core::import::notices::of_rest(self.library.clone(), &told(&t))) {
                        crate::core::import::notices::post(n);
                    }
                }
            }
            // a download that went on after a restart is told of by itself
            None if notify => {
                if let Some(n) = crate::core::import::notices::of_rest(self.library.clone(), &told(&[(piece, Some(id.to_string()))])) {
                    crate::core::import::notices::post(n);
                }
            }
            None => {}
        }
        self.pump();
    }
}
