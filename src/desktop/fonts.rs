//! What the font viewer and the font thumbnails read of a font file (web/src/formats/FontView.vue, fontThumb.js).

use std::path::PathBuf;

use crate::core::metadata::fonts::{facts, source::FileSource};
use crate::desktop::AppState;
use crate::desktop::R;

fn font_path(state: &AppState, id: i64) -> R<PathBuf> {
    let a = state.read(|db| db.asset(id)?.ok_or_else(|| "not found".to_string()))?;
    Ok(PathBuf::from(a.abs_path))
}

/// Everything about one face: names, style, category, scripts, axes, features and the characters as ranges, with the names of a collection's faces.
#[tauri::command]
pub(super) async fn font_describe(state: tauri::State<'_, AppState>, id: i64, face: usize) -> R<serde_json::Value> {
    let path = font_path(&state, id)?;
    let f = facts::read(FileSource::open(&path)?, face, true)?;
    let mut v = f.describe();
    v["face"] = serde_json::json!(face);
    v["faceNames"] = serde_json::json!(if f.faces > 1 { facts::face_names(FileSource::open(&path)?)? } else { vec![] });
    Ok(v)
}

/// The glyph names of some characters, where the font names its glyphs.
#[tauri::command]
pub(super) async fn font_glyph_names(state: tauri::State<'_, AppState>, id: i64, face: usize, chars: Vec<u32>) -> R<Vec<Option<String>>> {
    let path = font_path(&state, id)?;
    facts::glyph_names(FileSource::open(&path)?, face, &chars[..chars.len().min(4096)])
}

/// One face of a collection or a dfont as a font file of its own, which the web view can load.
#[tauri::command]
pub(super) async fn font_face_bytes(state: tauri::State<'_, AppState>, id: i64, face: usize) -> R<tauri::ipc::Response> {
    let path = font_path(&state, id)?;
    Ok(tauri::ipc::Response::new(facts::face_file(FileSource::open(&path)?, face)?))
}
