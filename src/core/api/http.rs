//! The API's address on this computer: `POST /api/v1/<namespace>/<method>`, events at `POST /api/v1/events`, the client at `GET /api/v1/curio.js` and `curio.d.ts`, and MCP at `/mcp`, on 127.0.0.1 only.
//! The first Curio process to bind the port answers for every library; `handle` passes each call to the process holding its library, and `Gather` brings every process's events here.

use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::events::{self, Gather, Since, Subscription};
use super::tokens::{self, ApiToken};
use super::{as_file, find, handle, mcp, reference, Activity, ApiError, Caller, CallerKind, Host, Target};

pub const PREFIX: &str = "/api/v1/";
/// The client, from `src/core/api/client/`; it holds no secrets and needs no token.
const CLIENT_JS: &str = include_str!("client/curio.js");
/// Room for a file sent as base64 to `item.add.fromBytes`.
const MAX_BODY: u64 = 64 << 20;
/// Requests handled at once; a client that never finishes its body holds one until it disconnects.
const MAX_REQUESTS: usize = 8;
/// Event streams open at once, beside the requests.
const MAX_STREAMS: usize = 16;
/// How soon a stream ends once this process stops answering on the port, so its clients move to the next one.
const STREAM_CHECK: Duration = Duration::from_millis(500);
/// How often an open event stream checks that its token still stands as it was.
const GRANT_CHECK: Duration = Duration::from_secs(3);

pub struct Server {
    pub port: u16,
    stop: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<()>,
}

impl Server {
    /// Returns once the port is free again.
    pub fn stop(self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.thread.join();
    }
}

/// Binds the port, or fails when another process has it. Port 0 picks a free one.
pub fn start(host: Arc<dyn Host>, port: u16) -> Result<Server, String> {
    start_with(host, port, false)
}

/// The address plugin programs call with their run tokens (see `programs`), on a free port.
pub fn start_programs(host: Arc<dyn Host>) -> Result<Server, String> {
    start_with(host, 0, true)
}

fn start_with(host: Arc<dyn Host>, port: u16, programs: bool) -> Result<Server, String> {
    let server = tiny_http::Server::http(("127.0.0.1", port)).map_err(|e| format!("cannot listen on 127.0.0.1:{port}: {e}"))?;
    let port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(port);
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let busy = Arc::new(AtomicUsize::new(0));
    let streams = Arc::new(AtomicUsize::new(0));
    let gather = Gather::new(host.clone(), stop.clone());
    let pairing = Arc::new(super::pair::Gate::default());
    if !programs {
        super::upload::sweep(&super::upload::spool(&host.data_dir()), false);
    }
    let thread = std::thread::Builder::new()
        .name(if programs { "api-programs" } else { "api" }.into())
        .spawn(move || {
            while !stop2.load(Ordering::Relaxed) {
                let Ok(Some(req)) = server.recv_timeout(Duration::from_millis(500)) else { continue };
                if busy.fetch_add(1, Ordering::AcqRel) >= MAX_REQUESTS {
                    busy.fetch_sub(1, Ordering::AcqRel);
                    let _ = req.respond(tiny_http::Response::from_string("Too many requests at once").with_status_code(503));
                    continue;
                }
                let host = host.clone();
                let (busy2, streams, gather, pairing) = (busy.clone(), streams.clone(), gather.clone(), pairing.clone());
                let spawned = std::thread::Builder::new().name("api-request".into()).spawn(move || {
                    let _held = Count(&busy2, -1);
                    serve(host.as_ref(), req, port, &Streams { busy: &busy2, open: &streams, gather: &gather, pairing: &pairing }, programs);
                });
                if spawned.is_err() {
                    busy.fetch_sub(1, Ordering::AcqRel);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(Server { port, stop, thread })
}

/// Gives back a count when dropped, so a call that panics doesn't keep its place: `-1` takes one off, `1` puts one back.
struct Count<'a>(&'a AtomicUsize, i8);

impl Drop for Count<'_> {
    fn drop(&mut self) {
        if self.1 < 0 {
            self.0.fetch_sub(1, Ordering::AcqRel);
        } else {
            self.0.fetch_add(1, Ordering::AcqRel);
        }
    }
}

/// DNS rebinding shows up as a foreign Host.
fn host_ok(host: Option<&str>, port: u16) -> bool {
    host.is_some_and(|h| [format!("127.0.0.1:{port}"), format!("localhost:{port}"), format!("[::1]:{port}")].iter().any(|x| x.eq_ignore_ascii_case(h)))
}

/// A plain-text POST is what a page can send without asking the browser first.
fn is_json(content_type: Option<&str>) -> bool {
    content_type.is_some_and(|c| c.split(';').next().unwrap_or("").trim().eq_ignore_ascii_case("application/json"))
}

fn header(name: &str, value: &str) -> Option<tiny_http::Header> {
    tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()).ok()
}

fn send(req: tiny_http::Request, status: u16, body: &Value, cors: Option<&str>) {
    let mut r = tiny_http::Response::from_string(body.to_string()).with_status_code(status);
    r.add_header(header("Content-Type", "application/json").unwrap());
    if let Some(o) = cors {
        r.add_header(header("Access-Control-Allow-Origin", o).unwrap_or_else(|| header("Access-Control-Allow-Origin", "null").unwrap()));
        r.add_header(header("Vary", "Origin").unwrap());
    }
    let _ = req.respond(r);
}

fn fail(req: tiny_http::Request, e: ApiError, cors: Option<&str>) {
    send(req, e.status(), &e.json(), cors)
}

/// The body, refused when it is too large.
fn body_of(req: &mut tiny_http::Request) -> Result<String, ApiError> {
    let too_large = || ApiError::new("bad_request", "The request is too large");
    if req.body_length().is_some_and(|n| n as u64 > MAX_BODY) {
        return Err(too_large());
    }
    let mut body = String::new();
    let n = std::io::Read::read_to_string(&mut std::io::Read::take(req.as_reader(), MAX_BODY + 1), &mut body).map_err(|e| ApiError::new("bad_request", e.to_string()))?;
    if n as u64 > MAX_BODY {
        return Err(too_large());
    }
    Ok(body)
}

fn token_of<'a>(list: &'a [ApiToken], auth: Option<&str>) -> Result<&'a ApiToken, ApiError> {
    let secret = tokens::bearer(auth).ok_or_else(|| ApiError::new("unauthorized", "Send a token from Settings › API as `Authorization: Bearer <token>`"))?;
    tokens::find(list, secret).ok_or_else(|| ApiError::new("unauthorized", "That token isn't known; it may have been deleted"))
}

/// What an event stream needs beside the request.
struct Streams<'a> {
    busy: &'a AtomicUsize,
    open: &'a AtomicUsize,
    gather: &'a Arc<Gather>,
    pairing: &'a super::pair::Gate,
}

/// Whether a Curio answers the API's ping on this port, quickly: a browser's background start ends when one does.
pub fn answers_ping(port: u16) -> bool {
    use std::io::{Read, Write};
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut s) = std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(300)) else { return false };
    let _ = s.set_read_timeout(Some(std::time::Duration::from_millis(700)));
    let _ = s.set_write_timeout(Some(std::time::Duration::from_millis(300)));
    if s.write_all(format!("GET {}ping HTTP/1.0\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n", PREFIX).as_bytes()).is_err() {
        return false;
    }
    let mut buf = Vec::new();
    let _ = s.take(8192).read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf);
    let body = text.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");
    serde_json::from_str::<serde_json::Value>(body).is_ok_and(|v| v["name"] == "Curio")
}

fn serve(host: &dyn Host, mut req: tiny_http::Request, port: u16, streams: &Streams, programs: bool) {
    let url = req.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((&url, ""));
    let get = |name: &'static str| req.headers().iter().find(|h| h.field.equiv(name)).map(|h| h.value.as_str().to_string());
    let (origin, auth, ctype, last_id) = (get("Origin"), get("Authorization"), get("Content-Type"), get("Last-Event-ID"));
    if !host_ok(get("Host").as_deref(), port) {
        return send(req, 403, &ApiError::new("not_allowed", "Use 127.0.0.1 or localhost").json(), None);
    }
    if req.method() == &tiny_http::Method::Options {
        let mut r = tiny_http::Response::empty(204);
        if let Some(o) = &origin {
            // Chrome asks before a public page reaches this computer (Private Network Access)
            for h in [("Access-Control-Allow-Origin", o.as_str()), ("Access-Control-Allow-Methods", "GET, POST"), ("Access-Control-Allow-Headers", "Authorization, Content-Type, Last-Event-ID"), ("Access-Control-Allow-Private-Network", "true"), ("Access-Control-Max-Age", "600"), ("Vary", "Origin")] {
                r.add_header(header(h.0, h.1).unwrap());
            }
        }
        let _ = req.respond(r);
        return;
    }
    let is_mcp = path == "/mcp" && !programs;
    if !is_mcp && !path.starts_with(PREFIX) {
        return fail(req, ApiError::new("not_found", "Curio's API is at /api/v1/<namespace>/<method>, MCP at /mcp"), None);
    }
    // the one call without a token: a person allows it in a window
    if !programs && path == format!("{PREFIX}pair") && req.method() == &tiny_http::Method::Post {
        let body = match body_of(&mut req) {
            Ok(b) => b,
            Err(e) => return fail(req, e, origin.as_deref()),
        };
        return match super::pair::request(host, streams.pairing, origin.as_deref(), &body) {
            Ok((v, id)) => {
                let mut r = tiny_http::Response::from_string(v.to_string());
                r.add_header(header("Content-Type", "application/json").unwrap());
                if let Some(o) = &origin {
                    r.add_header(header("Access-Control-Allow-Origin", o).unwrap_or_else(|| header("Access-Control-Allow-Origin", "null").unwrap()));
                }
                if req.respond(r).is_err() {
                    host.drop_token(&id);
                }
            }
            Err(e) => fail(req, e, origin.as_deref()),
        };
    }
    if req.method() == &tiny_http::Method::Get {
        let file = match &path[PREFIX.len().min(path.len())..] {
            "curio.js" => Some((CLIENT_JS.to_string(), "text/javascript; charset=utf-8")),
            "curio.d.ts" => Some((reference::typescript(), "text/plain; charset=utf-8")),
            // tells nothing a port scan wouldn't; a browser's background start ends when it answers
            "ping" => Some((serde_json::json!({ "name": "Curio", "apiVersion": 1 }).to_string(), "application/json")),
            _ => None,
        };
        if let Some((text, mime)) = file {
            let mut r = tiny_http::Response::from_string(text);
            r.add_header(header("Content-Type", mime).unwrap());
            r.add_header(header("Access-Control-Allow-Origin", "*").unwrap());
            let _ = req.respond(r);
            return;
        }
    }
    // errors are readable by the page that asked; a token is still needed for anything else
    let cors = origin.as_deref();
    if req.method() != &tiny_http::Method::Post {
        return fail(req, ApiError::new("bad_request", "Send a POST with a JSON body"), cors);
    }
    if programs {
        // programs send no Origin; a page that guessed the address still lacks a run's token
        if origin.is_some() {
            return fail(req, ApiError::new("not_allowed", "This address is for plugin programs"), None);
        }
        let caller = match tokens::bearer(auth.as_deref()).and_then(super::programs::plugin_of) {
            Some(id) => match super::programs::caller_for(host, &id) {
                Some(c) => c,
                None => return fail(req, ApiError::new("not_allowed", format!("The plugin {id} is off")), None),
            },
            None => return fail(req, ApiError::new("unauthorized", "Use the CURIO_API_TOKEN this program was started with; it ends with the program's run"), None),
        };
        return answer_call(host, req, path, query, ctype.as_deref(), &caller, None);
    }
    let cfg = host.config();
    if is_mcp && !cfg.mcp_enabled {
        return fail(req, ApiError::new("not_found", "MCP is turned off in Settings › MCP"), cors);
    }
    let token = match token_of(&cfg.tokens, auth.as_deref()) {
        Ok(t) => t,
        Err(e) => return fail(req, e, cors),
    };
    if !tokens::origin_allowed(token, origin.as_deref()) {
        return fail(req, ApiError::new("not_allowed", format!("{} may not be used from {}", token.name, origin.as_deref().unwrap_or_default())), cors);
    }
    if !is_mcp && path[PREFIX.len()..].trim_matches('/') == "upload" {
        if !token.has_perm(super::Perm::FilesWrite) {
            return fail(req, ApiError::new("not_allowed", format!("{} may not add files", token.name)), cors);
        }
        let name = url::form_urlencoded::parse(query.as_bytes()).find(|(k, _)| k == "name").map(|(_, v)| v.to_string()).unwrap_or_default();
        let length = req.body_length().map(|n| n as u64);
        let dir = super::upload::spool(&host.data_dir());
        return match super::upload::receive(&dir, req.as_reader(), length, &token.id, &name, super::upload::MAX_UPLOAD) {
            Ok(v) => send(req, 200, &v, cors),
            Err(e) => fail(req, e, cors),
        };
    }
    if !is_mcp && path[PREFIX.len()..].trim_matches('/') != "events" {
        return answer_call(host, req, path, query, ctype.as_deref(), &Caller::from_token(token, CallerKind::Web), cors);
    }
    let body = match body_of(&mut req) {
        Ok(b) => b,
        Err(e) => return fail(req, e, cors),
    };
    if !body.trim().is_empty() && !is_json(ctype.as_deref()) {
        return fail(req, ApiError::new("bad_request", "Send the body as application/json"), cors);
    }
    if is_mcp {
        let caller = Caller::from_token(token, CallerKind::Mcp);
        return match mcp::answer_body(host, &caller, &body) {
            Some(v) => send(req, 200, &v, cors),
            None => {
                let _ = req.respond(tiny_http::Response::empty(202));
            }
        };
    }
    let args: Value = if body.trim().is_empty() { json!({}) } else {
        match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => return fail(req, ApiError::new("bad_request", format!("The body isn't JSON: {e}")), cors),
        }
    };
    let caller = Caller::from_token(token, CallerKind::Web);
    let filter: events::Filter = match serde_json::from_value(args) {
        Ok(f) => f,
        Err(e) => return fail(req, ApiError::new("bad_request", e.to_string()), cors),
    };
    let sub = match events::subscription(host, &caller, &filter) {
        Ok(s) => s,
        Err(e) => return fail(req, e, cors),
    };
    if streams.open.fetch_add(1, Ordering::AcqRel) >= MAX_STREAMS {
        streams.open.fetch_sub(1, Ordering::AcqRel);
        return fail(req, ApiError::new("busy", "Too many event streams are open"), cors);
    }
    let _open = Count(streams.open, -1);
    // a token deleted or changed in Settings, in this process or another, ends its streams; the client reconnects under what it has now
    let granted = token.clone();
    let still = || token_of(&host.config().tokens, auth.as_deref()).is_ok_and(|t| *t == granted);
    // a stream doesn't keep a request from being answered
    streams.busy.fetch_sub(1, Ordering::AcqRel);
    let _back = Count(streams.busy, 1);
    stream(host, req, &sub, &Since::after(last_id.as_deref().or(filter.after.as_deref())), streams.gather, cors, &still);
}

/// One call: the body as its parameters, the path as its name, the answer as JSON or a file.
fn answer_call(host: &dyn Host, mut req: tiny_http::Request, path: &str, query: &str, ctype: Option<&str>, caller: &Caller, cors: Option<&str>) {
    let body = match body_of(&mut req) {
        Ok(b) => b,
        Err(e) => return fail(req, e, cors),
    };
    if !body.trim().is_empty() && !is_json(ctype) {
        return fail(req, ApiError::new("bad_request", "Send the body as application/json"), cors);
    }
    let mut args: Value = if body.trim().is_empty() { json!({}) } else {
        match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => return fail(req, ApiError::new("bad_request", format!("The body isn't JSON: {e}")), cors),
        }
    };
    let Some(obj) = args.as_object_mut() else { return fail(req, ApiError::new("bad_request", "The body is a JSON object of parameters"), cors) };
    let op = path[PREFIX.len()..].trim_matches('/').replace('/', ".");
    // a client's default library, left out where it can't apply
    if let Some((_, lib)) = url::form_urlencoded::parse(query.as_bytes()).find(|(k, _)| k == "library") {
        if find(&op).is_some_and(|o| o.target != Target::App) {
            obj.entry("library").or_insert_with(|| json!(lib));
        }
    }
    match handle(host, caller, &op, args) {
        Ok(v) if find(&op).is_some_and(|o| o.bytes) => match as_file(&v) {
            Some((file, mime)) => send_file(req, &file, &mime, cors),
            None => send(req, 200, &v, cors),
        },
        Ok(v) => send(req, 200, &v, cors),
        Err(e) => fail(req, e, cors),
    }
}

/// Server-sent events until the client goes or this process stops answering on the port. A subscriber keeps a background process from ending.
fn stream(host: &dyn Host, req: tiny_http::Request, sub: &Subscription, since: &Since, gather: &Arc<Gather>, cors: Option<&str>, still: &dyn Fn() -> bool) {
    let floor = since.floor();
    let last = match since {
        Since::After(id) => Some(id.as_str()),
        _ => None,
    };
    let rx = gather.subscribe(since);
    let _answering = host.activity().map(Activity::begin);
    // a keep-alive connection outlives the stream, so the stream's end is its last chunk, which tells the client to reconnect
    let chunked = *req.http_version() >= (1, 1);
    let mut head = String::from("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\n");
    head += if chunked { "Transfer-Encoding: chunked\r\n" } else { "Connection: close\r\n" };
    if let Some(o) = cors {
        head += &format!("Access-Control-Allow-Origin: {o}\r\nVary: Origin\r\n");
    }
    head += "\r\n";
    let mut out = req.into_writer();
    if out.write_all(head.as_bytes()).is_err() {
        return;
    }
    let mut send = |text: &str| {
        if chunked {
            write!(out, "{:x}\r\n{text}\r\n", text.len())?;
        } else {
            out.write_all(text.as_bytes())?;
        }
        out.flush()
    };
    if send(": connected\nretry: 3000\n\n").is_err() {
        return;
    }
    let mut quiet = Instant::now();
    let mut checked = Instant::now();
    while !gather.stopped() {
        if checked.elapsed() >= GRANT_CHECK {
            checked = Instant::now();
            if !still() {
                break;
            }
        }
        let text = match rx.recv_timeout(STREAM_CHECK) {
            Ok(e) if e.at < floor || last == Some(e.id.as_str()) => continue,
            Ok(e) => match sub.deliver(host, &e) {
                Some(v) => format!("id: {}\nevent: {}\ndata: {v}\n\n", e.id, e.name),
                None => continue,
            },
            Err(crossbeam_channel::RecvTimeoutError::Timeout) if quiet.elapsed() >= events::HEARTBEAT => ": ping\n\n".to_string(),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
            // fell too far behind: the client reconnects with its last id
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        };
        quiet = Instant::now();
        if send(&text).is_err() {
            return;
        }
    }
    if chunked {
        let _ = out.write_all(b"0\r\n\r\n").and_then(|_| out.flush());
    }
}

fn send_file(req: tiny_http::Request, file: &std::path::Path, mime: &str, cors: Option<&str>) {
    let f = match std::fs::File::open(file) {
        Ok(f) => f,
        Err(e) => return fail(req, ApiError::new("not_found", format!("The file can't be read: {e}")), cors),
    };
    let mut r = tiny_http::Response::from_file(f);
    r.add_header(header("Content-Type", mime).unwrap_or_else(|| header("Content-Type", "application/octet-stream").unwrap()));
    if let Some(o) = cors {
        r.add_header(header("Access-Control-Allow-Origin", o).unwrap_or_else(|| header("Access-Control-Allow-Origin", "null").unwrap()));
        r.add_header(header("Vary", "Origin").unwrap());
    }
    let _ = req.respond(r);
}
