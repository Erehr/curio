//! Loading and saving the app's own settings (`app.json`), where its data is, its installation id and the plugin folders.

use crate::desktop::AppConfig;
use std::path::{Path, PathBuf};
use crate::core::system::logging;
use crate::core::model::*;
use crate::core::system::app_dirs::{config_dir, data_dir};

pub(super) fn load_config() -> AppConfig {
    load_config_from(&config_dir())
}

fn load_config_from(dir: &Path) -> AppConfig {
    let mut c = std::fs::read_to_string(dir.join("app.json")).ok().and_then(|s| parse_config(&s).ok()).unwrap_or_default();
    if let Some(base) = crate::core::system::app_dirs::portable() {
        map_paths(&mut c, |p| crate::core::system::app_dirs::resolve(base, p));
    }
    c
}

/// Reads each known key on its own so one unparseable value falls back without discarding the rest or unknown keys.
fn parse_config(text: &str) -> Result<AppConfig, String> {
    let mut map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut c = AppConfig::default();
    fn take<T: serde::de::DeserializeOwned>(map: &mut serde_json::Map<String, serde_json::Value>, raw: &mut std::collections::BTreeMap<String, serde_json::Value>, key: &str, out: &mut T) {
        let Some(v) = map.remove(key) else { return };
        match serde_json::from_value(v.clone()) {
            Ok(parsed) => *out = parsed,
            Err(_) => {
                raw.insert(key.to_string(), v);
            }
        }
    }
    take(&mut map, &mut c.raw_fallbacks, "libraries", &mut c.libraries);
    take(&mut map, &mut c.raw_fallbacks, "current_library", &mut c.current_library);
    take(&mut map, &mut c.raw_fallbacks, "plugin_dirs", &mut c.plugin_dirs);
    take(&mut map, &mut c.raw_fallbacks, "cache_dir", &mut c.cache_dir);
    take(&mut map, &mut c.raw_fallbacks, "gpu", &mut c.gpu);
    take(&mut map, &mut c.raw_fallbacks, "api_enabled", &mut c.api_enabled);
    if map.contains_key("api_port") {
        take(&mut map, &mut c.raw_fallbacks, "api_port", &mut c.api_port);
    } else if map.contains_key("mcp_port") {
        take(&mut map, &mut c.raw_fallbacks, "mcp_port", &mut c.api_port);
    }
    if let Some(v) = map.remove("api_tokens") {
        if let Some(arr) = v.as_array() {
            for item in arr {
                match serde_json::from_value::<crate::core::api::tokens::ApiToken>(item.clone()) {
                    Ok(t) => c.api_tokens.push(t),
                    Err(_) => c.raw_tokens.push(item.clone()),
                }
            }
        } else {
            c.raw_fallbacks.insert("api_tokens".to_string(), v);
        }
    }
    take(&mut map, &mut c.raw_fallbacks, "mcp_enabled", &mut c.mcp_enabled);
    take(&mut map, &mut c.raw_fallbacks, "mcp_disabled", &mut c.mcp_disabled);
    take(&mut map, &mut c.raw_fallbacks, "link_previews", &mut c.link_previews);
    take(&mut map, &mut c.raw_fallbacks, "installation_id", &mut c.installation_id);
    take(&mut map, &mut c.raw_fallbacks, "close_to_tray", &mut c.close_to_tray);
    take(&mut map, &mut c.raw_fallbacks, "library_looks", &mut c.library_looks);
    take(&mut map, &mut c.raw_fallbacks, "library_ids", &mut c.library_ids);
    take(&mut map, &mut c.raw_fallbacks, "no_browser_start", &mut c.no_browser_start);
    c.extra = map.into_iter().collect();
    Ok(c)
}

fn serialize_config(c: &AppConfig) -> Result<String, serde_json::Error> {
    let mut map = match serde_json::to_value(c)? {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };
    if !c.raw_tokens.is_empty() {
        let arr = map.entry("api_tokens").or_insert_with(|| serde_json::Value::Array(Vec::new()));
        if let Some(list) = arr.as_array_mut() {
            list.extend(c.raw_tokens.iter().cloned());
        }
    }
    if !c.raw_fallbacks.is_empty() {
        if let Ok(serde_json::Value::Object(defaults)) = serde_json::to_value(AppConfig::default()) {
            for (k, raw_v) in &c.raw_fallbacks {
                if map.get(k) == defaults.get(k) {
                    map.insert(k.clone(), raw_v.clone());
                }
            }
        }
    }
    serde_json::to_string_pretty(&map)
}

/// Every path `app.json` holds.
fn map_paths(c: &mut AppConfig, f: impl Fn(&Path) -> PathBuf) {
    for p in c.libraries.iter_mut().chain(c.plugin_dirs.iter_mut()).chain(c.current_library.iter_mut()).chain(c.cache_dir.iter_mut()) {
        *p = f(p);
    }
}

/// Windows are separate processes sharing this file, so it is replaced whole. A file that can't be read back is set aside rather than overwritten.
pub(super) fn save_config(c: &AppConfig) {
    save_config_in(&config_dir(), c)
}

fn save_config_in(dir: &Path, c: &AppConfig) {
    let path = dir.join("app.json");
    let _ = std::fs::create_dir_all(dir);
    match std::fs::read_to_string(&path) {
        Ok(text) if serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&text).is_err() => {
            logging::error("app.json couldn't be read; it was kept as app.json.damaged");
            let _ = std::fs::rename(&path, dir.join("app.json.damaged"));
        }
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            logging::error(&format!("settings not saved: {e}"));
            return;
        }
        _ => {}
    }
    // a portable copy keeps paths on its own drive relative, so they still work under another drive letter
    let mut c = c.clone();
    if let Some(base) = crate::core::system::app_dirs::portable() {
        map_paths(&mut c, |p| crate::core::system::app_dirs::relative_to(base, p).map(PathBuf::from).unwrap_or_else(|| p.to_path_buf()));
    }
    let Ok(s) = serialize_config(&c) else { return };
    let tmp = dir.join(format!("app.json.{}.tmp", std::process::id()));
    if let Err(e) = std::fs::write(&tmp, s).and_then(|_| std::fs::rename(&tmp, &path)) {
        logging::error(&format!("settings not saved: {e}"));
        let _ = std::fs::remove_file(&tmp);
    }
}

/// In portable mode the web view keeps its storage in the portable folder too.
pub(super) fn own_data<'a, R: tauri::Runtime, M: tauri::Manager<R>>(b: tauri::WebviewWindowBuilder<'a, R, M>) -> tauri::WebviewWindowBuilder<'a, R, M> {
    match crate::core::system::app_dirs::webview_dir() {
        Some(d) => b.data_directory(d),
        None => b,
    }
}

pub(super) fn ensure_installation_id() -> String {
    let mut cfg = load_config();
    if cfg.installation_id.is_empty() {
        cfg.installation_id = new_uid();
        save_config(&cfg);
    }
    cfg.installation_id
}

/// Drops plugin folder entries whose path no longer holds a plugin (moved, renamed or deleted) so they don't linger in `app.json`.
pub(super) fn prune_plugin_dirs(c: &mut AppConfig) -> bool {
    let before = c.plugin_dirs.len();
    c.plugin_dirs.retain(|d| has_plugin(d));
    let changed = c.plugin_dirs.len() != before;
    if changed {
        save_config(c);
    }
    changed
}

fn has_plugin(d: &Path) -> bool {
    if !d.is_dir() {
        return false;
    }
    if d.join("plugin.json").is_file() {
        return true;
    }
    std::fs::read_dir(d).ok().into_iter().flatten().flatten().any(|e| {
        let p = e.path();
        p.is_dir() && p.join("plugin.json").is_file()
    })
}

pub(super) fn plugin_dirs(c: &AppConfig) -> Vec<PathBuf> {
    let mut v = c.plugin_dirs.clone();
    v.push(data_dir().join("plugins"));
    v
}
