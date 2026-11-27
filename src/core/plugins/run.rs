//! Plugin programs run: extractors, commands and their replies.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use serde::Deserialize;
use crate::core::system::process;
use crate::core::plugins::manifest::ExtractorDecl;
use crate::core::plugins::manifest::JsonSource;

/// The largest JSON file a `json` source reads.
const MAX_JSON_SOURCE: u64 = 8 * 1024 * 1024;

fn read_json_source(j: &JsonSource, parts: &[crate::core::metadata::readers::PartFile]) -> Result<Vec<(String, serde_json::Value)>, String> {
    let file = j.file.to_lowercase();
    let Some(p) = parts.iter().find(|p| p.role == j.role && p.name.to_lowercase().ends_with(&file)) else { return Ok(vec![]) };
    let len = std::fs::metadata(&p.path).map_err(|e| e.to_string())?.len();
    if len > MAX_JSON_SOURCE {
        return Err(format!("{} is larger than 8 MB", p.name));
    }
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p.path).map_err(|e| e.to_string())?).map_err(|e| format!("{} is not JSON: {e}", p.name))?;
    let mut out: Vec<(String, serde_json::Value)> = j.fields.iter().filter_map(|(k, ptr)| v.pointer(ptr).filter(|x| !x.is_null()).map(|x| (k.clone(), x.clone()))).collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Run a declared extractor on one file. Returns field key → JSON value.
pub fn run_extractor(plugin: &str, x: &ExtractorDecl, path: &Path, ext: &str, parts: &[crate::core::metadata::readers::PartFile]) -> Result<Vec<(String, serde_json::Value)>, String> {
    if !x.exts.is_empty() && !x.exts.iter().any(|e| e == ext) {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    if let Some(j) = &x.json {
        out.extend(read_json_source(j, parts)?);
    }
    if (!x.png_text.is_empty() || !x.regex.is_empty()) && ext == "png" {
        let chunks = crate::core::metadata::extract::png_text_chunks(path)?;
        for (chunk, key) in &x.png_text {
            if let Some(t) = chunks.get(chunk) {
                out.push((key.clone(), serde_json::Value::String(t.clone())));
            }
        }
        for r in &x.regex {
            if let (Some(t), Ok(re)) = (chunks.get(&r.chunk), regex::Regex::new(&r.pattern)) {
                if let Some(c) = re.captures(t) {
                    if let Some(m) = c.get(1) {
                        out.push((r.field.clone(), serde_json::Value::String(m.as_str().to_string())));
                    }
                }
            }
        }
    }
    if !x.command.is_empty() {
        let listed: Vec<serde_json::Value> = parts.iter().map(|p| serde_json::json!({"role": p.role, "path": p.path})).collect();
        let argv = process::expand(&x.command, &[("input", path.to_string_lossy().to_string()), ("parts", serde_json::Value::Array(listed).to_string())]);
        let (prog, args) = argv.split_first().ok_or("empty command")?;
        let token = crate::core::api::programs::issue(plugin);
        let r = process::run_in(prog, args, None, Duration::from_secs(30), x.dir.as_deref(), &token.env)?;
        if r.status != Some(0) {
            return Err(format!("extractor exited with {:?}: {}", r.status, r.stderr.trim()));
        }
        let v: serde_json::Value = serde_json::from_slice(&r.stdout).map_err(|e| format!("extractor output is not JSON: {e}"))?;
        if let Some(o) = v.as_object() {
            for (k, v) in o {
                out.push((k.clone(), v.clone()));
            }
        }
    }
    Ok(out)
}

/// What a program answers: changes for the app to make to the items it was given, and a message to show.
#[derive(Debug, Default, Deserialize)]
pub struct PluginReply {
    #[serde(default)]
    pub set: Vec<AssetChange>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct AssetChange {
    pub id: String,
    #[serde(default)]
    pub fields: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub rating: Option<i64>,
    #[serde(default)]
    pub annotation: Option<String>,
}

/// Runs one of `plugin`'s programs with a JSON payload on stdin and a run token for the API in its environment. Returns the parsed reply.
pub fn run_command(plugin: &str, argv: &[String], dir: Option<&Path>, payload: &serde_json::Value, timeout: Duration) -> Result<PluginReply, String> {
    let (prog, args) = argv.split_first().ok_or("empty command")?;
    let data = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
    let token = crate::core::api::programs::issue(plugin);
    let r = process::run_in(prog, args, Some(&data), timeout, dir, &token.env)?;
    if r.status != Some(0) {
        return Err(format!("exited with {:?}: {}", r.status, r.stderr.trim()));
    }
    let s = String::from_utf8_lossy(&r.stdout);
    let s = s.trim();
    if s.is_empty() {
        return Ok(PluginReply::default());
    }
    serde_json::from_str(s).map_err(|e| format!("reply is not valid JSON: {e}"))
}

pub fn json_to_raw(v: &serde_json::Value) -> crate::core::model::RawValue {
    use crate::core::model::RawValue;
    match v {
        serde_json::Value::Bool(b) => RawValue::Bool(*b),
        serde_json::Value::Number(n) => RawValue::Num(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => RawValue::Text(s.clone()),
        serde_json::Value::Array(a) => RawValue::List(a.iter().map(json_to_raw).collect()),
        other => RawValue::Text(other.to_string()),
    }
}
