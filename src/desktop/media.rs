//! Video and sound: frames as thumbnails, codecs and the media engine, the pages' thumbnail jobs and subtitles.

use crate::desktop::R;
use crate::desktop::AppState;
use std::path::{Path, PathBuf};
use base64::Engine;
use crate::core::library::{changes::Aspect, changes::Changed};
use crate::core::model::*;
use crate::core::library::edits;
use crate::desktop::plugin_manager::ffmpeg_missing_message;

/// Use a picture as the thumbnail of these items. Refresh Thumbnail goes back to the automatic one.
fn apply_thumbnail(state: &AppState, ids: Vec<i64>, img: image::DynamicImage) -> R<()> {
    let dir = state.thumbs_dir.read().clone().ok_or("No library is open")?;
    let t = crate::core::thumbs::custom_thumbnail(&img)?;
    state.write(move |db| edits::custom_thumbnail(db, &dir, &ids, &t))
}

#[tauri::command]
pub(super) async fn clipboard_thumbnail(state: tauri::State<'_, AppState>, ids: Vec<i64>) -> R<()> {
    let img = image::load_from_memory(&crate::core::import::clipboard::clipboard_image_png()?).map_err(|e| e.to_string())?;
    apply_thumbnail(&state, ids, img)
}

/// A video's frame at a moment: as its thumbnail, on the clipboard, or saved beside it.
#[tauri::command]
pub(super) async fn video_frame_op(state: tauri::State<'_, AppState>, id: i64, time: f64, op: String) -> R<String> {
    let a = state.read(|db| db.asset(id)?.ok_or_else(|| "not found".to_string()))?;
    let src = PathBuf::from(&a.abs_path);
    let frame = frame_with_ffmpeg(&state, &src, time)?;
    let result = (|| -> R<String> {
        match op.as_str() {
            "thumb" => {
                apply_thumbnail(&state, vec![id], frame.clone())?;
                Ok("Thumbnail set".into())
            }
            "copy" => {
                let img = frame.to_rgba8();
                let (w, h) = (img.width() as usize, img.height() as usize);
                let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
                cb.set_image(arboard::ImageData { width: w, height: h, bytes: std::borrow::Cow::Owned(img.into_raw()) }).map_err(|e| e.to_string())?;
                Ok("Copied to clipboard".into())
            }
            "save" => {
                state.read(|db| db.require_writable(a.root_id))?;
                // next to the video, as a JPEG at 95%, in the same folders with the same source link
                let stem = src.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "frame".into());
                let managed = state.read(|db| Ok(db.root(a.root_id).is_some_and(|r| r.kind == RootKind::Managed)))?;
                if managed {
                    // a copied video's frame becomes a copied item of its own, in each of the video's folders
                    let img = frame.to_rgb8();
                    let mut bytes = Vec::new();
                    let enc = jpeg_encoder::Encoder::new(&mut bytes, FRAME_JPEG_QUALITY);
                    enc.encode(img.as_raw(), img.width() as u16, img.height() as u16, jpeg_encoder::ColorType::Rgb).map_err(|e| e.to_string())?;
                    let name = crate::core::media::frame_file_name(&stem, time);
                    let source_url = a.source_url.clone();
                    let shown = name.clone();
                    state.with_lib(|l| {
                        let style = l.settings().name_suffix;
                        let lp = l.paths().clone();
                        l.job(move |db, _| {
                            let folders: Vec<i64> = db.asset_folders(id)?.into_iter().map(|f| f.id).collect();
                            let url = (!source_url.is_empty()).then_some(source_url.as_str());
                            let new = crate::core::import::import_bytes(db, &lp, &name, &bytes, url, folders.first().copied(), style)?;
                            db.tx(|db| {
                                for f in folders.iter().skip(1) {
                                    db.add_to_folder(&[new], *f)?;
                                }
                                Ok(())
                            })?;
                            Ok(Changed::added(&[new], "window"))
                        });
                        l.kick_thumbs();
                        Ok(())
                    })?;
                    return Ok(format!("Saved {shown}"));
                }
                let dir = src.parent().ok_or("no folder")?.to_path_buf();
                let wanted = crate::core::media::frame_file_name(&stem, time);
                let mut dest = dir.join(&wanted);
                let mut n = 2;
                while dest.exists() {
                    dest = dir.join(wanted.replace(".jpg", &format!(" {n}.jpg")));
                    n += 1;
                }
                let img = frame.to_rgb8();
                let enc = jpeg_encoder::Encoder::new_file(&dest, FRAME_JPEG_QUALITY).map_err(|e| e.to_string())?;
                enc.encode(img.as_raw(), img.width() as u16, img.height() as u16, jpeg_encoder::ColorType::Rgb).map_err(|e| e.to_string())?;
                // just after the video when sorted by date
                if let Ok(t) = std::fs::metadata(&src).and_then(|m| m.modified()) {
                    if let Ok(f) = std::fs::OpenOptions::new().write(true).open(&dest) {
                        let _ = f.set_modified(t + std::time::Duration::from_millis(1));
                    }
                }
                let source_url = a.source_url.clone();
                let name = dest.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                let added = dest.clone();
                state.with_lib(|l| {
                    l.write(move |db, _| {
                        let new = crate::core::scan::add_loose_files(db, &[added], &[])?;
                        for f in db.asset_folders(id)? {
                            db.add_to_folder(&new, f.id)?;
                        }
                        if !source_url.is_empty() {
                            db.set_source_url(&new, &source_url)?;
                        }
                        Ok(Changed::added(&new, "window"))
                    });
                    l.kick_thumbs();
                    Ok(())
                })?;
                Ok(format!("Saved {name}"))
            }
            other => Err(format!("unknown frame action {other}")),
        }
    })();
    result
}

/// A picture as base64 or a data: address, as plugin pages send them.
fn decode_picture(data: &str) -> R<image::DynamicImage> {
    let b64 = data.split_once(";base64,").map_or(data, |(_, b)| b);
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|e| e.to_string())?;
    image::load_from_memory(&bytes).map_err(|e| e.to_string())
}

fn frame_with_ffmpeg(state: &AppState, src: &Path, time: f64) -> R<image::DynamicImage> {
    if crate::core::media::engine() != crate::core::media::Engine::Ffmpeg {
        return Err(ffmpeg_missing_message());
    }
    let tmp = state.tmp_dir.read().clone().unwrap_or_else(std::env::temp_dir);
    let _ = std::fs::create_dir_all(&tmp);
    let out = tmp.join(format!("frame_{}.png", fastrand::u64(..)));
    crate::core::media::ffmpeg::video_frame(src, &out, FRAME_MAX_WIDTH, Some(time))?;
    let img = image::open(&out).map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&out);
    img
}

/// What the window found this computer's player can read, by the names `mediaEngine.js` asks about.
#[tauri::command]
pub(super) fn media_codecs_found(plays: Vec<String>) {
    crate::core::media::ffmpeg::set_player_codecs(plays);
}

/// Which FFmpeg reads video and sound: its path, none when it isn't found.
pub(super) fn media_engine_info() -> serde_json::Value {
    match crate::core::media::ffmpeg::ffmpeg().filter(|_| crate::core::media::engine() == crate::core::media::Engine::Ffmpeg) {
        Some(p) => serde_json::json!({ "engine": "ffmpeg", "path": p.display().to_string().replace(r"\\?\", "") }),
        None => serde_json::json!({ "engine": "off", "path": null }),
    }
}

/// Items whose thumbnails plugin pages make, while a window is open (see web/src/plugins/pageJobs.js).
#[tauri::command]
pub(super) async fn page_jobs(state: tauri::State<'_, AppState>, limit: i64, skip: Option<Vec<i64>>) -> R<Vec<serde_json::Value>> {
    let skip = skip.unwrap_or_default();
    let rows = state.read(|db| db.page_jobs(limit, &skip))?;
    let contrib = state.with_lib(|l| Ok(l.contributions()))?;
    Ok(rows
        .into_iter()
        .filter_map(|(id, uid, ext)| contrib.page_thumb(&ext).map(|t| serde_json::json!({ "id": id, "uid": uid, "ext": ext, "plugin": t.plugin, "page": t.page, "size": t.size })))
        .collect())
}

/// A plugin page's thumbnail, or none: the system's thumbnailer then gets a try.
#[tauri::command]
pub(super) async fn page_job_done(state: tauri::State<'_, AppState>, id: i64, image: Option<String>, width: Option<i64>, height: Option<i64>) -> R<()> {
    window_thumb_done(&state, id, image, width, height, None)
}

fn window_thumb_done(state: &AppState, id: i64, image: Option<String>, width: Option<i64>, height: Option<i64>, duration: Option<f64>) -> R<()> {
    let a = state.read(|db| db.asset(id)?.ok_or_else(|| "not found".to_string()))?;
    let dir = state.thumbs_dir.read().clone().ok_or("No library is open")?;
    let picture = image
        .and_then(|data| decode_picture(&data).ok())
        .or_else(|| crate::core::thumbs::shell::thumbnail(Path::new(&a.abs_path), crate::core::thumbs::shell::NATIVE_THUMB_DEFAULT).ok());
    let dest = crate::core::thumbs::thumb_path(&dir, &a.uid, a.thumb_ver);
    let written = picture.and_then(|p| crate::core::thumbs::write_custom_thumbnail(&p, &dest).ok());
    if written.is_some() {
        crate::core::thumbs::remove_older(&dir, &a.uid, a.thumb_ver);
    }
    let visual = matches!(a.kind, crate::core::model::AssetKind::Image | crate::core::model::AssetKind::Video);
    let dims = visual.then(|| width.zip(height).filter(|(w, h)| *w > 0 && *h > 0)).flatten();
    let aspect = written.as_ref().map(|(_, _, asp)| *asp).filter(|a| a.is_finite() && *a > 0.0);
    state.write(move |db| {
        match &written {
            Some((palette, look, _)) => db.set_thumb_done(id, true, dims, Some(palette), Some(look))?,
            None => db.set_thumb_done(id, false, dims, None, None)?,
        }
        if let Some(d) = duration.filter(|d| d.is_finite() && *d > 0.0) {
            db.set_duration(id, d)?;
        }
        if let Some((w, h)) = dims {
            db.conn
                .execute("UPDATE assets SET width = ?, height = ?, aspect = ? WHERE id = ?", rusqlite::params![w, h, w as f64 / h as f64, id])
                .map_err(|e| e.to_string())?;
        } else if let Some(asp) = aspect {
            // plugin-rendered covers (books, PDFs, comics, 3D) set their grid aspect without storing thumbnail dimensions
            db.conn
                .execute("UPDATE assets SET aspect = ?, width = CASE WHEN kind IN ('image', 'video') THEN width ELSE NULL END, height = CASE WHEN kind IN ('image', 'video') THEN height ELSE NULL END WHERE id = ?", rusqlite::params![asp, id])
                .map_err(|e| e.to_string())?;
        }
        // a new shape is asked for in a while, not with a search of the whole library for each cover
        Ok(if dims.is_some() || aspect.is_some() { Changed::reshaped(&[id]) } else { Changed::shown(&[id], Aspect::Thumbnail) })
    })
}

/// The copy FFmpeg made for the player, of a file it can't read itself.
pub(super) fn copy_file(st: &AppState, uid: &str) -> Option<PathBuf> {
    Some(st.previews_dir.read().clone()?.join("video").join(format!("{}.mp4", crate::core::thumbs::cache_key(uid))))
}

const FRAME_MAX_WIDTH: u32 = 16384;

const FRAME_JPEG_QUALITY: u8 = 92;

#[tauri::command]
pub(super) async fn subtitle_tracks(state: tauri::State<'_, AppState>, id: i64) -> R<Vec<serde_json::Value>> {
    let a = state.read(|db| db.asset(id)?.ok_or_else(|| "not found".to_string()))?;
    Ok(crate::core::media::sibling_subtitles(Path::new(&a.abs_path)).into_iter().map(|(name, text)| serde_json::json!({ "name": name, "text": text })).collect())
}

#[tauri::command]
pub(super) async fn subtitle_load(state: tauri::State<'_, AppState>, id: i64, path: String) -> R<serde_json::Value> {
    let a = state.read(|db| db.asset(id)?.ok_or_else(|| "not found".to_string()))?;
    let why = state.read(|db| Ok(db.writes_beside(a.root_id).err()))?;
    let (name, text, saved) = crate::core::media::adopt_subtitle(Path::new(&a.abs_path), Path::new(&path), why.is_none())?;
    Ok(serde_json::json!({ "name": name, "text": text, "saved": saved, "readOnly": why.is_some(), "why": why }))
}
