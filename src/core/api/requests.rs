//! Adds named by a request id: a repeat within ten minutes answers what the first answered, or that it still runs, and adds nothing.
//! Kept in the process that holds the library, where the add runs.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::{with_library, ApiError, ApiResult, Ctx};

const KEEP: Duration = Duration::from_secs(600);
const MOST: usize = 5000;

/// By library, caller and id: when it began, and its answer once it finished.
static SEEN: LazyLock<Mutex<HashMap<String, (Instant, Option<Value>)>>> = LazyLock::new(Default::default);

fn seen() -> MutexGuard<'static, HashMap<String, (Instant, Option<Value>)>> {
    SEEN.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Clone, Debug)]
pub struct Request {
    key: String,
    pub id: String,
}

pub enum Begun {
    Fresh(Option<Request>),
    Repeat(Value),
}

pub fn begin(c: &Ctx, id: Option<&str>) -> Result<Begun, ApiError> {
    let Some(id) = id.map(str::trim).filter(|i| !i.is_empty()) else { return Ok(Begun::Fresh(None)) };
    if id.len() > 128 {
        return Err(ApiError::new("bad_request", "`requestId` is up to 128 characters"));
    }
    let library = with_library(c.host, |lib| Ok(lib.paths().root.to_string_lossy().to_string()))?;
    let caller = c.caller.token.clone().or_else(|| c.caller.plugin.clone()).unwrap_or_else(|| format!("{:?} {}", c.caller.kind, c.caller.name));
    let key = format!("{library}\n{caller}\n{id}");
    let mut seen = seen();
    seen.retain(|_, (at, _)| at.elapsed() < KEEP);
    if let Some((_, answer)) = seen.get(&key) {
        let mut v = answer.clone().unwrap_or_else(|| json!({"success": true, "queued": true}));
        v["repeat"] = json!(true);
        return Ok(Begun::Repeat(v));
    }
    if seen.len() >= MOST {
        return Err(ApiError::new("busy", "Too many adds are remembered by request id; try again in a few minutes"));
    }
    seen.insert(key.clone(), (Instant::now(), None));
    Ok(Begun::Fresh(Some(Request { key, id: id.to_string() })))
}

impl Request {
    /// Who asked with which id, as a hash: kept in the downloads' file, which must not hold a token's id in the clear.
    pub fn key(&self) -> String {
        blake3::hash(self.key.as_bytes()).to_hex()[..32].to_string()
    }

    /// The add's answer, once it finished.
    pub fn done(&self, answer: &Value) {
        if let Some(e) = seen().get_mut(&self.key) {
            e.1 = Some(answer.clone());
        }
    }

    /// An add that ended in an error with nothing added frees the id for a retry.
    pub fn forget(&self) {
        let mut seen = seen();
        if seen.get(&self.key).is_some_and(|e| e.1.is_none()) {
            seen.remove(&self.key);
        }
    }

    /// The call's own answer: kept unless the add still runs; an error with nothing added frees the id for a retry.
    pub fn finish(&self, out: &ApiResult) {
        match out {
            Ok(v) if v["queued"] == true => {}
            Ok(v) => self.done(v),
            Err(_) => self.forget(),
        }
    }
}
