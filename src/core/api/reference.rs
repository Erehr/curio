//! The operation reference made from the registry: `curio.d.ts` for editors (served at `/api/v1/curio.d.ts`, kept in `src/core/api/client/`).
//! `curio-dev reference` (Curio's development tools) writes it from here, and `curio-dev reference --check` says whether it matches.


use serde_json::Value;

use super::events::EVENTS;
use super::{registry, CallerKind, Op, Target};

/// The types answers and events use, by name.
pub const TYPES: &str = r#"/** The call was done; nothing more to tell. */
type Done = { success: true };

/** An entry of this computer's library list. */
interface Library {
  /** Stable across renames and moves; null until the library was first open in Curio 0.7 or later. */
  id: string | null;
  name: string;
  icon: string | null;
  color: string | null;
  /** window: a window shows it; background: a process without a window holds it; closed: a call opens it in the background; offline: its folder can't be reached. */
  state: 'window' | 'background' | 'closed' | 'offline';
  /** Where calls that name no library go. */
  default: boolean;
  /** The id of the window showing it, for window operations. */
  window?: string;
  /** With files.read. */
  path?: string;
}

/** A window's theme. `tokens` maps each CSS variable's name, without the leading dashes, to its value: colours such as `bg`, `panel`, `surface`, `text` and `accent`, the scales `r` (corners), `round` (the corners of what is otherwise fully round), `g` (spacing) and `row`, and the fonts `ui-font` and `mono-font`. */
interface Theme {
  name: string;
  dark: boolean;
  tokens: Record<string, string>;
}

interface LibraryInfo {
  id: string | null;
  name: string | null;
  schema: number;
  floor: number;
  counts: { all: number; favorites: number; uncategorized: number; untagged: number; trash: number; missing: number; recent: number; tags: number; size: number };
  path?: string;
}

type Kind = 'image' | 'video' | 'audio' | 'document' | 'code' | 'link' | 'archive' | 'font' | 'model' | 'other';

interface ItemMember {
  id: string;
  role: string | null;
  name: string;
  ext: string;
  size: number;
  status: 'Available' | 'Storage offline' | 'Missing';
}

interface Item {
  /** A UUID, stable across moves, renames and relinks. */
  id: string;
  name: string;
  fileName: string;
  ext: string;
  kind: Kind;
  size: number;
  width: number | null;
  height: number | null;
  /** Seconds. */
  duration: number | null;
  /** 0 to 5. */
  rating: number;
  favorite: boolean;
  explicit: boolean;
  annotation: string;
  /** Where it came from; for a link, the link. */
  url: string;
  tags: string[];
  folders: { id: number; name: string }[];
  status: 'Available' | 'Storage offline' | 'Missing';
  /** Milliseconds since 1970. */
  imported: number;
  modified: number | null;
  /** Number of grouped member files. */
  parts: number;
  /** Parent item UUID when this asset is a grouped member. */
  partOf?: string;
  partRole?: string | null;
  /** With files.read. */
  path?: string;
}

interface ItemWithFields extends Omit<Item, 'parts'> {
  /** Custom field values by key. */
  fields: Record<string, unknown>;
  /** 0 when ungrouped, or member summaries when grouped. */
  parts: number | ItemMember[];
}

interface Page<T> {
  total: number;
  offset: number;
  items: T[];
}

/** New items, or `queued` when adding them took longer than the call waits. */
type Added = { success: true; ids: string[]; errors: string[]; handed?: { url: string; plugin: string; handler: string }[] } | { success: true; queued: true };

interface Folder {
  id: number;
  parentId: number | null;
  name: string;
  color: string | null;
  sortOrder: number;
  /** Items in it, not counting subfolders. */
  count: number;
  icon: string | null;
  /** Tags its new items get. */
  autoTags: string[];
}

interface SmartFolder {
  id: number;
  name: string;
  query: string;
  sort: string | null;
  icon: string | null;
  groupBy: string | null;
  parentId: number | null;
  sortOrder: number;
  count: number | null;
}

interface Tag {
  id: number;
  name: string;
  /** The group that gives it its colour. */
  groupId: number | null;
  color: string | null;
  count: number;
  starred: boolean;
  groupIds: number[];
}

interface TagGroup {
  id: number;
  name: string;
  color: string | null;
  icon: string | null;
  description: string | null;
}

interface Field {
  id: number;
  key: string;
  label: string;
  type: 'text' | 'integer' | 'float' | 'boolean' | 'date' | 'date_time' | 'single_select' | 'multi_select' | 'json' | 'url' | 'file_ref';
  description: string;
  defaultValue: string | null;
  options: string[];
  validation: { min?: number; max?: number; maxLength?: number; pattern?: string; required: boolean };
  format: string;
  widget: string;
  /** user, or the id of the plugin that defined it. */
  owner: string;
  visible: boolean;
  sortOrder: number;
  ownFilter: boolean;
  filterKind: string;
  /** Curio keeps what readers find for it, and it takes part in filters, sorts, columns and search. Always true for a field no reader fills. */
  indexed: boolean;
  /** The key of the field this one only makes sense with; switching one switches both. */
  pair: string | null;
  /** Number fields of one owner with the same name here are one filter of that name, with a range for each. */
  filterGroup: string | null;
  /** The readers that fill it from files; empty for a field filled by hand. */
  readers: string[];
  /** Switched on without reading its files yet. */
  needsScan: boolean;
}

interface FieldCoverage {
  key: string;
  /** Items with a value of it. */
  withValue: number;
  /** Files its readers that are on read. */
  applies: number;
  /** Files waiting to be read: new or changed, or queued for its readers. */
  waiting: number;
  /** Files one of its readers failed on. */
  failed: number;
}

interface FieldFacets {
  /** Each value with how many items have it. */
  values: [string | number, number][];
  set: number;
  total: number;
}

interface Facets {
  rating: Record<string, number>;
  shape: { landscape: number; portrait: number; square: number; wide: number; tall: number; '4:3': number; '16:9': number };
  kinds: Record<string, number>;
  exts: Record<string, number>;
  animatedExts: Record<string, number>;
  animated: number;
  total: number;
}

interface Location {
  id: number;
  name: string;
  online: boolean;
  watch: boolean;
  access: 'read_only' | 'full' | 'organize';
  count: number;
  lastScan: number | null;
  icon: string | null;
  color: string | null;
  path?: string;
}

/** What a window shows. */
interface View {
  show: 'all' | 'favorites' | 'uncategorized' | 'untagged' | 'recent' | 'random' | 'trash' | 'missing' | 'folder' | 'smartFolder' | 'tag' | 'location' | 'locationFolder' | 'folders' | 'locations';
  /** The folder's, smart folder's, tag's or location's number; the numbers for several. */
  id: number | number[] | null;
  /** The folder inside a location. */
  path: string | null;
  name: string;
  search: string;
  /** The search and filters as one query. */
  query: string;
  count: number;
  /** tags: the tag manager; `tab:<plugin>/<tab>`: a Tab plugin's page. */
  view: 'grid' | 'tags' | `tab:${string}`;
  /** What font samples say, empty for each font's own line: view.setSample. */
  sample: { text: string };
}

interface Operation {
  name: string;
  description: string;
  permission: string;
  kind: 'read' | 'write' | 'window';
  /** JSON Schema. */
  params: object;
}

/** Long work a plugin does, shown in Curio's status area. */
interface Job {
  id: string;
  /** The plugin it belongs to. */
  plugin: string | null;
  title: string;
  done: number;
  /** 0 when not known. */
  total: number;
  text: string;
  state: 'running' | 'done' | 'failed' | 'cancelled';
  /** What job.finish gave; for process.run `{ code, stdout, stderr, folder }`. */
  result: unknown;
  error: string | null;
  /** Milliseconds since 1970. */
  started: number;
  ended: number | null;
}

/** A program process.run started: at once, or with `wait` when it ended. */
interface ProcessRun {
  job: string;
  /** The run's own folder, where the program starts. */
  folder: string;
  state: 'running' | 'done' | 'failed' | 'cancelled';
  /** With `wait`, once it ended: its exit code, the last megabyte it printed on each pipe, and what went wrong. */
  code?: number | null;
  stdout?: string;
  stderr?: string;
  error?: string | null;
}

/** A program Curio finds for plugins. */
interface Tool {
  name: string;
  title: string;
  about: string;
  found: boolean;
  version: string | null;
  /** Where to get it. */
  homepage: string;
  /** Its place was chosen in Settings. */
  chosen: boolean;
  /** For callers that may run programs. */
  path?: string | null;
}"#;

/// The calls a token or a plugin's page can make; the window's own calls aren't part of the client.
fn public() -> impl Iterator<Item = &'static Op> {
    registry().iter().filter(|o| o.callers.contains(&CallerKind::Web) || o.callers.contains(&CallerKind::Plugin))
}

fn union(parts: Vec<String>) -> String {
    let mut out: Vec<String> = Vec::new();
    for p in parts {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out.join(" | ")
}

/// A JSON schema as a TypeScript type.
fn ts(s: &Value) -> String {
    if let Some(list) = s.get("enum").and_then(Value::as_array) {
        return union(list.iter().map(Value::to_string).collect());
    }
    for k in ["anyOf", "oneOf"] {
        if let Some(list) = s.get(k).and_then(Value::as_array) {
            return union(list.iter().map(ts).collect());
        }
    }
    match s.get("type") {
        Some(Value::Array(types)) => union(types.iter().filter_map(Value::as_str).map(|t| prim(t, s)).collect()),
        Some(Value::String(t)) => prim(t, s),
        _ => "unknown".into(),
    }
}

fn prim(t: &str, s: &Value) -> String {
    match t {
        "string" => "string".into(),
        "integer" | "number" => "number".into(),
        "boolean" => "boolean".into(),
        "null" => "null".into(),
        "array" => {
            let item = s.get("items").map(ts).unwrap_or_else(|| "unknown".into());
            if item.contains(' ') {
                format!("({item})[]")
            } else {
                format!("{item}[]")
            }
        }
        "object" => object(s),
        _ => "unknown".into(),
    }
}

fn object(s: &Value) -> String {
    let props = props(s);
    if props.is_empty() {
        return match s.get("additionalProperties") {
            Some(v @ Value::Object(_)) => format!("Record<string, {}>", ts(v)),
            _ => "Record<string, unknown>".into(),
        };
    }
    let fields: Vec<String> = props.iter().map(|p| format!("{}{}: {}", p.name, if p.required { "" } else { "?" }, p.ty)).collect();
    format!("{{ {} }}", fields.join("; "))
}

struct Prop {
    name: String,
    required: bool,
    ty: String,
    doc: String,
}

fn props(s: &Value) -> Vec<Prop> {
    let required: Vec<&str> = s.get("required").and_then(Value::as_array).map(|r| r.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    let mut list: Vec<Prop> = s
        .get("properties")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .map(|(k, v)| Prop { name: k.clone(), required: required.contains(&k.as_str()), ty: ts(v), doc: v.get("description").and_then(Value::as_str).unwrap_or_default().replace('\n', " ") })
                .collect()
        })
        .unwrap_or_default();
    // where the call goes comes after what it does
    list.sort_by_key(|p| matches!(p.name.as_str(), "library" | "window"));
    list
}

/// `item.add.fromUrls` → `ItemAddFromUrlsParams`
fn params_name(op: &str) -> String {
    let mut out = String::new();
    for part in op.split('.') {
        let mut c = part.chars();
        if let Some(f) = c.next() {
            out.extend(f.to_uppercase());
            out.push_str(c.as_str());
        }
    }
    out + "Params"
}

fn needs(o: &Op) -> String {
    let mut s = format!("Needs `{}`.", o.perm.name());
    if !o.callers.contains(&CallerKind::Web) {
        return s + if o.pages_only { " Plugin pages only." } else { " Plugins only: their pages and programs." };
    }
    match o.target {
        Target::App => {}
        Target::Library => s += " Takes `library`.",
        Target::Entry => s += " Names the library with `library`.",
        Target::Window => s += " Takes `window` or `library`.",
    }
    s
}

/// The methods and namespaces under one name, `item` holding `query` and `add`, in the registry's order.
#[derive(Default)]
struct Node {
    ops: Vec<&'static Op>,
    subs: Vec<(String, Node)>,
}

impl Node {
    fn sub(&mut self, name: &str) -> &mut Node {
        let at = match self.subs.iter().position(|(n, _)| n == name) {
            Some(i) => i,
            None => {
                self.subs.push((name.to_string(), Node::default()));
                self.subs.len() - 1
            }
        };
        &mut self.subs[at].1
    }
}

fn tree() -> Node {
    let mut root = Node::default();
    for o in public() {
        let parts: Vec<&str> = o.name.split('.').collect();
        let mut n = &mut root;
        for p in &parts[..parts.len() - 1] {
            n = n.sub(p);
        }
        n.ops.push(o);
    }
    root
}

fn api_members(n: &Node, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    for o in &n.ops {
        let method = o.name.rsplit('.').next().unwrap_or(o.name);
        let p = o.public_params();
        let args = if props(&p).is_empty() {
            String::new()
        } else if props(&p).iter().any(|x| x.required) {
            format!("params: {}", params_name(o.name))
        } else {
            format!("params?: {}", params_name(o.name))
        };
        out.push_str(&format!("{pad}/** {}. {} */\n{pad}{method}({args}): Promise<{}>;\n", o.doc, needs(o).replace('`', ""), o.returns));
    }
    for (name, sub) in &n.subs {
        out.push_str(&format!("{pad}{name}: {{\n"));
        api_members(sub, depth + 1, out);
        out.push_str(&format!("{pad}}};\n"));
    }
}

fn indent(text: &str, by: &str) -> String {
    text.lines().map(|l| if l.is_empty() { String::new() } else { format!("{by}{l}") }).collect::<Vec<_>>().join("\n")
}

/// `curio.d.ts`: the client's methods with their parameters and answers.
pub fn typescript() -> String {
    let mut out = String::from(CLIENT_HEAD);
    out.push_str(&indent(TYPES, "  "));
    out.push_str("\n\n  interface EventMap {\n");
    for (name, perm, doc, data) in EVENTS {
        out.push_str(&format!("    /** {doc} Needs {}. */\n    '{name}': {data};\n", perm.name()));
    }
    out.push_str("  }\n");
    for o in public() {
        let p = o.public_params();
        let list = props(&p);
        if list.is_empty() {
            continue;
        }
        out.push_str(&format!("\n  interface {} {{\n", params_name(o.name)));
        for x in list {
            if !x.doc.is_empty() {
                out.push_str(&format!("    /** {} */\n", x.doc));
            }
            out.push_str(&format!("    {}{}: {};\n", x.name, if x.required { "" } else { "?" }, x.ty));
        }
        out.push_str("  }\n");
    }
    out.push_str("\n  /** Every operation, by namespace. */\n  interface Api {\n");
    api_members(&tree(), 2, &mut out);
    out.push_str("  }\n}\n\n");
    out.push_str(CLIENT_TAIL);
    out
}

const CLIENT_HEAD: &str = r#"// Curio's web API for TypeScript and editors: the client in curio.js with every operation, its parameters and its answer.
// Generated from Curio's operation registry (src/core/api/reference.rs); served at http://127.0.0.1:41733/api/v1/curio.d.ts. Don't edit it by hand.
// In a script: /// <reference path="curio.d.ts" />

declare namespace Curio {
  interface Options {
    /** A token from Settings › API. */
    token: string;
    /** 41733 unless changed in Settings › API. */
    port?: number;
    host?: string;
    /** The library calls go to when they name none: an id from library.list, a folder or a name. */
    library?: string;
    /** fetch, or gm for GM_xmlhttpRequest in a userscript; found by itself when left out. Events use GM_xmlhttpRequest only in Tampermonkey, or with gm. */
    transport?: 'fetch' | 'gm';
  }

  interface CurioError extends Error {
    /** bad_request, unauthorized, not_allowed, not_found, unknown_operation, unknown_library, library_closed, library_offline, no_window, busy, failed, or network when Curio couldn't be reached. */
    code: string;
    /** The HTTP status; 0 when Curio couldn't be reached. */
    status: number;
  }

  interface EventFilter {
    /** Names, or a namespace as items.*; every event the token may receive when left out. */
    events?: (keyof EventMap | `${string}.*`)[];
    /** One library's events; the library list's still come. */
    library?: string;
    /** These items' events; needs library. */
    ids?: string[];
  }

  interface Event<N extends keyof EventMap = keyof EventMap> {
    id: string;
    name: N;
    /** The library's id; null for the library list. */
    library: string | null;
    /** Milliseconds since 1970. */
    at: number;
    data: EventMap[N];
  }

  interface Subscription {
    /** Ends the stream; it isn't opened again. */
    close(): void;
  }

  interface Client extends Api {
    /** Any operation by its name, for names this file doesn't know yet. */
    call(op: string, params?: Record<string, unknown>): Promise<any>;
    /** Follows events; the stream reconnects by itself, resuming after the last event, until closed. */
    subscribe(filter: EventFilter, onEvent: (e: Event) => void, onState?: (state: 'open' | 'retrying' | 'closed', error?: CurioError) => void): Subscription;
    on<N extends keyof EventMap>(name: N, onEvent: (e: Event<N>) => void, filter?: Omit<EventFilter, 'events'>): Subscription;
    /** The same client with calls going to another library when they name none. */
    forLibrary(library: string): Client;
  }

"#;

const CLIENT_TAIL: &str = r#"declare const Curio: {
  connect(options: Curio.Options): Curio.Client;
  Error: { new (code: string, message: string, status?: number): Curio.CurioError };
};
"#;
