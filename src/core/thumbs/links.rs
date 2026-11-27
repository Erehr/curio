//! Link items' thumbnails: the page's own picture, fetched only when link previews are on.

use crate::core::thumbs::log;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;
use image::DynamicImage;
use regex::Regex;

/// YouTube video id from any of its URL shapes.
pub fn youtube_id(url: &str) -> Option<String> {
    YOUTUBE_RE.captures(url).map(|c| c[1].to_string())
}

static YOUTUBE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:youtube\.com/(?:watch\?(?:.*&)?v=|shorts/|embed/|live/|v/)|youtu\.be/|youtube-nocookie\.com/embed/)([A-Za-z0-9_-]{11})").unwrap());

static URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"https?://[^\s"<>]+"#).unwrap());

static TITLE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap());

static META_IMAGE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?is)<meta[^>]+(?:property|name)=["'](?:og:image|twitter:image)(?::src)?["'][^>]*content=["']([^"']+)["']|<meta[^>]+content=["']([^"']+)["'][^>]+(?:property|name)=["'](?:og:image|twitter:image)["']"#).unwrap());

static ICON_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?is)<link[^>]+rel=["'][^"']*(?:apple-touch-icon|icon)[^"']*["'][^>]*href=["']([^"']+)["']"#).unwrap());

/// URL stored in a Windows InternetShortcut (.url) or a .webloc / plain text file.
pub fn url_from_file(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    for line in text.lines() {
        if let Some(u) = line.trim().strip_prefix("URL=") {
            return Some(u.trim().to_string());
        }
    }
    URL_RE.find(&text).map(|m| m.as_str().to_string())
}

fn http_get(url: &str, limit: u64) -> Result<(Vec<u8>, String), String> {
    use std::io::Read;
    let resp = crate::core::system::net::agent(Duration::from_secs(20)).get(url).call().map_err(|e| e.to_string())?;
    let ctype = resp.header("content-type").unwrap_or("").to_string();
    let mut buf = Vec::new();
    resp.into_reader().take(limit).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok((buf, ctype))
}

static LINK_PREVIEWS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Link previews fetch from the web, so they wait for the user to allow it in Settings: a scanned link file must not make requests by itself.
pub fn set_link_previews(on: bool) {
    LINK_PREVIEWS.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn link_previews() -> bool {
    LINK_PREVIEWS.load(std::sync::atomic::Ordering::Relaxed)
}

pub const LINK_PREVIEWS_OFF: &str = "link previews are off";

fn absolutize(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    if let Some(rest) = href.strip_prefix("//") {
        return format!("https://{rest}");
    }
    let origin = base.splitn(4, '/').take(3).collect::<Vec<_>>().join("/");
    if href.starts_with('/') {
        format!("{origin}{href}")
    } else {
        format!("{}/{href}", base.rsplit_once('/').map(|x| x.0).unwrap_or(&origin))
    }
}

/// Thumbnail for a link. YouTube: the largest video poster it offers.
/// Websites: a page capture when a browser is available, else the page's og:image / icon.
pub(super) fn link_image(url: &str, tmp_dir: &Path) -> Result<(DynamicImage, Option<String>), String> {
    if !link_previews() {
        return Err(LINK_PREVIEWS_OFF.into());
    }
    if let Some(id) = youtube_id(url) {
        for name in ["maxresdefault", "hqdefault", "mqdefault"] {
            if let Ok((bytes, _)) = http_get(&format!("https://i.ytimg.com/vi/{id}/{name}.jpg"), 8 << 20) {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    // maxres is missing for some videos and comes back as a 120×90 placeholder
                    if img.width() > 200 {
                        return Ok((img, None));
                    }
                }
            }
        }
        return Err("no YouTube thumbnail".into());
    }
    let captured = {
        let out = tmp_dir.join(format!("page_{}.png", fastrand::u64(..)));
        // the browser fetches by itself, so the address is checked here; the page's own requests are the browser's business, as in any browser
        let img = crate::core::system::net::check_public_url(url).and_then(|_| crate::core::media::capture_page(url, &out).and_then(|_| image::open(&out).map_err(|e| e.to_string())).map_err(|e| format!("page capture: {e}")));
        let _ = std::fs::remove_file(&out);
        img.map_err(|e| log::warn(&format!("link thumbnail {url}: {e}"))).ok()
    };
    let page = http_get(url, 2 << 20);
    let (html, ctype) = match (page, captured) {
        (Ok(p), c) => {
            if let Some(img) = c {
                let text = String::from_utf8_lossy(&p.0);
                let title = TITLE_RE.captures(&text).map(|c| c[1].trim().to_string());
                return Ok((img, title));
            }
            p
        }
        (Err(_), Some(img)) => return Ok((img, None)),
        (Err(err), None) => return Err(err),
    };
    if ctype.starts_with("image/") {
        return image::load_from_memory(&html).map(|i| (i, None)).map_err(|e| e.to_string());
    }
    let html = String::from_utf8_lossy(&html);
    let title = TITLE_RE.captures(&html).map(|c| c[1].trim().to_string());
    let (meta, icon) = (&*META_IMAGE_RE, &*ICON_RE);
    let mut candidates: Vec<String> = Vec::new();
    if let Some(c) = meta.captures(&html) {
        candidates.push(absolutize(url, c.get(1).or(c.get(2)).map(|m| m.as_str()).unwrap_or("")));
    }
    if let Some(c) = icon.captures(&html) {
        candidates.push(absolutize(url, &c[1]));
    }
    candidates.push(absolutize(url, "/favicon.ico"));
    for c in candidates {
        if let Ok((bytes, _)) = http_get(&c.replace("&amp;", "&"), 16 << 20) {
            if let Ok(img) = image::load_from_memory(&bytes) {
                return Ok((img, title));
            }
        }
    }
    Err("the page has no preview image".into())
}
