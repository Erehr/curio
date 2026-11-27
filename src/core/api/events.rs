//! What happens in Curio, for callers that follow along: `POST /api/v1/events` streams events to scripts and extensions (see docs/api/connect/events.md).
//! Each process keeps a `Hub` of its own library's events. The process answering on the API's port gathers them from every process (`Gather`) and adds what it sees itself: libraries opening and closing, and the library list changing.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::{bounded, Receiver, RecvTimeoutError, Sender};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{holders, known, library_list, may_reach, resolve, ApiError, Caller, Host, Perm};
use crate::core::library::{changes::Changed, LibEvent, changes::Parts};
use crate::core::model::{now_ms, AssetId};

/// Every event, the permission a caller needs to receive it, what it says, and its data as a TypeScript type (see `reference::TYPES`).
pub const EVENTS: &[(&str, Perm, &str, &str)] = &[
    ("app.themeChanged", Perm::LibraryRead, "A window shows another theme, or the same theme changed. `window` and what app.theme answers.", "Theme & { window: string }"),
    ("library.listChanged", Perm::LibraryRead, "The library list changed: a library added, removed, renamed, moved or given another look. `libraries`: the new list, as library.list gives it.", "{ libraries: Library[] }"),
    ("library.opened", Perm::LibraryRead, "A library opened in a window or in the background, or its background process showed its window. `name`, `state` (window or background).", "{ name: string; state: 'window' | 'background' }"),
    ("library.closed", Perm::LibraryRead, "No process holds the library any more. `name`.", "{ name: string }"),
    ("items.added", Perm::MetadataRead, "New items, at most 1,000 to an event. `ids`, `how`: window, api, autoImport, scan, duplicate or restore; `requestId` of the API add that made them, where it had one.", "{ ids: string[]; how: 'window' | 'api' | 'autoImport' | 'scan' | 'duplicate' | 'restore'; requestId?: string }"),
    ("items.changed", Perm::MetadataRead, "Items changed, at most 1,000 to an event. `ids`, `what`: metadata, file, thumbnail or palette.", "{ ids: string[]; what: 'metadata' | 'file' | 'thumbnail' | 'palette' }"),
    ("items.removed", Perm::MetadataRead, "Items moved to the trash, or deleted for good, at most 1,000 to an event. `ids`, `purged`.", "{ ids: string[]; purged: boolean }"),
    ("folders.changed", Perm::MetadataRead, "Folders were made, renamed, moved, deleted or given another look; read folder.list again.", "{}"),
    ("smartFolders.changed", Perm::MetadataRead, "Smart folders changed; read smartFolder.list again.", "{}"),
    ("tags.changed", Perm::MetadataRead, "Tags were made, renamed, merged, deleted, coloured or grouped; read tag.list again.", "{}"),
    ("tagGroups.changed", Perm::MetadataRead, "Tag groups changed; read tagGroup.list again.", "{}"),
    ("fields.changed", Perm::MetadataRead, "Custom fields were defined, deleted, or switched on or off; read field.list again.", "{}"),
    ("locations.changed", Perm::MetadataRead, "Locations were added, removed, changed or scanned, or went offline; read location.list again.", "{}"),
    ("scan.progress", Perm::LibraryRead, "A location's scan under way: once as it starts, with `files` 0, then about once a second. `location`, `files` so far.", "{ location: number; files: number }"),
    ("scan.done", Perm::LibraryRead, "A location's scan finished. `location`, `files`, `added`, `changed`, `moved`, `missing`, `offline`.", "{ location: number; files: number; added: number; changed: number; moved: number; missing: number; offline: number }"),
    ("library.switched", Perm::LibraryRead, "A window switched to another library. `window`, `from` and `to`: the libraries' ids.", "{ window: string; from: string; to: string }"),
    ("selection.changed", Perm::Ui, "The selection in a window changed. `window`, `count`, `ids`: the first 1000.", "{ window: string; count: number; ids: string[] }"),
    ("view.changed", Perm::Ui, "A window shows something else: another folder, tag or view, another search or other filters, or its font samples another text. `window` and what view.get answers.", "View & { window: string }"),
    ("viewer.opened", Perm::Ui, "A window's viewer shows an item, also when stepping to the next one or to another file grouped into it. `window`, `id`, `file`: the file shown, the item's own or one grouped into it.", "{ window: string; id: string; file: string }"),
    ("job.started", Perm::LibraryRead, "A plugin's job started: what job.get answers.", "Job"),
    ("job.progress", Perm::LibraryRead, "A job got further. `job`, `plugin`, `done`, `total` (0 when not known), `text`.", "{ job: string; plugin: string | null; done: number; total: number; text: string }"),
    ("job.output", Perm::LibraryRead, "Lines a job's program printed, gathered a few times a second. `job`, `plugin`, `pipe` (stdout or stderr), `lines`.", "{ job: string; plugin: string | null; pipe: 'stdout' | 'stderr'; lines: string[] }"),
    ("job.finished", Perm::LibraryRead, "A job ended: done, failed or cancelled. What job.get answers.", "Job"),
    ("viewer.closed", Perm::Ui, "A window's viewer closed. `window`, `id`: the item it showed last.", "{ window: string; id: string }"),
];

/// One thing that happened.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// `<at>-<source>-<n>`: unique, and in order within one process.
    pub id: String,
    pub name: String,
    /// The library's id; none for the library list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library: Option<String>,
    /// When, in milliseconds since 1970.
    pub at: i64,
    pub data: Value,
}

/// Where a subscription starts.
#[derive(Clone, Debug, PartialEq)]
pub enum Since {
    Now,
    /// Every event still kept.
    Kept,
    /// The events after this one: a reconnecting client's last event id.
    After(String),
}

impl Since {
    pub fn after(last: Option<&str>) -> Since {
        match last.map(str::trim).filter(|s| !s.is_empty()) {
            Some(id) => Since::After(id.to_string()),
            None => Since::Now,
        }
    }

    /// Events older than this were seen before, or happened before the subscriber came.
    pub fn floor(&self) -> i64 {
        match self {
            Since::Now => now_ms(),
            Since::Kept => 0,
            Since::After(id) => at_of(id),
        }
    }
}

fn at_of(id: &str) -> i64 {
    id.split('-').next().and_then(|t| t.parse().ok()).unwrap_or(0)
}

/// Kept for clients that reconnect, and for the router that gathers a process's events late.
const KEEP_FOR_MS: i64 = 120_000;
const KEEP_AT_MOST: usize = 2000;
/// A subscriber this far behind is dropped; it reconnects and catches up from what is kept.
const QUEUE: usize = 4096;

/// One process's events, handed to each subscriber.
pub struct Hub {
    source: String,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    n: u64,
    kept: VecDeque<Arc<Event>>,
    subs: Vec<Sender<Arc<Event>>>,
}

impl Default for Hub {
    fn default() -> Self {
        Hub { source: uuid::Uuid::new_v4().simple().to_string()[..8].to_string(), inner: Default::default() }
    }
}

impl Hub {
    pub fn publish(&self, name: &str, library: Option<&str>, data: Value) {
        let at = now_ms();
        let n = {
            let mut i = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            i.n += 1;
            i.n
        };
        self.put(Arc::new(Event { id: format!("{at}-{}-{n}", self.source), name: name.into(), library: library.map(String::from), at, data }));
    }

    /// Passes an event on as it is.
    pub fn put(&self, e: Arc<Event>) {
        let mut i = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        i.kept.push_back(e.clone());
        let old = e.at - KEEP_FOR_MS;
        while i.kept.len() > KEEP_AT_MOST || i.kept.front().is_some_and(|k| k.at < old) {
            i.kept.pop_front();
        }
        i.subs.retain(|s| s.try_send(e.clone()).is_ok());
    }

    pub fn subscribe(&self, since: &Since) -> Receiver<Arc<Event>> {
        let (tx, rx) = bounded(QUEUE);
        let mut i = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let replay: Vec<Arc<Event>> = match since {
            Since::Now => vec![],
            Since::Kept => i.kept.iter().cloned().collect(),
            Since::After(last) => match i.kept.iter().position(|k| &k.id == last) {
                Some(p) => i.kept.iter().skip(p + 1).cloned().collect(),
                // not kept here, as after a router change: everything from that moment on, which may repeat a few
                None => {
                    let at = at_of(last);
                    i.kept.iter().filter(|k| k.at >= at && &k.id != last).cloned().collect()
                }
            },
        };
        for e in replay {
            let _ = tx.try_send(e);
        }
        i.subs.push(tx);
        rx
    }
}

/// The events a library change makes.
/// Most items one `items.*` event names: a scan of a large location is told in several, never one payload of it all.
pub const EVENT_IDS_MAX: usize = 1000;

pub fn of_changed(c: &Changed, uid: &HashMap<AssetId, String>) -> Vec<(&'static str, Value)> {
    let named = |ids: &[AssetId]| ids.iter().filter_map(|i| uid.get(i)).collect::<Vec<_>>();
    let mut out = Vec::new();
    for added in named(&c.added).chunks(EVENT_IDS_MAX) {
        let mut e = json!({"ids": added, "how": if c.how.is_empty() { "window" } else { c.how }});
        if let Some(r) = &c.request {
            e["requestId"] = json!(r);
        }
        out.push(("items.added", e));
    }
    for changed in named(&c.ids).chunks(EVENT_IDS_MAX) {
        out.push(("items.changed", json!({"ids": changed, "what": c.aspect})));
    }
    for removed in named(&c.removed).chunks(EVENT_IDS_MAX) {
        out.push(("items.removed", json!({"ids": removed, "purged": c.purged})));
    }
    for (p, name) in Parts::EVENTS {
        if c.parts.has(p) {
            out.push((name, json!({})));
        }
    }
    out
}

/// Thumbnails are gathered for this long into one `items.changed`.
pub const THUMBS_EVERY: Duration = Duration::from_millis(500);

/// Reads items' ids as the API names them, from the library a relay tells about.
pub type Names = Box<dyn FnMut(&[AssetId]) -> Vec<(AssetId, String)>>;

/// Turns a library's own events into the API's, published on a hub.
pub struct Relay {
    hub: Arc<Hub>,
    library: String,
    names: Names,
    thumbs: Vec<AssetId>,
    since: Option<Instant>,
}

impl Relay {
    pub fn new(hub: Arc<Hub>, library: String, names: Names) -> Relay {
        Relay { hub, library, names, thumbs: vec![], since: None }
    }

    fn lib(&self) -> Option<&str> {
        Some(self.library.as_str()).filter(|l| !l.is_empty())
    }

    pub fn take(&mut self, ev: &LibEvent) {
        match ev {
            // told for good by the scan's own change when it ends
            LibEvent::Changed(c) if c.provisional => {}
            LibEvent::Changed(c) => {
                let mut uid: HashMap<AssetId, String> = c.uids.iter().cloned().collect();
                let want: Vec<AssetId> = c.ids.iter().chain(&c.added).chain(&c.removed).copied().filter(|i| !uid.contains_key(i)).collect();
                if !want.is_empty() {
                    uid.extend((self.names)(&want));
                }
                for (name, data) in of_changed(c, &uid) {
                    self.hub.publish(name, self.lib(), data);
                }
            }
            LibEvent::ThumbReady(id) => {
                self.thumbs.push(*id);
                self.since.get_or_insert_with(Instant::now);
            }
            LibEvent::Scan(n) => match &n.done {
                None => self.hub.publish("scan.progress", self.lib(), json!({"location": n.location, "files": n.files})),
                Some(s) => self.hub.publish("scan.done", self.lib(), json!({"location": n.location, "files": s.seen, "added": s.added, "changed": s.updated, "moved": s.moved, "missing": s.missing, "offline": s.offline})),
            },
            _ => {}
        }
        if self.since.is_some_and(|s| s.elapsed() >= THUMBS_EVERY) {
            self.flush();
        }
    }

    pub fn flush(&mut self) {
        self.since = None;
        if self.thumbs.is_empty() {
            return;
        }
        let ids: Vec<String> = (self.names)(&std::mem::take(&mut self.thumbs)).into_iter().map(|n| n.1).collect();
        if !ids.is_empty() {
            self.hub.publish("items.changed", self.lib(), json!({"ids": ids, "what": "thumbnail"}));
        }
    }
}

/// Publishes a library's events until its channel closes or `also` says to stop; `also` sees every event first.
/// How often `relay` gives `also` a chance to send what it holds while no event comes.
pub const RELAY_TICK: Duration = Duration::from_millis(100);

/// Publishes the library's events, and hands each to `also` first, with `None` on every quiet `RELAY_TICK`; `also` answering false ends it.
pub fn relay(rx: Receiver<LibEvent>, hub: Arc<Hub>, library: String, names: impl FnMut(&[AssetId]) -> Vec<(AssetId, String)> + 'static, mut also: impl FnMut(Option<&LibEvent>) -> bool) {
    let mut r = Relay::new(hub, library, Box::new(names));
    loop {
        match rx.recv_timeout(RELAY_TICK) {
            Ok(ev) => {
                if !also(Some(&ev)) {
                    break;
                }
                r.take(&ev);
            }
            Err(RecvTimeoutError::Timeout) => {
                if !also(None) {
                    break;
                }
                if r.since.is_some_and(|s| s.elapsed() >= THUMBS_EVERY) {
                    r.flush();
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    r.flush();
}

/// What a subscriber asks for.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    /// Event names, or a namespace as `items.*`; none for every event the caller may receive.
    #[serde(default)]
    pub events: Vec<String>,
    /// Only this library's events (the library list's still come).
    #[serde(default)]
    pub library: Option<String>,
    /// Only these items' events, by their ids; needs `library`.
    #[serde(default)]
    pub ids: Vec<String>,
    /// Where to start: the last event id received, as the `Last-Event-ID` header gives it.
    #[serde(default)]
    pub after: Option<String>,
}

/// A checked filter, for one caller.
pub struct Subscription {
    caller: Caller,
    names: HashSet<&'static str>,
    library: Option<String>,
    ids: HashSet<String>,
}

/// Checks what a caller asks to follow, as `handle` checks a call: unknown names and missing permissions are refused, not skipped.
pub fn subscription(host: &dyn Host, caller: &Caller, f: &Filter) -> Result<Subscription, ApiError> {
    let mut names = HashSet::new();
    for want in &f.events {
        let matched: Vec<&(&'static str, Perm, &str, &str)> = match want.strip_suffix(".*") {
            Some(ns) => EVENTS.iter().filter(|e| e.0.split('.').next() == Some(ns)).collect(),
            None => EVENTS.iter().filter(|e| e.0 == want).collect(),
        };
        if matched.is_empty() {
            return Err(ApiError::new("bad_request", format!("There is no event {want}")));
        }
        for e in matched {
            if !caller.may(e.1) {
                return Err(ApiError::new("not_allowed", format!("{} needs the {} permission, which {} doesn't have", e.0, e.1.name(), caller.name)));
            }
            names.insert(e.0);
        }
    }
    if f.events.is_empty() {
        names = EVENTS.iter().filter(|e| caller.may(e.1)).map(|e| e.0).collect();
        if names.is_empty() {
            return Err(ApiError::new("not_allowed", format!("Events need the library.read, metadata.read or ui permission, which {} doesn't have", caller.name)));
        }
    }
    let library = match &f.library {
        Some(w) => {
            let l = resolve(&known(host, &host.config()), w)?;
            may_reach(caller, &l)?;
            Some(l.id.ok_or_else(|| ApiError::new("unknown_library", "That library hasn't been open on this computer yet, so its events can't be told apart"))?)
        }
        None => None,
    };
    if !f.ids.is_empty() && library.is_none() {
        return Err(ApiError::new("bad_request", "`ids` are numbers within one library: give `library` too"));
    }
    Ok(Subscription { caller: caller.clone(), names, library, ids: f.ids.iter().cloned().collect() })
}

impl Subscription {
    /// What this subscriber receives of an event, if anything.
    pub fn deliver(&self, host: &dyn Host, e: &Event) -> Option<Value> {
        if !self.names.contains(e.name.as_str()) {
            return None;
        }
        // a job's lines and result are its plugin's own
        if e.name.starts_with("job.") && self.caller.plugin.is_some() && e.data["plugin"].as_str() != self.caller.plugin.as_deref() {
            return None;
        }
        if let Some(lib) = &e.library {
            if !self.caller.libraries.is_empty() && !self.caller.libraries.contains(lib) {
                return None;
            }
            if self.library.as_ref().is_some_and(|l| l != lib) {
                return None;
            }
        }
        let mut data = e.data.clone();
        if e.name == "library.listChanged" {
            data = json!({"libraries": library_list(host, &self.caller)});
        }
        if !self.ids.is_empty() && e.name.starts_with("items.") {
            let ids: Vec<Value> = data["ids"].as_array().into_iter().flatten().filter(|i| i.as_str().is_some_and(|i| self.ids.contains(i))).cloned().collect();
            if ids.is_empty() {
                return None;
            }
            data["ids"] = json!(ids);
        }
        Some(json!({"id": e.id, "name": e.name, "library": e.library, "at": e.at, "data": data}))
    }
}

/// Written while a stream has nothing to send, so a closed connection is noticed.
pub const HEARTBEAT: Duration = Duration::from_secs(15);

/// A process's own events for the router, one JSON line each, over its control channel: `{"after": id}` resumes after an event, `{"kept": true}` starts with every event kept.
pub fn serve_lines(hub: &Hub, body: &Value, req: tiny_http::Request) {
    let since = match body["after"].as_str() {
        Some(id) if !id.is_empty() => Since::After(id.into()),
        _ if body["kept"] == true => Since::Kept,
        _ => Since::Now,
    };
    let rx = hub.subscribe(&since);
    let mut out = req.into_writer();
    if out.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n").and_then(|_| out.flush()).is_err() {
        return;
    }
    loop {
        let line = match rx.recv_timeout(HEARTBEAT) {
            Ok(e) => serde_json::to_string(&*e).unwrap_or_default(),
            Err(RecvTimeoutError::Timeout) => String::new(),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        if writeln!(out, "{line}").and_then(|_| out.flush()).is_err() {
            return;
        }
    }
}

/// How often the router looks at who holds which library, and at the library list.
const LOOK_EVERY: Duration = Duration::from_secs(1);

/// Every process's events, gathered by the process answering on the API's port from its first subscriber on.
pub struct Gather {
    pub all: Hub,
    host: Arc<dyn Host>,
    stop: Arc<AtomicBool>,
    started: std::sync::Once,
    /// Processes whose events come in, by control port and token, with the last event each sent.
    sources: Mutex<HashMap<(u16, String), Source>>,
    wake: (Sender<()>, Receiver<()>),
}

#[derive(Default)]
struct Source {
    live: bool,
    last: Option<String>,
}

impl Gather {
    pub fn new(host: Arc<dyn Host>, stop: Arc<AtomicBool>) -> Arc<Gather> {
        Arc::new(Gather { all: Hub::default(), host, stop, started: std::sync::Once::new(), sources: Default::default(), wake: bounded(1) })
    }

    pub fn subscribe(self: &Arc<Self>, since: &Since) -> Receiver<Arc<Event>> {
        let rx = self.all.subscribe(since);
        let me = self.clone();
        self.started.call_once(move || {
            let _ = std::thread::Builder::new().name("api-events".into()).spawn(move || me.run());
        });
        rx
    }

    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    fn run(self: Arc<Self>) {
        if let Some(own) = self.host.events() {
            let me = self.clone();
            let rx = own.subscribe(&Since::Kept);
            let _ = std::thread::Builder::new().name("api-events-own".into()).spawn(move || {
                while !me.stopped() {
                    match rx.recv_timeout(HEARTBEAT) {
                        Ok(e) => me.all.put(e),
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            });
        }
        let mut list_was: Option<String> = None;
        let mut held_was: Option<HashMap<String, (String, &'static str)>> = None;
        while !self.stopped() {
            let cfg = self.host.config();
            let list = known(self.host.as_ref(), &cfg);
            let sig = serde_json::to_string(&list.iter().map(|l| json!([l.path, l.id, l.icon, l.color])).collect::<Vec<_>>()).unwrap_or_default();
            if list_was.as_ref().is_some_and(|w| *w != sig) {
                self.all.publish("library.listChanged", None, json!({}));
            }
            list_was = Some(sig);
            let held = holders(self.host.as_ref(), &list);
            let states: HashMap<String, (String, &'static str)> = held.iter().filter_map(|(l, h)| Some((l.id.clone()?, (crate::core::library::instance::library_name(&l.path), if h.background { "background" } else { "window" })))).collect();
            if let Some(was) = &held_was {
                for (id, (name, state)) in &states {
                    if was.get(id).map(|w| w.1) != Some(*state) {
                        self.all.publish("library.opened", Some(id), json!({"name": name, "state": state}));
                    }
                }
                for (id, (name, _)) in was.iter().filter(|(id, _)| !states.contains_key(*id)) {
                    self.all.publish("library.closed", Some(id), json!({"name": name}));
                }
            }
            held_was = Some(states);
            let me = self.host.me();
            for (_, h) in held {
                if h.port == me.port && h.token == me.token {
                    continue;
                }
                let key = (h.port, h.token.clone());
                let last = {
                    let mut s = self.sources.lock().unwrap_or_else(|e| e.into_inner());
                    let src = s.entry(key.clone()).or_default();
                    if src.live {
                        continue;
                    }
                    src.live = true;
                    src.last.clone()
                };
                let me = self.clone();
                let _ = std::thread::Builder::new().name("api-events-from".into()).spawn(move || me.read_from(key, last));
            }
            let _ = self.wake.1.recv_timeout(LOOK_EVERY);
        }
    }

    /// Takes in one process's events until it closes its stream.
    fn read_from(self: Arc<Self>, key: (u16, String), last: Option<String>) {
        let body = match &last {
            Some(id) => json!({"after": id}),
            None => json!({"kept": true}),
        };
        let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(5)).timeout_read(HEARTBEAT * 3).build();
        if let Ok(resp) = agent.post(&format!("http://127.0.0.1:{}/events", key.0)).set("X-Curio-Token", &key.1).send_string(&body.to_string()) {
            for line in std::io::BufReader::new(resp.into_reader()).lines() {
                let Ok(line) = line else { break };
                if self.stopped() {
                    break;
                }
                let Ok(e) = serde_json::from_str::<Event>(&line) else { continue };
                if let Some(src) = self.sources.lock().unwrap_or_else(|e| e.into_inner()).get_mut(&key) {
                    src.last = Some(e.id.clone());
                }
                self.all.put(Arc::new(e));
            }
        }
        if let Some(src) = self.sources.lock().unwrap_or_else(|e| e.into_inner()).get_mut(&key) {
            src.live = false;
        }
        let _ = self.wake.0.try_send(());
    }
}
