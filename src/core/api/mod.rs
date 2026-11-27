//! One set of operations for every caller: the app's own window, scripts and extensions with a token, plugins and MCP clients.
//! Each operation is defined once in the registry (`ops.rs`) with its parameters, the permission it needs and the callers that may use it.
//! Transports (`http.rs`, `mcp.rs`, the window's `api` command, the control channel between processes) only turn requests into `handle` calls.
//! See docs/api/connect/http.md.

pub mod events;
pub mod http;
pub mod jobs;
pub mod mcp;
pub(crate) mod operations;
pub mod pair;
pub mod programs;
pub mod reference;
mod requests;
pub mod tokens;
pub mod upload;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::db::Db;
use crate::core::library::instance::{self, Holder};
use crate::core::library::Library;

pub const API_VERSION: u32 = 1;
/// The address scripts, extensions and MCP clients use; the first Curio process to start answers on it.
pub const DEFAULT_PORT: u16 = 41733;
/// How long a call waits for a library that isn't open to open in the background.
const HOLDER_WAIT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Perm {
    #[serde(rename = "library.read")]
    LibraryRead,
    #[serde(rename = "metadata.read")]
    MetadataRead,
    #[serde(rename = "metadata.write")]
    MetadataWrite,
    #[serde(rename = "organize")]
    Organize,
    #[serde(rename = "files.read")]
    FilesRead,
    #[serde(rename = "files.write")]
    FilesWrite,
    /// Files elsewhere on the computer: exported to, added or indexed from a path.
    #[serde(rename = "files.outside")]
    FilesOutside,
    #[serde(rename = "library.manage")]
    LibraryManage,
    #[serde(rename = "ui")]
    Ui,
    #[serde(rename = "exec")]
    Exec,
    #[serde(rename = "network")]
    Network,
}

impl Perm {
    pub const ALL: [Perm; 11] = [Perm::LibraryRead, Perm::MetadataRead, Perm::MetadataWrite, Perm::Organize, Perm::FilesRead, Perm::FilesWrite, Perm::FilesOutside, Perm::LibraryManage, Perm::Ui, Perm::Exec, Perm::Network];
    /// `exec` and `network` belong to plugins.
    pub const FOR_TOKENS: [Perm; 9] = [Perm::LibraryRead, Perm::MetadataRead, Perm::MetadataWrite, Perm::Organize, Perm::FilesRead, Perm::FilesWrite, Perm::FilesOutside, Perm::LibraryManage, Perm::Ui];

    pub fn name(self) -> &'static str {
        match self {
            Perm::LibraryRead => "library.read",
            Perm::MetadataRead => "metadata.read",
            Perm::MetadataWrite => "metadata.write",
            Perm::Organize => "organize",
            Perm::FilesRead => "files.read",
            Perm::FilesWrite => "files.write",
            Perm::FilesOutside => "files.outside",
            Perm::LibraryManage => "library.manage",
            Perm::Ui => "ui",
            Perm::Exec => "exec",
            Perm::Network => "network",
        }
    }

    pub fn parse(s: &str) -> Option<Perm> {
        Perm::ALL.into_iter().find(|p| p.name() == s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CallerKind {
    Window,
    Web,
    Plugin,
    Mcp,
}

impl CallerKind {
    pub const ALL: &'static [CallerKind] = &[CallerKind::Window, CallerKind::Web, CallerKind::Plugin, CallerKind::Mcp];
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Caller {
    pub kind: CallerKind,
    /// The token's or plugin's name, for messages.
    pub name: String,
    pub perms: Vec<Perm>,
    /// Ids of the libraries a token may reach; empty for all.
    #[serde(default)]
    pub libraries: Vec<String>,
    /// The plugin's id, for its pages and programs: what `plugin.*` keeps is its own.
    #[serde(default)]
    pub plugin: Option<String>,
    /// The web token's id, which uploads are kept for.
    #[serde(default)]
    pub token: Option<String>,
    /// A paired browser extension's token (`pair::REFUSED`).
    #[serde(default)]
    pub paired: bool,
}

impl Caller {
    pub fn window() -> Caller {
        Caller { kind: CallerKind::Window, name: "window".into(), perms: Perm::ALL.to_vec(), libraries: vec![], plugin: None, token: None, paired: false }
    }

    pub fn from_token(t: &tokens::ApiToken, kind: CallerKind) -> Caller {
        Caller { kind, name: t.name.clone(), perms: t.known_perms(), libraries: t.libraries.clone(), plugin: None, token: Some(t.id.clone()), paired: t.paired }
    }

    /// A plugin's pages and programs, with what was granted to it.
    pub fn plugin(p: &crate::core::plugins::manifest::Plugin) -> Caller {
        Caller { kind: CallerKind::Plugin, name: p.manifest.name.clone(), perms: p.api_perms(), libraries: vec![], plugin: Some(p.manifest.id.clone()), token: None, paired: false }
    }

    pub fn may(&self, p: Perm) -> bool {
        self.perms.contains(&p)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    /// The message for the window, in sentences it can show in its own language; programs read `message` (`json`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub said: Vec<crate::core::said::Part>,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> ApiError {
        ApiError { code: code.into(), message: message.into(), said: vec![] }
    }

    /// An error the window shows in its own language.
    pub fn said(code: &str, said: crate::core::said::Said) -> ApiError {
        ApiError { code: code.into(), message: said.text(), said: said.0 }
    }

    pub fn status(&self) -> u16 {
        match self.code.as_str() {
            "bad_request" => 400,
            "unauthorized" => 401,
            "not_allowed" => 403,
            "unknown_operation" | "unknown_library" | "not_found" => 404,
            "library_closed" | "library_offline" | "no_window" => 409,
            "busy" => 503,
            "too_large" => 413,
            _ => 422,
        }
    }

    pub fn json(&self) -> Value {
        json!({"error": {"code": self.code, "message": self.message}})
    }
}

impl From<String> for ApiError {
    fn from(m: String) -> ApiError {
        ApiError::new("failed", m)
    }
}

impl From<&str> for ApiError {
    fn from(m: &str) -> ApiError {
        ApiError::new("failed", m)
    }
}

pub type ApiResult = Result<Value, ApiError>;

/// A library on this computer's list.
#[derive(Clone, Debug, Default)]
pub struct KnownLibrary {
    pub path: PathBuf,
    /// Learnt when the library was last open here.
    pub id: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

/// The app-wide settings the API reads.
#[derive(Clone, Debug, Default)]
pub struct ApiConfig {
    pub libraries: Vec<KnownLibrary>,
    pub tokens: Vec<tokens::ApiToken>,
    pub mcp_enabled: bool,
    pub mcp_disabled: Vec<String>,
    /// The library last open in a window, for calls that name none while no window has one.
    pub current: Option<PathBuf>,
}

/// How long a window has to do what a call asks, and a person to answer when asked something.
pub fn window_wait(op: &str, ask: bool) -> Duration {
    match (ask, op) {
        (true, "pair.ask") => Duration::from_secs(120),
        (true, _) => Duration::from_secs(600),
        (false, "library.switch") => Duration::from_secs(60),
        _ => Duration::from_secs(15),
    }
}

/// Calls this process is answering, and when it last answered one; a background holder ends once it has been idle a while.
#[derive(Debug)]
pub struct Activity {
    in_flight: AtomicUsize,
    last: AtomicI64,
}

impl Default for Activity {
    fn default() -> Self {
        Activity { in_flight: AtomicUsize::new(0), last: AtomicI64::new(crate::core::model::now_ms()) }
    }
}

pub struct Answering<'a>(&'a Activity);

impl Drop for Answering<'_> {
    fn drop(&mut self) {
        self.0.last.store(crate::core::model::now_ms(), Ordering::Relaxed);
        self.0.in_flight.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Activity {
    pub fn begin(&self) -> Answering<'_> {
        self.in_flight.fetch_add(1, Ordering::Relaxed);
        Answering(self)
    }

    /// `None` while a call is being answered.
    pub fn idle_for(&self) -> Option<Duration> {
        if self.in_flight.load(Ordering::Relaxed) > 0 {
            return None;
        }
        Some(Duration::from_millis((crate::core::model::now_ms() - self.last.load(Ordering::Relaxed)).max(0) as u64))
    }
}

/// A change to this computer's library list, made by the process that answers the call.
#[derive(Clone, Debug)]
pub enum LibraryChange {
    /// In a window of its own, or the window that has it comes forward.
    Open(PathBuf),
    /// A library just made at `path` joins the list.
    Add { path: PathBuf, id: String },
    Rename(PathBuf, String),
    Forget(PathBuf),
    /// Just before another library, or last.
    Place(PathBuf, Option<PathBuf>),
    /// `None` leaves the icon or colour as it is; an empty value clears it.
    Look { path: PathBuf, icon: Option<String>, color: Option<String> },
}

/// What a process offers the API: its library, its window, the app's settings.
pub trait Host: Send + Sync {
    /// The library this process holds.
    fn library(&self) -> Option<PathBuf>;
    /// This process's control channel and focus times.
    fn me(&self) -> Holder;
    /// Runs `f` on a read connection to this process's library.
    fn read(&self, f: &mut dyn FnMut(&Db)) -> Result<(), String>;
    /// Runs `f` with this process's open library.
    fn with_library(&self, f: &mut dyn FnMut(&Library)) -> Result<(), String>;
    /// The items selected in this process's window.
    fn selection(&self) -> Vec<i64>;
    /// Folders this library's window used last, newest first.
    fn recent_folders(&self) -> Vec<i64>;
    fn config(&self) -> ApiConfig;
    /// Returns the library's path afterwards.
    fn change_library(&self, change: LibraryChange) -> Result<PathBuf, String>;
    /// New libraries and the hand-over of items between libraries.
    fn data_dir(&self) -> PathBuf {
        crate::core::system::app_dirs::data_dir()
    }
    /// Counts the calls this process answers.
    fn activity(&self) -> Option<&Activity> {
        None
    }
    /// Opens a library no process holds in a process of its own without a window; the call then waits for it.
    fn start_holder(&self, library: &Path) -> Result<(), String> {
        Err(format!("{} isn't open. Open it in Curio first.", instance::library_name(library)))
    }
    /// This process's library events.
    fn events(&self) -> Option<Arc<events::Hub>> {
        None
    }
    /// Has this process's window do `op` and waits for its answer; `ask` brings the window forward and waits for the person. `None` when there is no window.
    fn window(&self, _op: &str, _args: Value, _ask: bool) -> Option<ApiResult> {
        None
    }
    /// Saves a new web token in the app's settings.
    fn add_token(&self, _token: tokens::ApiToken) -> Result<(), String> {
        Err("Tokens can't be saved from here".into())
    }
    /// Takes back a token whose secret never reached its caller.
    fn drop_token(&self, _id: &str) {}
    /// The theme this process's window shows, as `app.theme` answers.
    fn theme(&self) -> Option<Value> {
        None
    }
    /// The language Curio shows (`en`, `de`, `pt-BR`…), as `app.info` answers.
    fn language(&self) -> String {
        "en".into()
    }
    /// An address a plugin's page reads this item's file from (`item.fileUrl`); `None` for any other caller.
    fn page_file(&self, _item: crate::core::model::AssetId, _ext: &str) -> Option<String> {
        None
    }
    /// An address a plugin's page sends a file to (`item.writeUrl`), for one write; `None` for any other caller.
    fn page_write(&self, _write: PageWrite) -> Option<String> {
        None
    }
    /// Where plugins' settings on this computer are kept, one file each.
    fn plugin_settings_dir(&self) -> PathBuf {
        self.data_dir().join("plugin-settings")
    }
}

/// What a file sent to an `item.writeUrl` address becomes.
#[derive(Clone, Debug, PartialEq)]
pub struct PageWrite {
    pub item: crate::core::model::AssetId,
    /// Replaces the item's file, or becomes a new item beside it.
    pub beside: bool,
    /// The new item's file name, for `beside`.
    pub name: Option<String>,
}

/// A file sent to an `item.writeUrl` address, put where the address said, as `caller`.
pub fn finish_page_write(host: &dyn Host, caller: &Caller, w: &PageWrite, file: &Path) -> ApiResult {
    let uid = read(host, |db| Ok(db.asset(w.item)?.map(|a| a.uid)))?.ok_or_else(|| ApiError::new("not_found", "The item is gone"))?;
    // the file is the one Curio received for the address, so naming its path reaches nothing outside
    let mut lent = caller.clone();
    if !lent.perms.contains(&Perm::FilesOutside) {
        lent.perms.push(Perm::FilesOutside);
    }
    let caller = &lent;
    if w.beside {
        handle(host, caller, "item.add.beside", json!({"id": uid, "path": file, "name": w.name}))
    } else {
        handle(host, caller, "item.replaceFile", json!({"id": uid, "path": file})).map(|_| json!({"success": true, "id": uid}))
    }
}

pub(crate) fn read<T>(h: &dyn Host, f: impl FnOnce(&Db) -> Result<T, String>) -> Result<T, String> {
    let mut f = Some(f);
    let mut out = None;
    h.read(&mut |db| out = f.take().map(|f| f(db)))?;
    out.unwrap_or_else(|| Err("No library is open".into()))
}

pub(crate) fn with_library<T>(h: &dyn Host, f: impl FnOnce(&Library) -> Result<T, String>) -> Result<T, String> {
    let mut f = Some(f);
    let mut out = None;
    h.with_library(&mut |lib| out = f.take().map(|f| f(lib)))?;
    out.unwrap_or_else(|| Err("No library is open".into()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OpKind {
    Read,
    Write,
    /// Acts on what a window shows.
    Window,
}

/// What an operation acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The app as a whole.
    App,
    /// One library, in the process that holds it; callers outside the window may name it with `library`.
    Library,
    /// An entry of the library list, named with `library`; runs in the process that got the call.
    Entry,
    /// A window, named by `window` or by the library it shows; without either, the most recently focused window.
    Window,
}

pub struct Ctx<'a> {
    pub host: &'a dyn Host,
    pub caller: &'a Caller,
    /// The library named by an `Entry` operation.
    pub entry: Option<&'a KnownLibrary>,
}

type Run = Box<dyn Fn(&Ctx, Value) -> ApiResult + Send + Sync>;

pub struct Op {
    /// `namespace.method`
    pub name: &'static str,
    pub doc: &'static str,
    pub perm: Perm,
    pub kind: OpKind,
    pub target: Target,
    pub callers: &'static [CallerKind],
    pub mcp: bool,
    /// Answers with a file (see `file_reply`), sent as bytes over HTTP.
    pub bytes: bool,
    /// Only a plugin's page may call it: the page showing the frame answers it, or it needs the page's own address.
    pub pages_only: bool,
    /// JSON schema of the parameters.
    pub params: Value,
    /// The answer as a TypeScript type; the names it uses are defined in `reference::TYPES`.
    pub returns: &'static str,
    run: Run,
}

impl Op {
    /// The parameters as callers outside the window see them, `library` included where it applies.
    pub fn public_params(&self) -> Value {
        let mut p = self.params.clone();
        let doc = match self.target {
            Target::App => return p,
            Target::Library => "The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window.",
            Target::Entry => "The library: its id from library.list, its folder or its name.",
            Target::Window => "The window showing this library: the library's id from library.list, its folder or its name.",
        };
        if let Some(props) = p.get_mut("properties").and_then(|v| v.as_object_mut()) {
            props.insert("library".into(), json!({"type": "string", "description": doc}));
            if self.target == Target::Window {
                props.insert("window".into(), json!({"type": "string", "description": "The window: its id from library.list. Without it or `library`, the most recently focused window."}));
            }
        }
        if self.target == Target::Entry {
            match p.get_mut("required").and_then(|v| v.as_array_mut()) {
                Some(r) => r.push(json!("library")),
                None => p["required"] = json!(["library"]),
            }
        }
        p
    }

    pub fn mcp_name(&self) -> String {
        self.name.replace('.', "_")
    }
}

pub fn registry() -> &'static [Op] {
    static REG: OnceLock<Vec<Op>> = OnceLock::new();
    REG.get_or_init(operations::all)
}

pub fn find(name: &str) -> Option<&'static Op> {
    registry().iter().find(|o| o.name == name)
}

fn allowed(o: &Op, caller: &Caller) -> Result<(), ApiError> {
    if !o.callers.contains(&caller.kind) {
        return Err(ApiError::new("not_allowed", format!("{} isn't available here", o.name)));
    }
    if !caller.may(o.perm) {
        return Err(ApiError::new("not_allowed", format!("{} needs the {} permission, which {} doesn't have", o.name, o.perm.name(), caller.name)));
    }
    if caller.paired && pair::REFUSED.contains(&o.name) {
        return Err(ApiError::new("not_allowed", format!("A paired browser extension can't use {}", o.name)));
    }
    Ok(())
}

fn take_library(args: &mut Value) -> Result<Option<String>, ApiError> {
    let Some(obj) = args.as_object_mut() else { return Ok(None) };
    match obj.remove("library") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(Some(s.trim().to_string())),
        Some(_) => Err(ApiError::new("bad_request", "`library` is a library's id, folder or name")),
    }
}

fn take_window(args: &mut Value) -> Result<Option<String>, ApiError> {
    let Some(obj) = args.as_object_mut() else { return Ok(None) };
    match obj.remove("window") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(Some(s.trim().to_string())),
        Some(_) => Err(ApiError::new("bad_request", "`window` is a window's id from library.list")),
    }
}

/// The same folder, however its path is written.
pub fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        let s = p.to_string_lossy().replace('\\', "/");
        let s = s.trim_end_matches('/').to_string();
        if cfg!(windows) {
            s.to_lowercase()
        } else {
            s
        }
    };
    norm(a) == norm(b)
}

/// The libraries callers can reach: this computer's list, and this process's own library even when it isn't on it.
fn known(host: &dyn Host, cfg: &ApiConfig) -> Vec<KnownLibrary> {
    let mut list = cfg.libraries.clone();
    if let Some(mine) = host.library() {
        if !list.iter().any(|l| same_path(&l.path, &mine)) {
            list.push(KnownLibrary { path: mine, ..Default::default() });
        }
    }
    list
}

/// A library named by its id, its folder or its name (names must be unique).
fn resolve(list: &[KnownLibrary], wanted: &str) -> Result<KnownLibrary, ApiError> {
    if let Some(l) = list.iter().find(|l| l.id.as_deref() == Some(wanted) || same_path(&l.path, Path::new(wanted))) {
        return Ok(l.clone());
    }
    let named: Vec<&KnownLibrary> = list.iter().filter(|l| instance::library_name(&l.path).eq_ignore_ascii_case(wanted)).collect();
    match named.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(ApiError::new("unknown_library", format!("No library on this computer's list is called {wanted}"))),
        _ => Err(ApiError::new("unknown_library", format!("Several libraries are called {wanted}; use the id from library.list"))),
    }
}

/// Who holds each open library, this process included.
fn holders(host: &dyn Host, list: &[KnownLibrary]) -> Vec<(KnownLibrary, Holder)> {
    let mine = host.library();
    list.iter()
        .filter_map(|l| {
            if mine.as_deref().is_some_and(|m| same_path(m, &l.path)) {
                return Some((l.clone(), host.me()));
            }
            instance::holder(&l.path).filter(|h| h.port != 0).map(|h| (l.clone(), h))
        })
        .collect()
}

/// The library of the most recently focused window, else of the window opened last, else of a background holder, else the one last open in a window.
fn default_library(host: &dyn Host, list: &[KnownLibrary], cfg: &ApiConfig) -> Option<KnownLibrary> {
    let held = holders(host, list).into_iter().max_by_key(|(_, h)| (!h.background, h.focused, h.opened)).map(|(l, _)| l);
    held.or_else(|| cfg.current.as_ref().and_then(|c| list.iter().find(|l| same_path(&l.path, c))).filter(|l| instance::is_library(&l.path)).cloned())
}

fn may_reach(caller: &Caller, l: &KnownLibrary) -> Result<(), ApiError> {
    if caller.libraries.is_empty() || l.id.as_ref().is_some_and(|id| caller.libraries.contains(id)) {
        return Ok(());
    }
    Err(ApiError::new("not_allowed", format!("{} may not use the library {}", caller.name, instance::library_name(&l.path))))
}

/// A library on the list named by id, folder or name, that the caller may reach.
pub(crate) fn library_named(host: &dyn Host, caller: &Caller, wanted: &str) -> Result<KnownLibrary, ApiError> {
    let l = resolve(&known(host, &host.config()), wanted)?;
    may_reach(caller, &l)?;
    Ok(l)
}

/// Checks the caller, finds the library the call is for, and runs it here or in the process holding that library.
pub fn handle(host: &dyn Host, caller: &Caller, op: &str, mut args: Value) -> ApiResult {
    let o = find(op).ok_or_else(|| ApiError::new("unknown_operation", format!("There is no operation {op}")))?;
    allowed(o, caller)?;
    let _answering = host.activity().map(Activity::begin);
    if args.is_null() {
        args = json!({});
    }
    let wanted = take_library(&mut args)?;
    let window = take_window(&mut args)?;
    if window.is_some() && o.target != Target::Window {
        return Err(ApiError::new("bad_request", format!("{op} doesn't take a window")));
    }
    match o.target {
        Target::Window => {
            if caller.kind == CallerKind::Plugin && (wanted.is_some() || window.is_some()) {
                return Err(ApiError::new("not_allowed", "A plugin works in the window it was started from"));
            }
            return match window_holder(host, caller, wanted, window)? {
                None if host.me().background => Err(ApiError::new("no_window", "This library has no window open")),
                None => (o.run)(&Ctx { host, caller, entry: None }, args),
                Some(h) => forward(&h, caller, op, args).unwrap_or_else(|e| Err(ApiError::new("busy", e))),
            };
        }
        Target::App if wanted.is_some() => return Err(ApiError::new("bad_request", format!("{op} doesn't take a library"))),
        Target::App => return (o.run)(&Ctx { host, caller, entry: None }, args),
        Target::Entry => {
            let w = wanted.ok_or_else(|| ApiError::new("bad_request", format!("{op} needs `library`: a library's id from library.list, its folder or its name")))?;
            let l = library_named(host, caller, &w)?;
            return (o.run)(&Ctx { host, caller, entry: Some(&l) }, args);
        }
        Target::Library => {}
    }
    if wanted.is_some() && caller.kind == CallerKind::Plugin {
        return Err(ApiError::new("not_allowed", "A plugin works on the library of the window it was started from"));
    }
    // the window's own calls, many a second while editing, skip reading the library list
    if wanted.is_none() && matches!(caller.kind, CallerKind::Window | CallerKind::Plugin) && caller.libraries.is_empty() {
        if host.library().is_none() {
            return Err(ApiError::new("library_closed", "No library is open in this window"));
        }
        return (o.run)(&Ctx { host, caller, entry: None }, args);
    }
    let cfg = host.config();
    let list = known(host, &cfg);
    let target = match (&wanted, caller.kind) {
        (Some(w), _) => resolve(&list, w)?,
        (None, CallerKind::Window | CallerKind::Plugin) => {
            let mine = host.library().ok_or_else(|| ApiError::new("library_closed", "No library is open in this window"))?;
            list.iter().find(|l| same_path(&l.path, &mine)).cloned().unwrap_or(KnownLibrary { path: mine, ..Default::default() })
        }
        (None, _) => default_library(host, &list, &cfg).ok_or_else(|| ApiError::new("library_closed", "No library is open in Curio"))?,
    };
    may_reach(caller, &target)?;
    if host.library().as_deref().is_some_and(|m| same_path(m, &target.path)) {
        return (o.run)(&Ctx { host, caller, entry: None }, args);
    }
    let name = instance::library_name(&target.path);
    for again in [false, true] {
        let h = match instance::holder(&target.path) {
            Some(h) if h.port != 0 => h,
            Some(_) => return Err(ApiError::new("busy", format!("The window of {name} is still starting"))),
            None if !instance::is_library(&target.path) => return Err(ApiError::new("library_offline", format!("{name} can't be reached at {}", target.path.display()))),
            None => wake(host, &target.path, &name)?,
        };
        match forward(&h, caller, op, args.clone()) {
            // a background holder that was ending as the call came: the next one takes it
            Err(e) if h.background && !again => {
                crate::core::system::logging::info(&format!("{name}'s background holder didn't answer ({e}); trying again"));
                instance::wait_gone(&target.path, h.pid, HOLDER_WAIT);
            }
            Err(e) => return Err(ApiError::new("busy", e)),
            Ok(r) => return r,
        }
    }
    Err(ApiError::new("busy", format!("{name} didn't answer")))
}

/// The process whose window a window operation is for; `None` for this one. Windows are never opened for it.
fn window_holder(host: &dyn Host, caller: &Caller, library: Option<String>, window: Option<String>) -> Result<Option<Holder>, ApiError> {
    if library.is_none() && window.is_none() && matches!(caller.kind, CallerKind::Window | CallerKind::Plugin) {
        return Ok(None);
    }
    let cfg = host.config();
    let list = known(host, &cfg);
    let shown: Vec<(KnownLibrary, Holder)> = holders(host, &list).into_iter().filter(|(_, h)| !h.background && !h.window.is_empty()).collect();
    let (l, h) = match (window, library) {
        (Some(w), named) => {
            let found = shown.into_iter().find(|(_, h)| h.window == w).ok_or_else(|| ApiError::new("not_found", format!("No Curio window has the id {w}")))?;
            if let Some(n) = named {
                if !same_path(&resolve(&list, &n)?.path, &found.0.path) {
                    return Err(ApiError::new("bad_request", format!("Window {w} doesn't show {n}")));
                }
            }
            found
        }
        (None, Some(w)) => {
            let l = resolve(&list, &w)?;
            may_reach(caller, &l)?;
            let name = instance::library_name(&l.path);
            shown.into_iter().find(|(o, _)| same_path(&o.path, &l.path)).ok_or_else(|| ApiError::new("no_window", format!("No window shows {name}")))?
        }
        (None, None) => shown.into_iter().filter(|(l, _)| may_reach(caller, l).is_ok()).max_by_key(|(_, h)| (h.focused, h.opened)).ok_or_else(|| ApiError::new("no_window", "No Curio window is open"))?,
    };
    may_reach(caller, &l)?;
    let me = host.me();
    Ok(if h.port == me.port && h.token == me.token { None } else { Some(h) })
}

/// Has a library no process holds opened in the background, and waits until it answers; calls arriving meanwhile wait for the same holder.
fn wake(host: &dyn Host, path: &Path, name: &str) -> Result<Holder, ApiError> {
    static STARTING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    let first = {
        let mut s = STARTING.lock().unwrap_or_else(|e| e.into_inner());
        let first = !s.iter().any(|p| same_path(p, path));
        if first {
            s.push(path.to_path_buf());
        }
        first
    };
    let started = Instant::now();
    let out = (|| {
        if first {
            host.start_holder(path).map_err(|e| ApiError::new("library_closed", e))?;
        }
        loop {
            if let Some(h) = instance::holder(path).filter(|h| h.port != 0) {
                return Ok(h);
            }
            if started.elapsed() > HOLDER_WAIT {
                return Err(ApiError::said("busy", crate::core::said::Said::new("errors.library.stillOpening", json!({"name": name}), format!("{name} is still opening; try again shortly"))));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    })();
    if first {
        STARTING.lock().unwrap_or_else(|e| e.into_inner()).retain(|p| !same_path(p, path));
    }
    out
}

/// Hands a call to the process holding its library, over that process's control channel; the outer error is the channel's.
fn forward(h: &Holder, caller: &Caller, op: &str, args: Value) -> Result<ApiResult, String> {
    let reply = instance::tell(h, "api", &json!({"caller": caller, "op": op, "args": args}))?;
    if let Some(e) = reply.get("error") {
        return Ok(Err(serde_json::from_value(e.clone()).unwrap_or_else(|_| ApiError::new("failed", e.to_string()))));
    }
    Ok(Ok(reply.get("ok").cloned().unwrap_or(Value::Null)))
}

/// A call forwarded by another process (see `forward`): run here, never passed on again.
pub fn answer_forwarded(host: &dyn Host, body: &Value) -> Value {
    let _answering = host.activity().map(Activity::begin);
    let run = || -> ApiResult {
        let caller: Caller = serde_json::from_value(body["caller"].clone()).map_err(|e| ApiError::new("bad_request", e.to_string()))?;
        let op = body["op"].as_str().unwrap_or("");
        let o = find(op).ok_or_else(|| ApiError::new("unknown_operation", format!("There is no operation {op}")))?;
        allowed(o, &caller)?;
        if host.library().is_none() {
            // a background process announces itself before its library has finished opening, and holds no other
            if host.me().background {
                return Err(ApiError::new("busy", "The library is still opening; try again shortly"));
            }
            return Err(ApiError::new("library_closed", "That window has no library open any more"));
        }
        (o.run)(&Ctx { host, caller: &caller, entry: None }, body["args"].clone())
    };
    match run() {
        Ok(v) => json!({"ok": v}),
        Err(e) => json!({"error": e}),
    }
}

/// An answer that is a file: HTTP sends its bytes, other callers get its path and type.
pub fn file_reply(path: &Path, mime: &str) -> Value {
    json!({"$file": {"path": path, "type": mime}})
}

pub fn as_file(v: &Value) -> Option<(PathBuf, String)> {
    let f = v.get("$file")?;
    Some((PathBuf::from(f["path"].as_str()?), f["type"].as_str().unwrap_or("application/octet-stream").to_string()))
}

/// `library.list`: every library on the list with the state the API sees it in.
fn library_list(host: &dyn Host, caller: &Caller) -> Value {
    let cfg = host.config();
    let list = known(host, &cfg);
    let open = holders(host, &list);
    let default = default_library(host, &list, &cfg).map(|l| l.path);
    let items: Vec<Value> = list
        .iter()
        .filter(|l| may_reach(caller, l).is_ok())
        .map(|l| {
            let held = open.iter().find(|(o, _)| same_path(&o.path, &l.path));
            let state = if let Some((_, h)) = held {
                if h.background {
                    "background"
                } else {
                    "window"
                }
            } else if instance::is_library(&l.path) {
                "closed"
            } else {
                "offline"
            };
            let mut v = json!({
                "id": l.id, "name": instance::library_name(&l.path), "icon": l.icon, "color": l.color, "state": state,
                "default": default.as_deref().is_some_and(|d| same_path(d, &l.path)),
            });
            if let Some((_, h)) = held.filter(|(_, h)| !h.background && !h.window.is_empty()) {
                v["window"] = json!(h.window);
            }
            if caller.may(Perm::FilesRead) {
                v["path"] = json!(l.path);
            }
            v
        })
        .collect();
    Value::Array(items)
}
