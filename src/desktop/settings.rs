//! The window's settings and their sync between windows, a library's settings, the app's config as the window reads it, system fonts and auto-import.

use crate::desktop::AppState;
use crate::desktop::R;
use parking_lot::Mutex;
use tauri::Emitter;
use crate::core::api;
use crate::core::library::{changes::Changed, Library, settings::LibrarySettings, changes::Parts};
use crate::core::system::app_dirs::config_dir;
use crate::desktop::api_host::token_json;
use crate::desktop::api_host::claim_api_port;
use crate::desktop::config::save_config;
use crate::desktop::config::load_config;
use crate::desktop::media::media_engine_info;

/// The window settings as this process last read or wrote them; another library's process may have saved others since.
static UI_SEEN: Mutex<Option<String>> = Mutex::new(None);

#[tauri::command]
pub(super) async fn ui_settings_load() -> Option<String> {
    let mut seen = UI_SEEN.lock();
    let text = read_ui_settings();
    // a window opened later doesn't mark another library's save as seen before the others take it
    if seen.is_none() {
        *seen = text.clone();
    }
    text
}

/// The window settings when another process saved them since this one last read or wrote them, with the ones it had then, so a window can take only what changed.
#[tauri::command]
pub(super) async fn ui_settings_changed() -> R<Option<serde_json::Value>> {
    let mut seen = UI_SEEN.lock();
    let Some(text) = read_ui_settings() else { return Ok(None) };
    if seen.as_deref() == Some(text.as_str()) {
        return Ok(None);
    }
    let base = seen.replace(text.clone());
    Ok(Some(serde_json::json!({ "text": text, "base": base })))
}

pub(crate) fn read_ui_settings() -> Option<String> {
    std::fs::read_to_string(config_dir().join("ui-settings.json")).ok()
}

#[tauri::command]
pub(super) async fn ui_settings_save(json: String) -> R<()> {
    let mut seen = UI_SEEN.lock();
    std::fs::create_dir_all(config_dir()).map_err(|e| e.to_string())?;
    let tmp = config_dir().join("ui-settings.json.tmp");
    std::fs::write(&tmp, &json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, config_dir().join("ui-settings.json")).map_err(|e| e.to_string())?;
    *seen = Some(json);
    Ok(())
}

/// The window material saved in the UI settings, so a new window starts with it instead of waiting for its page.
pub(super) fn saved_look() -> crate::desktop::system::material::Look {
    let ui: serde_json::Value = read_ui_settings().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let mut look: crate::desktop::system::material::Look = serde_json::from_value(serde_json::json!({
        "material": ui["windowMaterial"],
    }))
    .unwrap_or_default();
    look.dark = match ui["themeMode"].as_str() {
        Some("dark") => Some(true),
        Some("light") => Some(false),
        _ => None,
    };
    look
}

/// The calling window's material and tint, set from its page whenever the settings or theme change.
#[tauri::command]
pub(super) fn window_look(window: tauri::WebviewWindow, look: crate::desktop::system::material::Look) {
    crate::desktop::system::material::apply(&window, look)
}

/// One window's settings changed: keep them and tell the others (Settings has its own window).
#[tauri::command]
pub(super) fn ui_sync(app: tauri::AppHandle, from: String, json: String) -> R<()> {
    let _ = app.emit("ui-sync", serde_json::json!({ "from": from, "json": json }));
    Ok(())
}

#[tauri::command]
pub(super) fn library_settings(state: tauri::State<AppState>, set: Option<serde_json::Map<String, serde_json::Value>>) -> R<LibrarySettings> {
    state.with_lib(|l| {
        if let Some(patch) = set {
            l.patch_settings(patch);
        }
        Ok(l.settings())
    })
}

#[tauri::command]
pub(super) async fn app_config(app: tauri::AppHandle, state: tauri::State<'_, AppState>, set: Option<serde_json::Value>) -> R<serde_json::Value> {
    let mut cfg = load_config();
    if let Some(v) = set {
        if let Some(g) = v["gpu"].as_bool() {
            cfg.gpu = g;
        }
        if let Some(t) = v["closeToTray"].as_bool() {
            cfg.close_to_tray = t;
        }
        if let Some(on) = v["autostart"].as_bool() {
            crate::desktop::system::autostart::set(on)?;
        }
        if let Some(on) = v["browserStart"].as_bool() {
            crate::desktop::system::scheme::set(on)?;
            cfg.no_browser_start = !on;
        }
        if let Some(list) = v["mcpDisabled"].as_array() {
            cfg.mcp_disabled = list.iter().filter_map(|x| x.as_str().map(String::from)).collect();
        }
        if let Some(on) = v["mcpEnabled"].as_bool() {
            cfg.mcp_enabled = on;
        }
        if let Some(on) = v["apiEnabled"].as_bool() {
            cfg.api_enabled = on;
        }
        if let Some(p) = v["apiPort"].as_u64() {
            cfg.api_port = Some(p.clamp(1024, 65535) as u16);
        }
        if let Some(on) = v["linkPreviews"].as_bool() {
            cfg.link_previews = on;
            crate::core::thumbs::links::set_link_previews(on);
            if on {
                let _ = state.with_lib(|lib| {
                    lib.job(|db, _| {
                        db.conn.execute("UPDATE assets SET thumb_state = 0, thumb_ver = thumb_ver + 1 WHERE ext IN ('url', 'webloc', 'website') AND thumb_state = 2", []).map_err(|e| e.to_string())?;
                        Ok(Changed::parts(Parts::SIDEBAR))
                    });
                    lib.kick_thumbs();
                    Ok(())
                });
            }
        }
        save_config(&cfg);
        claim_api_port(&app, &cfg);
    }
    let port = cfg.api_port();
    Ok(serde_json::json!({
        "gpu": cfg.gpu, "closeToTray": cfg.close_to_tray, "autostart": crate::desktop::system::autostart::enabled(), "portable": crate::core::system::app_dirs::portable().is_some(), "linkPreviews": cfg.link_previews,
        "browserStart": !cfg.no_browser_start, "browserStartAllowed": crate::desktop::system::scheme::allowed(),
        "apiEnabled": cfg.api_enabled, "apiPort": port, "apiUrl": format!("http://127.0.0.1:{port}{}", api::http::PREFIX),
        "apiServing": state.api.lock().as_ref().is_some_and(|s| s.port == port),
        "apiPermissions": api::Perm::FOR_TOKENS.iter().map(|p| p.name()).collect::<Vec<_>>(),
        "apiTokens": cfg.api_tokens.iter().map(token_json).collect::<Vec<_>>(),
        "mcpEnabled": cfg.mcp_enabled, "mcpTools": api::mcp::tool_names(), "mcpDisabled": cfg.mcp_disabled, "mcpUrl": format!("http://127.0.0.1:{port}/mcp"),
        "mediaEngine": media_engine_info(),
    }))
}

/// Installed font families, for the font setting.
#[tauri::command]
pub(super) async fn system_fonts() -> Vec<String> {
    crate::core::system::filesystem::font_families()
}

/// Which watches can run here, and why not.
fn auto_import_report(lib: &Library) -> serde_json::Value {
    use crate::core::import::auto_import as ai;
    let cfg = lib.settings().auto_import;
    let approved = ai::load_approvals();
    let watches: Vec<serde_json::Value> = cfg
        .watches
        .iter()
        .map(|w| {
            serde_json::json!({
                "id": w.id,
                "allowed": approved.contains(&ai::fingerprint(w)),
                "problem": ai::watch_problem(&w.path, &lib.paths().root),
            })
        })
        .collect();
    serde_json::json!({ "config": cfg, "watches": watches })
}

#[tauri::command]
pub(super) async fn auto_import_get(state: tauri::State<'_, AppState>) -> R<serde_json::Value> {
    state.with_lib(|l| Ok(auto_import_report(l)))
}

/// Save the watches. What the person just made or changed here counts as allowed; a watch that
/// arrived with the library and wasn't touched keeps waiting for `auto_import_allow`.
#[tauri::command]
pub(super) async fn auto_import_save(state: tauri::State<'_, AppState>, config: crate::core::import::auto_import::AutoImportConfig) -> R<serde_json::Value> {
    use crate::core::import::auto_import as ai;
    state.with_lib(|l| {
        let s = l.settings();
        let before: std::collections::HashSet<String> = s.auto_import.watches.iter().map(ai::fingerprint).collect();
        let fresh: Vec<String> = config.watches.iter().map(ai::fingerprint).filter(|f| !before.contains(f)).collect();
        ai::allow(&fresh);
        let mut patch = serde_json::Map::new();
        patch.insert("auto_import".to_string(), serde_json::to_value(&config).map_err(|e| e.to_string())?);
        l.patch_settings(patch);
        Ok(auto_import_report(l))
    })
}

#[tauri::command]
pub(super) async fn auto_import_allow(state: tauri::State<'_, AppState>, id: String) -> R<serde_json::Value> {
    use crate::core::import::auto_import as ai;
    state.with_lib(|l| {
        if let Some(w) = l.settings().auto_import.watches.iter().find(|w| w.id == id) {
            ai::allow(&[ai::fingerprint(w)]);
        }
        Ok(auto_import_report(l))
    })
}
