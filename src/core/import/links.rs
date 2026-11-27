//! Addresses brought in: which may be fetched, whether one is a page or a file, and the name a download takes.

use crate::core::import::sanitize_name;
use std::path::Path;
use crate::core::system::filesystem;

/// Only web addresses, and nothing that could add lines to a `.url` file.
pub fn check_web_url(url: &str) -> Result<(), String> {
    let web = url::Url::parse(url).is_ok_and(|u| matches!(u.scheme(), "http" | "https"));
    if !web || url.contains(['\r', '\n']) {
        return Err(format!("{url} isn't a web address"));
    }
    Ok(())
}

/// True when a URL should become a bookmark (web page / video page) rather than a download.
pub fn is_page_url(url: &str) -> bool {
    if crate::core::thumbs::links::youtube_id(url).is_some() {
        return true;
    }
    let path = url.split(['?', '#']).next().unwrap_or(url).to_lowercase();
    let ext = path.rsplit('.').next().unwrap_or("");
    let looks_like_file = path.rsplit('/').next().map(|seg| seg.contains('.')).unwrap_or(false)
        && crate::core::model::kind_for_ext(ext) != crate::core::model::AssetKind::Other
        && !matches!(ext, "html" | "htm" | "xhtml" | "php" | "asp" | "aspx");
    !looks_like_file
}

/// Types kept as files when an address that looks like a page answers with them.
pub(crate) fn is_file_type(ctype: &str) -> bool {
    let t = ctype.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    t.starts_with("image/") || t.starts_with("video/") || t.starts_with("audio/") || t.starts_with("font/") || t.starts_with("model/")
        || matches!(t.as_str(), "application/pdf" | "application/zip" | "application/x-zip-compressed" | "application/x-7z-compressed" | "application/x-rar-compressed" | "application/vnd.rar" | "application/x-tar" | "application/gzip" | "application/x-gzip" | "application/x-bzip2" | "application/x-xz" | "application/zstd" | "application/epub+zip" | "application/vnd.comicbook+zip")
}

/// The file name a `Content-Disposition` header gives, `filename*` (RFC 5987) before `filename`.
pub(crate) fn disposition_name(h: &str) -> Option<String> {
    let mut plain = None;
    for part in h.split(';') {
        let Some((k, v)) = part.split_once('=') else { continue };
        let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
        if k == "filename*" {
            let encoded = v.split_once("''").map_or(v, |(_, e)| e).trim_matches('"');
            let bytes: Vec<u8> = percent_decoded(encoded);
            if let Ok(s) = String::from_utf8(bytes) {
                return Some(s).filter(|s| !s.trim().is_empty());
            }
        } else if k == "filename" {
            plain = Some(v.trim_matches('"').to_string()).filter(|s| !s.trim().is_empty());
        }
    }
    plain
}

fn percent_decoded(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Some(v) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// A `Referer` worth sending: a web address, on one line.
pub(crate) fn referer_ok(r: &str) -> bool {
    r.len() <= 2048 && !r.contains(['\r', '\n']) && url::Url::parse(r).is_ok_and(|u| matches!(u.scheme(), "http" | "https"))
}

/// A downloaded file's name: the address's last part, else the one `Content-Disposition` gives, with an extension from the type when neither has one.
pub(crate) fn file_name(url: &str, from_header: Option<String>, ctype: &str) -> String {
    let mut name = url.split(['?', '#']).next().unwrap_or(url).rsplit('/').next().unwrap_or("download").to_string();
    if name.is_empty() {
        name = "download".into();
    }
    if filesystem::ext_of(Path::new(&name)).is_empty() {
        if let Some(n) = from_header {
            name = n;
        }
    }
    if filesystem::ext_of(Path::new(&name)).is_empty() {
        let ext = match ctype.split(';').next().unwrap_or("") {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/webp" => "webp",
            "image/gif" => "gif",
            "image/svg+xml" => "svg",
            "image/avif" => "avif",
            "video/mp4" => "mp4",
            "application/pdf" => "pdf",
            _ => "bin",
        };
        name = format!("{name}.{ext}");
    }
    sanitize_name(&name)
}
