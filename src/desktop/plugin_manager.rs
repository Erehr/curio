//! The plugin list as the window shows it, plugin and tool operations, and jobs.

use crate::desktop::AppConfig;
use crate::desktop::AppState;
use crate::desktop::R;
use std::path::{Path, PathBuf};
use crate::core::api;
use crate::core::plugins;
use crate::desktop::config::save_config;
use crate::desktop::config::plugin_dirs;
use crate::desktop::config::prune_plugin_dirs;
use crate::desktop::config::load_config;
use crate::desktop::media::media_engine_info;

fn is_linked_external(dir: &Path, cfg: &AppConfig) -> bool {
    if dir.as_os_str().is_empty() {
        return false;
    }
    let target = crate::core::system::filesystem::canonical(dir);
    let parent = dir.parent().map(crate::core::system::filesystem::canonical);
    cfg.plugin_dirs.iter().any(|entry| {
        let ce = crate::core::system::filesystem::canonical(entry);
        ce == target || (!entry.join("plugin.json").is_file() && parent.as_deref() == Some(ce.as_path()))
    })
}

/// Removes `dir` from `cfg.plugin_dirs`, expanding any legacy parent folder entry into its remaining sibling plugin folders.
fn unlink_plugin_dir(dir: &Path, cfg: &mut AppConfig) -> bool {
    let target = crate::core::system::filesystem::canonical(dir);
    let parent = dir.parent().map(crate::core::system::filesystem::canonical);
    let mut next: Vec<PathBuf> = Vec::with_capacity(cfg.plugin_dirs.len());
    let mut changed = false;
    for entry in &cfg.plugin_dirs {
        let ce = crate::core::system::filesystem::canonical(entry);
        if ce == target {
            changed = true;
            continue;
        }
        if !entry.join("plugin.json").is_file() && parent.as_deref() == Some(ce.as_path()) {
            changed = true;
            if let Ok(rd) = std::fs::read_dir(entry) {
                for sub in rd.flatten() {
                    let p = sub.path();
                    if p.is_dir() && p.join("plugin.json").is_file() {
                        let cp = crate::core::system::filesystem::canonical(&p);
                        if cp != target && !next.iter().any(|x| crate::core::system::filesystem::canonical(x) == cp) {
                            next.push(p);
                        }
                    }
                }
            }
            continue;
        }
        if !next.iter().any(|x| crate::core::system::filesystem::canonical(x) == ce) {
            next.push(entry.clone());
        }
    }
    if changed {
        cfg.plugin_dirs = next;
    }
    changed
}

/// The picture beside a built-in plugin in the list.
fn plugin_icon(id: &str) -> &'static str {
    if let Some(f) = crate::core::formats::FAMILIES.iter().find(|f| crate::core::formats::plugin_id(f) == id) {
        return f.icon;
    }
    match id {
        _ => "puzzle",
    }
}

pub(super) fn ffmpeg_missing_message() -> String {
    let dir = crate::core::media::ffmpeg::plugin_folder().map(|d| d.display().to_string()).unwrap_or_default();
    format!("FFmpeg not found. Video thumbnails, video metadata and audio waveforms need it. Put ffmpeg and ffprobe in {dir} or on PATH.")
}

/// What a plugin's details say beyond its manifest: for video and sound, which FFmpeg reads them.
fn plugin_detail(id: &str) -> Option<String> {
    use crate::core::metadata::extract::{AUDIO_OWNER, VIDEO_OWNER};
    if id != VIDEO_OWNER && id != AUDIO_OWNER {
        return None;
    }
    let info = media_engine_info();
    Some(match info["path"].as_str() {
        Some(p) => format!("Uses FFmpeg at {p}."),
        None => ffmpeg_missing_message(),
    })
}

#[tauri::command]
pub(super) async fn plugin_list(state: tauri::State<'_, AppState>) -> R<serde_json::Value> {
    let cfg = load_config();
    state.with_lib(|l| {
        let all = l.plugins();
        // what each active plugin shows or thumbnails itself, by extension
        let claims: Vec<(String, Vec<String>)> = all
            .iter()
            .filter(|p| p.active() && p.source == plugins::manifest::Source::Native)
            .map(|p| (p.manifest.name.clone(), p.manifest.formats.iter().filter(|f| f.viewer.is_some() || f.thumbnail_page.is_some() || f.thumbnail.is_some()).flat_map(|f| f.exts.iter().map(|e| e.trim_start_matches('.').to_lowercase())).collect()))
            .collect();
        Ok(serde_json::Value::Array(
            all.iter()
                .map(|p| {
                    let m = &p.manifest;
                    let own: Vec<String> = m.formats.iter().flat_map(|f| f.exts.iter().map(|e| e.to_lowercase())).collect();
                    let overridden: Vec<serde_json::Value> = if p.source == plugins::manifest::Source::Builtin {
                        claims.iter().filter_map(|(name, exts)| {
                            let taken: Vec<&String> = exts.iter().filter(|e| own.contains(e)).collect();
                            (!taken.is_empty()).then(|| serde_json::json!({"plugin": name, "exts": taken}))
                        }).collect()
                    } else {
                        vec![]
                    };
                    let removable = p.source == plugins::manifest::Source::Native && is_linked_external(&p.dir, &cfg);
                    serde_json::json!({
                        "id": m.id, "name": m.name, "version": m.version, "description": m.description,
                        "source": match p.source { plugins::manifest::Source::Builtin => "builtin", plugins::manifest::Source::Native => "plugin" },
                        "enabled": p.enabled, "configured": p.configured, "removable": removable, "granted": p.granted, "requested": m.permissions, "problem": p.problem, "waitingApproval": p.problem.as_deref() == Some(plugins::loading::WAITING_APPROVAL),
                        "locked": plugins::always_on(&m.id), "dir": p.dir, "icon": m.icon.clone().unwrap_or_else(|| plugin_icon(&m.id).to_string()), "types": plugins::contributions::plugin_types(m), "fields": m.fields.len(), "widgets": m.widgets.len(), "actions": m.actions.len(),
                        "filters": m.filters.len(), "formats": m.formats.iter().map(|f| serde_json::json!({"exts": f.exts, "thumbnail": f.thumbnail.as_ref().map(|t| &t.command)})).collect::<Vec<_>>(),
                        "urls": m.urls.iter().map(|u| serde_json::json!({"title": u.title, "patterns": u.patterns})).collect::<Vec<_>>(), "settings": m.settings, "author": m.author, "detail": plugin_detail(&m.id), "windows": m.windows, "tabs": m.tabs, "network": m.network, "overriddenBy": overridden,
                    })
                })
                .collect(),
        ))
    })
}

#[tauri::command]
pub(super) fn job_cancel(id: String) -> R<()> {
    api::jobs::cancel(&id, None)
}

/// The tools plugins use, with which plugins list each; `set` chooses a tool's program, or with no path forgets the choice.
#[tauri::command]
pub(super) async fn tool_op(state: tauri::State<'_, AppState>, op: String, name: Option<String>, path: Option<String>) -> R<serde_json::Value> {
    if op == "set" {
        crate::core::plugins::tools::set_path(name.as_deref().unwrap_or(""), path.as_deref().map(Path::new))?;
    }
    let users: Vec<(String, Vec<String>)> = state.with_lib(|l| Ok(l.plugins().iter().map(|p| (p.manifest.name.clone(), p.manifest.tools.clone())).collect())).unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        crate::core::plugins::tools::TOOLS.iter().map(|t| {
            let mut v = crate::core::plugins::tools::describe(t, true);
            v["usedBy"] = serde_json::json!(users.iter().filter(|(_, tools)| tools.iter().any(|x| x == t.name)).map(|(n, _)| n).collect::<Vec<_>>());
            v
        }).collect::<Vec<_>>()
    }).await.map(serde_json::Value::Array).map_err(|e| e.to_string())
}

#[tauri::command]
pub(super) async fn plugin_op(state: tauri::State<'_, AppState>, op: String, id: Option<String>, enabled: Option<bool>, granted: Option<Vec<String>>, ids: Option<Vec<i64>>, kind: Option<String>) -> R<String> {
    let mut cfg = load_config();
    prune_plugin_dirs(&mut cfg);
    match op.as_str() {
        "add-dir" => {
            let Some(d) = rfd::FileDialog::new().set_title(crate::desktop::locale::text("dialog.pluginFolder", &[])).pick_folder() else { return Ok(String::new()) };
            if !d.join("plugin.json").is_file() {
                return Err("That folder has no plugin.json. Choose a single plugin folder.".into());
            }
            let loaded = plugins::loading::load_dir(&d).ok_or("Couldn't read plugin.json in that folder.")?;
            if let Some(err) = loaded.problem {
                return Err(err);
            }
            let pid = loaded.manifest.id;
            let asks = loaded.manifest.permissions;
            let canon = crate::core::system::filesystem::canonical(&d);
            if !cfg.plugin_dirs.iter().any(|x| crate::core::system::filesystem::canonical(x) == canon) {
                cfg.plugin_dirs.push(d);
                save_config(&cfg);
            }
            let _ = state.with_lib(|l| Ok(l.reload_plugins(&plugin_dirs(&cfg))))?.recv();
            let _ = state.with_lib(|l| Ok(l.set_plugin(&pid, true, asks, &plugin_dirs(&cfg))))?.recv();
            state.refresh_handlers();
            return Ok(String::new());
        }
        "remove" => {
            let pid = id.clone().ok_or("id")?;
            let dir = state.with_lib(|l| Ok(l.plugins().iter().find(|p| p.manifest.id == pid).map(|p| p.dir.clone())))?.ok_or("That plugin isn't there")?;
            if !unlink_plugin_dir(&dir, &mut cfg) {
                return Err("Only externally added plugins can be removed here.".into());
            }
            save_config(&cfg);
            let _ = state.with_lib(|l| Ok(l.remove_plugin(&pid, &plugin_dirs(&cfg))))?.recv();
            state.refresh_handlers();
            return Ok(String::new());
        }
        // a plugin of the kind asked for, written where the person chooses, that plugin folder added and switched on
        "example" => {
            let kind = kind.unwrap_or_else(|| "window".into());
            let Some(d) = rfd::FileDialog::new().set_title(crate::desktop::locale::text("dialog.examplePlugin", &[])).pick_folder() else { return Ok(String::new()) };
            let (folder, pid) = plugins::examples::write_example(&d, &kind)?;
            let canon = crate::core::system::filesystem::canonical(&folder);
            if !cfg.plugin_dirs.iter().any(|x| crate::core::system::filesystem::canonical(x) == canon) {
                cfg.plugin_dirs.push(folder.clone());
                save_config(&cfg);
            }
            let _ = state.with_lib(|l| Ok(l.reload_plugins(&plugin_dirs(&cfg))))?.recv();
            let asks = state.with_lib(|l| Ok(l.plugins().iter().find(|p| p.manifest.id == pid).map(|p| p.manifest.permissions.clone()).unwrap_or_default()))?;
            let _ = state.with_lib(|l| Ok(l.set_plugin(&pid, true, asks, &plugin_dirs(&cfg))))?.recv();
            state.refresh_handlers();
            return Ok(format!("Example plugin made in {}", folder.display()));
        }
        "reveal" => {
            let pid = id.clone().ok_or("id")?;
            let dir = state.with_lib(|l| Ok(l.plugins().iter().find(|p| p.manifest.id == pid).map(|p| p.dir.clone())))?.ok_or("That plugin isn't there")?;
            crate::core::system::filesystem::reveal(&dir.join("plugin.json"))?;
            return Ok(String::new());
        }
        "set" => {
            let id = id.clone().ok_or("id")?;
            let turning_on = enabled.unwrap_or(false);
            if !turning_on && plugins::always_on(&id) {
                return Err("Built-in formats can't be switched off".into());
            }
            // waited for outside the library lock
            let done = state.with_lib(|l| Ok(l.set_plugin(&id, turning_on, granted.clone().unwrap_or_default(), &plugin_dirs(&cfg))))?;
            let _ = done.recv();
        }
        // a plugin's files read again: what is missing (`kind` "missing"), or everything (`kind` "all"); its readers through the queue, a format's thumbnails made again
        "read-again" => {
            let pid = id.clone().ok_or("id")?;
            let all = kind.as_deref() == Some("all");
            let (exts, readers) = state.with_lib(|l| {
                let p = l.plugins().into_iter().find(|p| p.manifest.id == pid).ok_or("That plugin isn't there")?;
                if !p.active() {
                    return Err(format!("Switch {} on first", p.manifest.name));
                }
                let exts: Vec<String> = p.manifest.formats.iter().flat_map(|f| f.exts.iter().map(|e| e.trim_start_matches('.').to_lowercase())).collect();
                Ok((exts, l.contributions().readers.iter().filter(|r| r.owner == pid).map(|r| r.info(true)).collect::<Vec<_>>()))
            })?;
            let thumbs = state.write_wait(move |db| db.remake_thumbs(&exts, all))?;
            let mode = if all { crate::core::metadata::readers::Mode::All } else { crate::core::metadata::readers::Mode::Missing };
            let hub = state.hub.clone();
            let files = if readers.is_empty() { 0 } else { state.with_lib(|l| Ok(l.rescan(readers, None, mode, Some(hub), None)))?.recv().map_err(|e| e.to_string())??.1 };
            let n = thumbs.max(files);
            return Ok(match n {
                0 => "Nothing to rescan".into(),
                1 => "Rescanning 1 file".into(),
                n => format!("Rescanning {n} files"),
            });
        }
        "ffmpeg-folder" => {
            let dir = crate::core::media::ffmpeg::plugin_folder().ok_or("no data folder")?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            opener::open(&dir).map_err(|e| e.to_string())?;
            return Ok(String::new());
        }
        // waits for the program, so a step of Curio's own actions runs after the one before; its message reaches the window as the program's reply
        "run" => {
            let pid = id.clone().ok_or("id")?;
            let (plugin, action) = pid.split_once("::").ok_or("bad action id")?;
            let rx = state.with_lib(|l| {
                let c = l.contributions();
                let a = c.actions.iter().find(|(p, a)| p == plugin && a.id == action && !a.run.is_empty()).map(|(_, a)| a.clone()).ok_or("That action isn't there")?;
                if let Some(why) = &a.disabled_reason {
                    return Err(format!("{}: {why}", a.title));
                }
                Ok(l.run_action(plugin.to_string(), a, ids.clone().unwrap_or_default()))
            })?;
            return tauri::async_runtime::spawn_blocking(move || rx.recv()).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?.map(|_| String::new());
        }
        _ => {}
    }
    let _ = state.with_lib(|l| Ok(l.reload_plugins(&plugin_dirs(&cfg))))?.recv();
    state.refresh_handlers();
    Ok(String::new())
}
