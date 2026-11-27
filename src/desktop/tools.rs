//! Curio's tool windows: Duplicates, Batch rename and Compare, each the window's own page (`web/src/tools/`) in a window of its own.

use crate::desktop::app_windows::{raise, reveal_fallback};
use crate::desktop::background;
use crate::desktop::config::own_data;
use crate::desktop::protocol::browser_args;
use crate::desktop::settings::saved_look;
use crate::desktop::{AppState, ToolWin, R};
use tauri::{Emitter, Manager};

/// Tool windows are labelled like this.
const TOOL_WINDOW_PREFIX: &str = "tool";

/// The tools, with their windows' sizes; one window each, raised and told the new run when opened again.
const TOOLS: &[(&str, f64, f64)] = &[("duplicates", 1280.0, 800.0), ("rename", 1080.0, 700.0), ("compare", 1100.0, 760.0)];

/// Opens a tool's window for these items (the API's item ids), or brings it forward with them.
#[tauri::command]
pub(super) async fn tool_window(app: tauri::AppHandle, state: tauri::State<'_, AppState>, tool: String, title: String, items: Option<Vec<String>>, run: Option<serde_json::Value>) -> R<()> {
    let &(id, width, height) = TOOLS.iter().find(|t| t.0 == tool).ok_or_else(|| format!("There is no tool {tool}"))?;
    let mut run = match run {
        Some(serde_json::Value::Object(o)) => serde_json::Value::Object(o),
        _ => serde_json::json!({}),
    };
    run["items"] = serde_json::json!(items.unwrap_or_default());
    let open: Vec<String> = state.tool_windows.lock().iter().filter(|(_, x)| x.tool == id).map(|(l, _)| l.clone()).collect();
    for label in open {
        match app.get_webview_window(&label) {
            Some(existing) => {
                raise(&existing);
                if let Some(x) = state.tool_windows.lock().get_mut(&label) {
                    x.run = run.clone();
                }
                let _ = app.emit_to(label.as_str(), "tool-rerun", serde_json::json!({ "to": label, "run": run }));
                return Ok(());
            }
            None => {
                state.tool_windows.lock().remove(&label);
            }
        }
    }
    let label = format!("{TOOL_WINDOW_PREFIX}{}", fastrand::u32(..));
    state.tool_windows.lock().insert(label.clone(), ToolWin { tool: id.to_string(), run });
    let url = format!("index.html?tool={id}");
    let built = crate::desktop::system::material::sized(crate::desktop::system::material::frame(own_data(tauri::WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::App(url.into())))), &app, width, height)
        .focused(!background())
        .always_on_bottom(background())
        .visible(false)
        .title(&title)
        .additional_browser_args(&browser_args())
        .min_inner_size(320.0, 240.0)
        .build()
        .map_err(|e| e.to_string())?;
    crate::desktop::system::material::apply(&built, saved_look());
    reveal_fallback(&built);
    Ok(())
}

/// What a tool window shows and what it was opened for.
#[tauri::command]
pub(super) fn tool_window_info(window: tauri::WebviewWindow, state: tauri::State<AppState>) -> Option<serde_json::Value> {
    let w = state.tool_windows.lock().get(window.label()).cloned()?;
    Some(serde_json::json!({ "tool": w.tool, "run": w.run }))
}
