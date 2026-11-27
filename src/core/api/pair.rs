//! Setup by approval: a browser extension asks for a token with no token, and a person allows it in a Curio window, comparing a code the extension shows.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::{known, tokens, ApiError, ApiResult, Host, Perm};

/// What a paired extension may do: read, describe, add and organize, never delete files or run anything.
pub const PERMS: [Perm; 5] = [Perm::LibraryRead, Perm::MetadataRead, Perm::MetadataWrite, Perm::Organize, Perm::FilesWrite];
/// What `files.write` allows that a paired extension still may not: files deleted or changed in place. Files outside the library need `files.outside`, which it never gets.
pub const REFUSED: [&str; 6] = ["item.add.beside", "item.purge", "item.forgetFile", "item.replaceFile", "item.rotateFile", "item.moveToLibrary"];

const REFUSALS: u32 = 3;
const CLOSED_FOR: Duration = Duration::from_secs(10 * 60);

/// One server's door: one question at a time, and closed for a while after refusals in a row.
#[derive(Default)]
pub struct Gate {
    asking: Mutex<()>,
    state: Mutex<(u32, Option<Instant>)>,
}

/// An extension's own origin, never a page's: `chrome-extension://` with an id of 32 letters a to p, or Firefox's `moz-extension://` with a UUID.
pub fn extension_origin(origin: &str) -> bool {
    if let Some(id) = origin.strip_prefix("chrome-extension://") {
        return id.len() == 32 && id.bytes().all(|b| (b'a'..=b'p').contains(&b));
    }
    if let Some(id) = origin.strip_prefix("moz-extension://") {
        let parts: Vec<&str> = id.split('-').collect();
        return parts.iter().map(|p| p.len()).eq([8, 4, 4, 4, 12]) && parts.iter().all(|p| p.bytes().all(|b| b.is_ascii_hexdigit()));
    }
    false
}

/// The name shown and given to the token: one line, no control characters, short.
fn clean_name(name: &str) -> String {
    let n: String = name.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
    let n: String = n.chars().take(60).collect();
    if n.is_empty() { "Browser extension".into() } else { n }
}

/// Asks a window to allow the extension; answers the new token's secret, once, and the token's id to take it back if the answer can't be sent.
pub fn request(host: &dyn Host, gate: &Gate, origin: Option<&str>, body: &str) -> Result<(Value, String), ApiError> {
    let origin = origin.filter(|o| extension_origin(o)).ok_or_else(|| ApiError::new("not_allowed", "Only a browser extension asks to be paired"))?;
    let v: Value = serde_json::from_str(body).map_err(|_| ApiError::new("bad_request", "Send { \"name\", \"code\" } as JSON"))?;
    let code = v["code"].as_str().unwrap_or("");
    if !(4..=8).contains(&code.len()) || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ApiError::new("bad_request", "`code` is 4 to 8 digits the extension shows"));
    }
    let name = clean_name(v["name"].as_str().unwrap_or(""));
    {
        let mut st = gate.state.lock().unwrap_or_else(|e| e.into_inner());
        if st.1.is_some_and(|t| t > Instant::now()) {
            return Err(ApiError::new("not_allowed", "Curio was refused three times in a row; try again in ten minutes"));
        }
        if st.1.is_some() {
            *st = (0, None);
        }
    }
    let Ok(_one) = gate.asking.try_lock() else { return Err(ApiError::new("busy", "Curio is already asking about another extension")) };
    let cfg = host.config();
    let list: Vec<(String, String)> = known(host, &cfg).into_iter().filter_map(|l| l.id.clone().map(|id| (id, crate::core::library::instance::library_name(&l.path)))).collect();
    let args = json!({
        "name": name, "origin": origin, "code": code,
        "perms": PERMS.iter().map(|p| p.name()).collect::<Vec<_>>(),
        "libraries": list.iter().map(|(id, n)| json!({"id": id, "name": n})).collect::<Vec<_>>(),
    });
    let answer = match host.window("pair.ask", args.clone(), true) {
        Some(a) => a,
        None => elsewhere(host, &args).ok_or_else(|| ApiError::new("no_window", "Open Curio's window, then connect again"))?,
    };
    // only a Deny and a question left unanswered count as refusals; a window still starting or asking something else is told as it is
    let allowed = match answer {
        Ok(a) if a["ok"] == true => a,
        Ok(_) => {
            refused(gate);
            return Err(ApiError::new("not_allowed", "Not allowed in Curio"));
        }
        Err(e) if e.code == "busy" && e.message.contains("Nobody answered") => {
            refused(gate);
            return Err(ApiError::new("not_allowed", "Nobody answered in Curio"));
        }
        Err(e) => return Err(e),
    };
    *gate.state.lock().unwrap_or_else(|e| e.into_inner()) = (0, None);
    let libraries: Vec<String> = allowed["libraries"].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).filter(|id| list.iter().any(|(k, _)| k == id)).map(String::from).collect()).unwrap_or_default();
    let (mut token, secret) = tokens::make(&name, PERMS.to_vec(), vec![origin.to_string()], libraries.clone());
    token.paired = true;
    let id = token.id.clone();
    host.add_token(token).map_err(|e| ApiError::new("failed", e))?;
    Ok((json!({ "token": secret, "name": name, "libraries": libraries }), id))
}

fn refused(gate: &Gate) {
    let mut st = gate.state.lock().unwrap_or_else(|e| e.into_inner());
    st.0 += 1;
    if st.0 >= REFUSALS {
        *st = (0, Some(Instant::now() + CLOSED_FOR));
    }
}

/// The question put to a window of another process, when this one has none (a background holder answering on the port).
fn elsewhere(host: &dyn Host, args: &Value) -> Option<ApiResult> {
    let cfg = host.config();
    let h = super::holders(host, &known(host, &cfg)).into_iter().map(|(_, h)| h).filter(|h| !h.background && !h.window.is_empty() && h.port != 0).max_by_key(|h| (h.focused, h.opened))?;
    let reply = crate::core::library::instance::tell(&h, "ask", &json!({"op": "pair.ask", "args": args})).ok()?;
    Some(match reply.get("error") {
        Some(e) => Err(serde_json::from_value(e.clone()).unwrap_or_else(|_| ApiError::new("failed", e.to_string()))),
        None => Ok(reply.get("ok").cloned().unwrap_or(Value::Null)),
    })
}
