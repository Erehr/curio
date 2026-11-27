//! Programs a plugin starts (actions, URL handlers, extractors, thumbnailers) call the API live: each run gets `CURIO_API_URL` and a token made for it alone, valid until the program ends.
//! The address is this process's own, so a program reaches the library it was started for without the router.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use super::{Caller, Host};

/// Run tokens by their hash, each naming the plugin it was made for.
static RUNS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
static URL: RwLock<Option<String>> = RwLock::new(None);

/// A run's token; the token stops working when this is dropped.
pub struct RunToken {
    hash: String,
    /// `CURIO_API_TOKEN`, `CURIO_PLUGIN` and, once this process answers programs, `CURIO_API_URL`, for the program's environment.
    pub env: Vec<(String, String)>,
}

impl Drop for RunToken {
    fn drop(&mut self) {
        if let Some(m) = RUNS.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            m.remove(&self.hash);
        }
    }
}

/// A token for one run of `plugin`'s program.
pub fn issue(plugin: &str) -> RunToken {
    let secret = format!("curio_run_{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    let hash = super::tokens::hash(&secret);
    RUNS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(hash.clone(), plugin.to_string());
    // Python writing its bytecode into the plugin folder would change what was approved
    let mut env = vec![("CURIO_API_TOKEN".to_string(), secret.clone()), ("CURIO_PLUGIN".into(), plugin.to_string()), ("PYTHONDONTWRITEBYTECODE".into(), "1".into())];
    if let Some(url) = URL.read().unwrap_or_else(|e| e.into_inner()).clone() {
        env.push(("CURIO_API_URL".into(), url));
    }
    RunToken { hash, env }
}

/// The plugin a run token was made for, while its program runs.
pub fn plugin_of(secret: &str) -> Option<String> {
    let h = super::tokens::hash(secret);
    RUNS.lock().unwrap_or_else(|e| e.into_inner()).as_ref()?.get(&h).cloned()
}

/// A plugin's caller, with what was granted to it now: a permission taken away mid-run counts at once.
pub fn caller_for(host: &dyn Host, plugin: &str) -> Option<Caller> {
    let mut found = None;
    let _ = host.with_library(&mut |lib| found = lib.plugins().into_iter().find(|p| p.manifest.id == plugin && p.active()));
    found.map(|p| Caller::plugin(&p))
}

/// Answers programs on a port of its own, once per process.
pub fn start(host: Arc<dyn Host>) -> Result<u16, String> {
    if let Some(u) = URL.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        return Ok(u.trim_start_matches("http://127.0.0.1:").split('/').next().and_then(|p| p.parse().ok()).unwrap_or(0));
    }
    let server = super::http::start_programs(host)?;
    let port = server.port;
    *URL.write().unwrap_or_else(|e| e.into_inner()) = Some(format!("http://127.0.0.1:{port}{}", super::http::PREFIX));
    // it lives as long as the process
    std::mem::forget(server);
    Ok(port)
}
