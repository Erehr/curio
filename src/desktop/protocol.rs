//! The `thumb://` protocol, what it serves and the media server that streams video and sound to the web view.

use crate::desktop::media::copy_file;
use crate::desktop::AppState;
use std::path::{Path, PathBuf};
use parking_lot::Mutex;
use tauri::Manager;
use crate::core::model::*;
use crate::desktop::config::load_config;

/// The only page allowed to read these responses from script.
#[cfg(windows)]
pub(super) const APP_ORIGIN: &str = "http://tauri.localhost";

#[cfg(not(windows))]
pub(super) const APP_ORIGIN: &str = "tauri://localhost";

/// Windows showing a web page ("Open Link in App") are labelled like this and never see library files.
pub(crate) const WEB_WINDOW_PREFIX: &str = "web";

pub(super) fn http_response(status: u16, mime: &str, body: Vec<u8>) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(status)
        .header("Content-Type", mime)
        .header("Access-Control-Allow-Origin", APP_ORIGIN)
        // a refusal is never kept: the same address answers once its thumbnail or preview is made
        .header("Cache-Control", if status == 200 { "max-age=31536000, immutable" } else { "no-store" })
        .body(body)
        .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
}

/// The span a `Range` header asks for: `Ok(None)` without one, `Err` when it lies outside the file.
fn byte_range(range: Option<&str>, len: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some(r) = range.and_then(|r| r.trim().strip_prefix("bytes=")) else { return Ok(None) };
    let Some((a, b)) = r.split_once('-') else { return Ok(None) };
    let (start, end) = if a.is_empty() {
        let n: u64 = b.trim().parse().map_err(|_| ())?;
        (len.saturating_sub(n), len.saturating_sub(1))
    } else {
        let start: u64 = a.trim().parse().map_err(|_| ())?;
        let end = if b.trim().is_empty() { len.saturating_sub(1) } else { b.trim().parse::<u64>().map_err(|_| ())?.min(len.saturating_sub(1)) };
        (start, end)
    };
    if len == 0 || start > end || start >= len {
        return Err(());
    }
    Ok(Some((start, end)))
}

/// What one ranged reply carries at most; the player asks again for the rest.
const RANGE_CAP: u64 = 8 << 20;

/// Serve a file honouring HTTP Range requests (video seeking; big media files are never read whole).
fn serve_file(path: &Path, mime: &str, range: Option<&str>) -> tauri::http::Response<Vec<u8>> {
    serve_file_for(path, mime, range, APP_ORIGIN)
}

/// A file, or the part of it a `Range` header asks for, readable from script at `origin`.
pub(super) fn serve_file_for(path: &Path, mime: &str, range: Option<&str>, origin: &str) -> tauri::http::Response<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(path) else { return http_response(404, "text/plain", vec![]) };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let refused = || {
        tauri::http::Response::builder()
            .status(416)
            .header("Content-Range", format!("bytes */{len}"))
            .header("Access-Control-Allow-Origin", origin)
            .body(Vec::new())
            .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
    };
    let Ok(parsed) = byte_range(range, len) else { return refused() };
    let media = mime.starts_with("video/") || mime.starts_with("audio/");
    let fallback = (media && len > 32 << 20).then_some((0, len - 1));
    match parsed.or(fallback) {
        Some((start, end)) => {
            let end = end.min(start + RANGE_CAP - 1);
            let mut buf = vec![0u8; (end - start + 1) as usize];
            if f.seek(SeekFrom::Start(start)).is_err() || f.read_exact(&mut buf).is_err() {
                return refused();
            }
            tauri::http::Response::builder()
                .status(206)
                .header("Content-Type", mime)
                .header("Accept-Ranges", "bytes")
                .header("Content-Range", format!("bytes {start}-{end}/{len}"))
                .header("Content-Length", buf.len().to_string())
                .header("Access-Control-Allow-Origin", origin)
                .body(buf)
                .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
        }
        None => {
            let mut buf = Vec::with_capacity(len as usize);
            let _ = f.read_to_end(&mut buf);
            tauri::http::Response::builder()
                .status(200)
                .header("Content-Type", mime)
                .header("Accept-Ranges", "bytes")
                .header("Access-Control-Allow-Origin", origin)
                .body(buf)
                .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
        }
    }
}

/// The file to play for an item without FFmpeg: the copy it made earlier when there is one, else the original.
fn playable_plain(st: &AppState, id: i64) -> Result<(PathBuf, String), (u16, String)> {
    let Some((abs, ext, _, uid, _, _)) = asset_info(st, id) else { return Err((404, String::new())) };
    if let Some(copy) = copy_file(st, &uid).filter(|p| p.exists()) {
        return Ok((copy, "video/mp4".to_string()));
    }
    Ok((PathBuf::from(abs), mime_for_ext(&ext).to_string()))
}

/// The file to play for an item: the original when the web view can play it, else a copy FFmpeg converts now and keeps.
fn playable_file(st: &AppState, id: i64) -> Result<(PathBuf, String), (u16, String)> {
    if crate::core::media::engine() != crate::core::media::Engine::Ffmpeg {
        return playable_plain(st, id);
    }
    let Some((abs, ext, _, uid, _, _)) = asset_info(st, id) else { return Err((404, String::new())) };
    let src = Path::new(&abs);
    // asked for on every seek, so the answer is remembered
    let native = {
        static KNOWN: std::sync::OnceLock<Mutex<std::collections::HashMap<String, bool>>> = std::sync::OnceLock::new();
        let known = KNOWN.get_or_init(Default::default);
        let hit = known.lock().get(&abs).copied();
        hit.unwrap_or_else(|| {
            let n = crate::core::media::ffmpeg::natively_playable(&ext, &crate::core::media::ffmpeg::probe(src).unwrap_or_default());
            known.lock().insert(abs.clone(), n);
            n
        })
    };
    if native {
        return Ok((src.to_path_buf(), mime_for_ext(&ext).to_string()));
    }
    let Some(prev) = st.previews_dir.read().clone() else { return Err((404, String::new())) };
    crate::core::media::ffmpeg::playable_copy(src, &prev.join("video"), &crate::core::thumbs::cache_key(&uid)).map(|p| (p, "video/mp4".to_string())).map_err(|e| (415, e))
}

/// Names an item's version among the previews held in memory; the library's file tells one library's items from another's.
pub(super) fn preview_key(st: &AppState, uid: &str, ver: i64) -> String {
    format!("{}|{uid}_{ver}", st.db_path.read().as_deref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default())
}

pub(super) fn asset_info(st: &AppState, id: i64) -> Option<(String, String, AssetKind, String, i64, String)> {
    st.read(|db| Ok(db.asset(id)?.map(|a| (a.abs_path, a.ext, a.kind, a.uid, a.thumb_ver, a.source_url)))).ok().flatten()
}

/// Tauri's default WebView2 switches, sound that may start without a click (hovering a song), and
/// optionally no GPU. Every window of a process must use the same ones.
pub(super) fn browser_args() -> String {
    let mut args = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required".to_string();
    if !load_config().gpu {
        args.push_str(" --disable-gpu");
    }
    args
}

/// Formats the web view can't draw are decoded once at up to this size.
const FULL_PREVIEW: u32 = 8192;

pub(super) fn serve(app: &tauri::AppHandle, path: &str, range: Option<&str>) -> tauri::http::Response<Vec<u8>> {
    let st = app.state::<AppState>();
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    match parts.as_slice() {
        // /t/<uid>/<ver> — thumbnail
        ["t", uid, ver] => {
            let Some(dir) = st.thumbs_dir.read().clone() else { return http_response(404, "text/plain", vec![]) };
            let ver_num = ver.parse().unwrap_or(0);
            let p = crate::core::thumbs::thumb_path(&dir, uid, ver_num);
            match std::fs::read(&p) {
                Ok(b) => {
                    let mime = if b.starts_with(b"\x89PNG") { "image/png" } else { "image/jpeg" };
                    http_response(200, mime, b)
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        let _ = st.with_lib(|lib| {
                            lib.remake_missing_thumb(uid.to_string(), ver_num);
                            Ok(())
                        });
                    }
                    http_response(404, "text/plain", vec![])
                }
            }
        }
        // /p/<id>/<size> — scaled preview (progressive loading of large images)
        ["p", id, size] => {
            let Some((abs, ext, kind, uid, ver, _)) = asset_info(&st, id.parse().unwrap_or(0)) else { return http_response(404, "text/plain", vec![]) };
            let size: u32 = size.parse().unwrap_or(2048).clamp(256, 8192);
            let handlers = st.handlers.read().clone();
            let tmp = st.tmp_dir.read().clone().unwrap_or_else(std::env::temp_dir);
            match crate::core::thumbs::previews::preview(&preview_key(&st, &uid, ver), Path::new(&abs), &ext, kind, size, &handlers, &tmp) {
                Ok(b) => {
                    let b = b.to_vec();
                    let mime = if b.starts_with(b"\x89PNG") { "image/png" } else { "image/jpeg" };
                    http_response(200, mime, b)
                }
                Err(e) => http_response(415, "text/plain", e.into_bytes()),
            }
        }
        // /w/<id> — loudness peaks of a sound, for waveforms
        ["w", id] => {
            let Some((abs, _, _, uid, ver, _)) = asset_info(&st, id.parse().unwrap_or(0)) else { return http_response(404, "text/plain", vec![]) };
            let Some(prev) = st.previews_dir.read().clone() else { return http_response(404, "text/plain", vec![]) };
            match crate::core::media::ffmpeg::waveform(Path::new(&abs), &prev.join("wave"), &format!("{}_{ver}", crate::core::thumbs::cache_key(&uid))) {
                Ok(b) => http_response(200, "application/octet-stream", b),
                Err(e) => http_response(415, "text/plain", e.into_bytes()),
            }
        }
        // /f/<id> — the original, or a full-size decoded copy for formats the web view can't show
        ["f", id] => {
            let Some((abs, ext, kind, uid, ver, _)) = asset_info(&st, id.parse().unwrap_or(0)) else { return http_response(404, "text/plain", vec![]) };
            let native = matches!(ext.as_str(), "jpg" | "jpeg" | "jfif" | "jpe" | "png" | "apng" | "gif" | "webp" | "avif" | "svg" | "bmp" | "ico")
                || matches!(ext.as_str(), "mp4" | "webm" | "m4v" | "mp3" | "wav" | "ogg" | "oga" | "m4a" | "flac" | "aac" | "opus" | "pdf")
                // the list's font samples load the file itself
                || matches!(ext.as_str(), "ttf" | "otf" | "ttc" | "otc" | "woff" | "woff2");
            if native {
                return serve_file(Path::new(&abs), mime_for_ext(&ext), range);
            }
            let handlers = st.handlers.read().clone();
            let tmp = st.tmp_dir.read().clone().unwrap_or_else(std::env::temp_dir);
            match crate::core::thumbs::previews::preview(&preview_key(&st, &uid, ver), Path::new(&abs), &ext, kind, FULL_PREVIEW, &handlers, &tmp) {
                Ok(b) => {
                    let b = b.to_vec();
                    let mime = if b.starts_with(b"\x89PNG") { "image/png" } else { "image/jpeg" };
                    http_response(200, mime, b)
                }
                Err(e) => http_response(415, "text/plain", e.into_bytes()),
            }
        }
        // /v/<id> — web-playable video/audio (the original when compatible, else a cached converted copy)
        ["v", id] => match playable_file(&st, id.parse().unwrap_or(0)) {
            Ok((path, mime)) => serve_file(&path, &mime, range),
            Err((status, text)) => http_response(status, "text/plain", text.into_bytes()),
        },
        // /x/<id>[/<bytes>|/thumb] — text content (txt, md, json, code), capped at 4 MB; the grid asks
        // for what a thumbnail reads
        ["x", id] | ["x", id, _] => {
            let Some((abs, _, _, _, _, _)) = asset_info(&st, id.parse().unwrap_or(0)) else { return http_response(404, "text/plain", vec![]) };
            let cap: u64 = match parts.get(2).copied() {
                Some("thumb") => crate::core::consts::TEXT_THUMB_BYTES,
                other => other.and_then(|n| n.parse().ok()).unwrap_or(4 << 20).min(4 << 20),
            };
            use std::io::Read;
            let mut buf = Vec::new();
            if let Ok(f) = std::fs::File::open(&abs) {
                let _ = f.take(cap).read_to_end(&mut buf);
            }
            // never cut a UTF-8 character in half
            while !buf.is_empty() && std::str::from_utf8(&buf).is_err() && buf.len() as u64 >= cap.saturating_sub(4) {
                buf.pop();
            }
            http_response(200, "text/plain; charset=utf-8", buf)
        }
        _ => http_response(404, "text/plain", vec![]),
    }
}

/// Where the player fetches media from when it can't use the custom scheme.
pub(super) static MEDIA_BASE: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// WebKitGTK's player fails on a custom scheme ("Loading failed, error: FormatError" from
/// MediaPlayerPrivateGStreamer), so on Linux it gets a local address. Loopback only, and each request
/// must carry a token made fresh for this run.
#[cfg(target_os = "linux")]
pub(super) fn start_media_server(app: &tauri::AppHandle) {
    let Ok(server) = tiny_http::Server::http("127.0.0.1:0") else { return };
    let port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(0);
    let token = crate::desktop::process::new_token();
    if MEDIA_BASE.set(format!("http://127.0.0.1:{port}/{token}")).is_err() {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let app = app.clone();
            let token = token.clone();
            // the player holds a connection open while it reads ahead and seeks
            std::thread::spawn(move || answer_media(&app, &token, req));
        }
    });
}

#[cfg(target_os = "linux")]
fn answer_media(app: &tauri::AppHandle, token: &str, req: tiny_http::Request) {
    use std::io::{Read, Seek, SeekFrom};
    let empty = |code: u16| tiny_http::Response::empty(code);
    let url = req.url().split('?').next().unwrap_or("").trim_matches('/').to_string();
    let parts: Vec<&str> = url.split('/').collect();
    let id = match parts[..] {
        [t, "v", id] if t == token => id.parse::<i64>().ok(),
        _ => None,
    };
    let is_read = matches!(req.method(), tiny_http::Method::Get | tiny_http::Method::Head);
    let Some(id) = id.filter(|_| is_read) else {
        let _ = req.respond(empty(404));
        return;
    };
    let st = app.state::<AppState>();
    let found = playable_file(&st, id);
    let Some((path, mime, mut file, len)) = found.ok().and_then(|(path, mime)| {
        let file = std::fs::File::open(&path).ok()?;
        let len = file.metadata().ok()?.len();
        Some((path, mime, file, len))
    }) else {
        let _ = req.respond(empty(404));
        return;
    };
    let _ = path;
    let header = |k: &str, v: &str| tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()).ok();
    let range = req.headers().iter().find(|h| h.field.equiv("Range")).map(|h| h.value.as_str().to_string());
    let Ok(wanted) = byte_range(range.as_deref(), len) else {
        // says how long the file is, so the player can ask again for a part that exists
        let mut refused = empty(416);
        if let Some(h) = header("Content-Range", &format!("bytes */{len}")) {
            refused.add_header(h);
        }
        let _ = req.respond(refused);
        return;
    };
    let (status, start, count) = match wanted {
        Some((start, end)) => (206, start, end - start + 1),
        None => (200, 0, len),
    };
    if file.seek(SeekFrom::Start(start)).is_err() {
        let _ = req.respond(empty(500));
        return;
    }
    let mut headers: Vec<tiny_http::Header> = [header("Content-Type", &mime), header("Accept-Ranges", "bytes"), header("Access-Control-Allow-Origin", "*")].into_iter().flatten().collect();
    if status == 206 {
        headers.extend(header("Content-Range", &format!("bytes {start}-{}/{len}", start + count - 1)));
    }
    let _ = req.respond(tiny_http::Response::new(tiny_http::StatusCode(status), headers, file.take(count), Some(count as usize), None));
}
