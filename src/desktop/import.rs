//! Bringing files in: picking and adding them, transfers and their clashes, the clipboard, Eagle libraries, locations, relinking and copies.

use crate::desktop::AppState;
use crate::desktop::R;
use std::path::{Path, PathBuf};
use std::time::Duration;
use crate::core::library::{changes::Changed, LibEvent, changes::Parts};
use crate::core::system::logging;
use crate::core::model::*;
use crate::core::library::edits;
use crate::core::said::Said;
use crate::core::store::OnClash;
use crate::desktop::files::leave_transfer;

#[tauri::command]
pub(super) async fn pick(kind: String, title: Option<String>) -> Option<Vec<String>> {
    let d = match title {
        Some(t) => rfd::FileDialog::new().set_title(t),
        None => rfd::FileDialog::new(),
    };
    let r: Option<Vec<PathBuf>> = match kind.as_str() {
        "folder" => d.pick_folder().map(|p| vec![p]),
        "files" => d.pick_files(),
        "file" => d.pick_file().map(|p| vec![p]),
        "image" => d.add_filter(crate::desktop::locale::text("dialog.pictures", &[]), &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tif", "tiff", "avif"]).pick_file().map(|p| vec![p]),
        "subtitles" => d.add_filter(crate::desktop::locale::text("dialog.subtitles", &[]), &["srt", "vtt"]).pick_file().map(|p| vec![p]),
        _ => None,
    };
    r.map(|v| v.into_iter().map(|p| p.to_string_lossy().to_string()).collect())
}

/// Move items' files between the library and its locations: `copy_in` / `move_in` bring linked items into the library's store (into `folder`, which they join, when given), `to_folder` moves files into a folder (picked here) inside a location. With `root`, the items are all those of that location, or of its folder `path`. Returns false when the picker was cancelled.
#[tauri::command]
pub(super) async fn transfer(state: tauri::State<'_, AppState>, action: String, ids: Vec<i64>, folder: Option<i64>, root: Option<i64>, path: Option<String>) -> R<bool> {
    use crate::core::store::transfer::{self, Mode};
    let dir = if action == "to_folder" {
        match rfd::FileDialog::new().set_title(crate::desktop::locale::text("dialog.moveFiles", &[])).pick_folder() {
            Some(d) => Some(d),
            None => return Ok(false),
        }
    } else {
        None
    };
    state.with_lib(|lib| {
        let style = lib.settings().name_suffix;
        let lp = lib.paths().clone();
        lib.job(move |db, ctx| {
            let ids = match root {
                Some(r) => db.items_under(r, path.as_deref())?,
                None => ids,
            };
            let out = match (action.as_str(), &dir) {
                ("copy_in", _) => transfer::into_library(db, &lp, &ids, Mode::Copy, folder, style)?,
                ("move_in", _) => transfer::into_library(db, &lp, &ids, Mode::Move, folder, style)?,
                ("to_folder", Some(d)) => transfer::to_folder(db, &ids, d, style)?,
                (other, _) => return Err(format!("unknown transfer {other}")),
            };
            for p in &out.problems {
                logging::warn(&format!("moving files: {p}"));
            }
            if let Some(first) = out.problems.first() {
                ctx.send(LibEvent::Error(Said::from(first.as_str()).and_more(out.problems.len())));
            }
            if !out.moved.is_empty() {
                let n = out.moved.len();
                let items = format!("{n} item{}", if n == 1 { "" } else { "s" });
                let args = serde_json::json!({"count": n, "linked": out.linked});
                ctx.send(LibEvent::Done(match action.as_str() {
                    "copy_in" if out.linked == n => Said::new("results.transfer.linked", args, format!("{items} brought into the library as hard links, taking no extra space")),
                    "copy_in" if out.linked > 0 => Said::new("results.transfer.partlyLinked", args, format!("{items} brought into the library, {} as hard links", out.linked)),
                    "copy_in" => Said::new("results.transfer.copied", args, format!("{items} copied into the library")),
                    "move_in" => Said::new("results.transfer.movedIn", args, format!("{items} moved into the library")),
                    _ => Said::new("results.transfer.moved", args, format!("{items} moved")),
                }));
            } else if root.is_some() && out.problems.is_empty() {
                ctx.send(LibEvent::Done(Said::new("results.transfer.allThere", serde_json::Value::Null, "Every item there is already in the library")));
            }
            Ok(Changed::files(&out.moved))
        });
        // sources of moves across drives wait in the Recycle Bin queue
        lib.kick_file_ops();
        Ok(true)
    })
}

/// How many files an import would copy onto names already taken in their folders; the window asks what to do with them before `add_paths`.
#[tauri::command]
pub(super) async fn import_clashes(state: tauri::State<'_, AppState>, paths: Vec<String>, copy: bool, folder: Option<i64>) -> R<serde_json::Value> {
    let files: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let scan = state.with_lib(|l| Ok(l.ctx_scan_options()))?;
    let (count, names) = state.read(|db| crate::core::import::count_clashes(db, &files, copy, folder, &scan))?;
    Ok(serde_json::json!({ "count": count, "names": names }))
}

/// Files handed over by another window, the API or the command line, linked where they are.
pub(super) fn add_files(state: &AppState, paths: Vec<PathBuf>, how: &'static str) -> R<()> {
    state.with_lib(|lib| edits::add_paths(lib, paths, false, None, OnClash::Keep, None, how))
}

/// How often an import under way refreshes what's on screen.
const IMPORT_REFRESH: Duration = Duration::from_millis(400);

#[tauri::command]
pub(super) fn cancel_import(state: tauri::State<AppState>) -> R<()> {
    state.with_lib(|l| {
        l.cancel_jobs();
        Ok(())
    })
}

#[tauri::command]
pub(super) async fn import_clipboard(state: tauri::State<'_, AppState>, folder: Option<i64>) -> R<String> {
    // files copied in a file manager come first: they can carry a picture of their icon too
    let files = crate::core::import::clipboard::clipboard_files();
    if !files.is_empty() {
        return Ok(format!("files\n{}", files.join("\n")));
    }
    let image = crate::core::import::clipboard::clipboard_image_png();
    state.with_lib(|lib| {
        let style = lib.settings().name_suffix;
        let lp = lib.paths().clone();
        match image {
            Ok(png) => {
                let name = format!("Pasted {}.png", chrono::Local::now().format("%Y-%m-%d %H-%M-%S"));
                lib.job(move |db, _| {
                    let id = crate::core::import::import_bytes(db, &lp, &name, &png, None, folder, style)?;
                    Ok(Changed::added(&[id], "window"))
                });
                lib.kick_thumbs();
                Ok("image".into())
            }
            Err(_) => Ok(crate::core::import::clipboard::clipboard_text().unwrap_or_default()),
        }
    })
}

#[tauri::command]
pub(super) async fn import_dot_library(state: tauri::State<'_, AppState>, copy: Option<bool>) -> R<bool> {
    let Some(d) = rfd::FileDialog::new().set_title(crate::desktop::locale::text("dialog.eagleLibrary", &[])).pick_folder() else { return Ok(false) };
    let copy = copy.unwrap_or(false);
    state.with_lib(|lib| {
        let style = lib.settings().name_suffix;
        let lp = lib.paths().clone();
        let links = lib.hard_links();
        lib.job(move |db, ctx| {
            let s = if copy {
                let started = chrono::Utc::now().timestamp_millis();
                let batch = std::cell::RefCell::new((Vec::new(), std::time::Instant::now()));
                let step = |s: crate::core::import::ImportStep| {
                    *ctx.import.lock() = Some(crate::core::library::ImportState { done: s.done, total: s.total, name: s.name.to_string(), started });
                    let mut b = batch.borrow_mut();
                    b.0.extend_from_slice(s.new);
                    if !b.0.is_empty() && b.1.elapsed() >= IMPORT_REFRESH {
                        ctx.send(LibEvent::Changed(Changed::added(&b.0, "window")));
                        ctx.thumbs_wanted.store(true, std::sync::atomic::Ordering::Relaxed);
                        b.0.clear();
                        b.1 = std::time::Instant::now();
                    }
                };
                let got = crate::core::import::eagle::import_dot_library_copy(db, &lp, &d, None, style, links, &step, &ctx.cancel);
                *ctx.import.lock() = None;
                let (s, _) = got?;
                ctx.thumbs_wanted.store(true, std::sync::atomic::Ordering::Relaxed);
                s
            } else {
                crate::core::import::eagle::import_dot_library(db, &d, None)?
            };
            let _ = ctx.events.send(LibEvent::Done(crate::core::import::eagle::eagle_report(&s, copy)));
            Ok(Changed::structure())
        });
        lib.kick_thumbs();
        Ok(true)
    })
}

/// A location's place in the list, its watching and showing it in the file manager; the rest goes through the API's `location.*`.
#[tauri::command]
pub(super) async fn root_op(state: tauri::State<'_, AppState>, op: String, id: i64, flag: Option<bool>, before: Option<i64>) -> R<()> {
    state.with_lib(|lib| {
        match op.as_str() {
            "place" => lib.write(move |db, _| {
                db.place_root(id, before)?;
                Ok(Changed::parts(Parts::LOCATIONS))
            }),
            "watch" => {
                let on = flag.unwrap_or(true);
                lib.write(move |db, _| {
                    db.set_root_flags(id, true, on)?;
                    Ok(Changed::parts(Parts::LOCATIONS))
                });
            }
            "reveal" => {
                let r = lib.db.root(id).ok_or("no such location")?;
                crate::core::system::filesystem::open_default(Path::new(&r.path))?;
            }
            other => return Err(format!("unknown root op {other}")),
        }
        Ok(())
    })
}

#[tauri::command]
pub(super) async fn relink(state: tauri::State<'_, AppState>, id: i64) -> R<bool> {
    let Some(f) = rfd::FileDialog::new().set_title(crate::desktop::locale::text("dialog.relink", &[])).pick_file() else { return Ok(false) };
    state.with_lib(|lib| {
        lib.job(move |db, _| {
            let f = crate::core::system::filesystem::canonical(&f);
            let roots = db.roots()?;
            let root = match roots.iter().find(|r| f.starts_with(&r.path)) {
                Some(r) => r.id,
                None => db.add_root(&f.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(), RootKind::Linked, true)?,
            };
            let root_path = db.root(root).map(|r| r.path).unwrap_or_default();
            let rel = crate::core::system::filesystem::rel_to_db(f.strip_prefix(&root_path).unwrap_or(&f));
            db.move_asset(id, root, &rel, &crate::core::system::filesystem::name_of(&f))?;
            let st = crate::core::system::filesystem::stat(&f).map_err(|e| e.to_string())?;
            let ext = crate::core::system::filesystem::ext_of(&f);
            let animated = matches!(ext.as_str(), "gif" | "webp" | "png" | "apng").then(|| crate::core::metadata::extract::is_animated(&f, &ext));
            db.update_scanned(
                id,
                &crate::core::db::assets::ScannedFile {
                    rel_path: rel,
                    name: crate::core::system::filesystem::name_of(&f),
                    ext,
                    size: st.size as i64,
                    mtime: st.mtime,
                    ctime: st.ctime,
                    file_key: st.file_key.clone(),
                    quick_hash: crate::core::system::filesystem::quick_hash(&f, st.size).ok(),
                    width: None,
                    height: None,
                    duration: None,
                    animated,
                },
            )?;
            Ok(Changed::files(&[id]))
        });
        lib.kick_thumbs();
        Ok(true)
    })
}

/// Ctrl-drag onto a folder: see `import::copy_to_folder`.
#[tauri::command]
pub(super) fn copy_to_folder(state: tauri::State<AppState>, ids: Vec<i64>, folder: i64) -> R<()> {
    state.with_lib(|lib| {
        let style = lib.settings().name_suffix;
        let lp = lib.paths().clone();
        lib.job(move |db, _| Ok(Changed::added(&crate::core::import::copy_to_folder(db, &lp, &ids, folder, style)?, "duplicate")));
        lib.kick_thumbs();
        Ok(())
    })
}

/// Put the files on the clipboard, ready to paste into a file manager.
#[tauri::command]
pub(super) async fn copy_files(state: tauri::State<'_, AppState>, ids: Vec<i64>) -> R<usize> {
    let files: Vec<PathBuf> = state.read(|db| Ok(ids.iter().filter_map(|i| db.abs_path(*i)).collect()))?;
    leave_transfer(&state, &ids);
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set().file_list(&files).map_err(|e| e.to_string())?;
    Ok(files.len())
}

/// Small files kept with the library: `actions`, and `layout` (its own layouts, view and pinned filters, used while Settings › Layout keeps one for each library). (Saved filters have their own command.)
#[tauri::command]
pub(super) async fn library_json(state: tauri::State<'_, AppState>, name: String, set: Option<serde_json::Value>) -> R<serde_json::Value> {
    let empty = match name.as_str() {
        "actions" => serde_json::json!([]),
        "layout" => serde_json::Value::Null,
        _ => return Err(format!("unknown library file {name}")),
    };
    let path = state.with_lib(|l| Ok(l.paths().root.join(format!("{name}.json"))))?;
    if let Some(v) = set {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        return Ok(v);
    }
    Ok(std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(empty))
}
