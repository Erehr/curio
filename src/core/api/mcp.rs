//! MCP (Model Context Protocol, streamable HTTP, stateless JSON responses) over the registry: every operation marked for MCP is a tool.

use base64::Engine;
use serde_json::{json, Value};

use super::{as_file, handle, registry, Caller, Host};

pub const PROTOCOL_VERSION: &str = "2025-03-26";

/// Every tool's name, for the settings page.
pub fn tool_names() -> Vec<String> {
    registry().iter().filter(|o| o.mcp).map(|o| o.mcp_name()).collect()
}

fn tools(caller: &Caller, off: &[String]) -> Vec<Value> {
    registry()
        .iter()
        .filter(|o| o.mcp && o.callers.contains(&caller.kind) && caller.may(o.perm))
        .filter(|o| !off.contains(&o.mcp_name()))
        .map(|o| json!({"name": o.mcp_name(), "description": o.doc, "inputSchema": o.public_params()}))
        .collect()
}

/// One JSON-RPC message; notifications get no reply.
pub fn answer(host: &dyn Host, caller: &Caller, m: &Value) -> Option<Value> {
    let id = m.get("id").cloned()?;
    let method = m["method"].as_str().unwrap_or("");
    let result: Result<Value, (i64, String)> = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": {"name": "Curio", "version": env!("CURIO_VERSION")},
            "instructions": "Local asset library. Items have a stable text `id`. Use item_query with the query language (e.g. `tag = sky AND rating >= 4`, `prompt CONTAINS \"blue hair\"`). library_list shows the libraries; most tools take an optional `library`."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools(caller, &host.config().mcp_disabled)})),
        "tools/call" => {
            let name = m["params"]["name"].as_str().unwrap_or("");
            let args = m["params"].get("arguments").cloned().unwrap_or(json!({}));
            let op = registry().iter().find(|o| o.mcp && o.mcp_name() == name);
            let outcome = match op {
                None => Err(format!("There is no tool {name}")),
                Some(_) if host.config().mcp_disabled.iter().any(|n| n == name) => Err(format!("{name} is turned off in Settings")),
                Some(o) => handle(host, caller, o.name, args).map_err(|e| e.message),
            };
            let bytes = op.is_some_and(|o| o.bytes);
            Ok(match outcome.and_then(|v| content(&v, bytes)) {
                Ok(c) => json!({"content": [c], "isError": false}),
                Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
            })
        }
        _ => Err((-32601, format!("method not found: {method}"))),
    };
    Some(match result {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
        Err((code, msg)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": msg}}),
    })
}

/// Pictures are larger than this only when something is wrong with them; an assistant gets an error instead.
const MAX_IMAGE: u64 = 4 << 20;

/// A tool's answer as MCP content: JSON text, or the picture a file answer points at.
fn content(v: &Value, bytes: bool) -> Result<Value, String> {
    // only an operation that answers with a file can: another's answer may hold what someone stored, `$file` included
    let Some((path, mime)) = as_file(v).filter(|_| bytes) else { return Ok(json!({"type": "text", "text": serde_json::to_string_pretty(v).unwrap_or_default()})) };
    if !mime.starts_with("image/") || std::fs::metadata(&path).map(|m| m.len()).unwrap_or(u64::MAX) > MAX_IMAGE {
        return Err("That file can't be shown here".into());
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(json!({"type": "image", "data": base64::engine::general_purpose::STANDARD.encode(bytes), "mimeType": mime}))
}

/// A request body: one message or a batch.
pub fn answer_body(host: &dyn Host, caller: &Caller, body: &str) -> Option<Value> {
    match serde_json::from_str::<Value>(body) {
        Ok(Value::Array(batch)) => {
            let out: Vec<Value> = batch.iter().filter_map(|m| answer(host, caller, m)).collect();
            (!out.is_empty()).then_some(Value::Array(out))
        }
        Ok(m) => answer(host, caller, &m),
        Err(e) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}})),
    }
}
