//! A plugin's pages as served: their ids, files, types, policy and theme variables.

use std::path::{Path, PathBuf};

/// A host a page may reach: a name such as `tile.openstreetmap.org`, or `*.` before one for its subdomains.
pub fn network_host_ok(h: &str) -> bool {
    let name = h.strip_prefix("*.").unwrap_or(h);
    !name.is_empty() && name.len() <= 253 && name.contains('.') && name.split('.').all(|l| !l.is_empty() && l.len() <= 63 && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && !l.starts_with('-'))
}

/// An id that can stand as one segment of a page's address.
pub fn page_id_ok(id: &str) -> bool {
    !id.is_empty() && id.len() <= 100 && !id.starts_with('.') && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// The host label of a plugin's own origin: its id made fit for a host name, with a hash of the id as given, so ids differing only in case or punctuation stay apart.
/// `web/src/plugins/pageLabel.js` works it out the same way.
pub fn page_label(id: &str) -> String {
    let mut h: u32 = 0x811c_9dc5;
    for b in id.bytes() {
        h = (h ^ b as u32).wrapping_mul(0x0100_0193);
    }
    let readable: String = id.to_ascii_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).take(40).collect();
    let readable = readable.trim_matches('-');
    if readable.is_empty() {
        format!("p{h:08x}")
    } else {
        format!("{readable}-{h:08x}")
    }
}

/// The file a page address names inside the plugin's folder; nothing outside it, however the address is spelled.
pub fn page_file(dir: &Path, rel: &str) -> Option<PathBuf> {
    let rel = percent_decode(rel)?;
    let rel = if rel.is_empty() || rel.ends_with('/') { format!("{rel}index.html") } else { rel };
    let mut path = dir.to_path_buf();
    for seg in rel.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." || seg.contains(['\\', ':', '\0']) {
            return None;
        }
        path.push(seg);
    }
    // a link inside the folder may still point out of it
    let real = path.canonicalize().ok()?;
    let root = dir.canonicalize().ok()?;
    (real.starts_with(&root) && real.is_file()).then_some(real)
}

/// The type a page's file is served as.
pub fn page_mime(path: &Path) -> &'static str {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "txt" | "md" => "text/plain; charset=utf-8",
        "wasm" => "application/wasm",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        other => crate::core::model::mime_for_ext(other),
    }
}

pub fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The policy a plugin's page runs under: its own files and Curio's UI kit only, and the internet only with `network`.
/// Sources are named in full, as `'self'` would take in the kit's address and the item files beside the pages.
/// Frames, styles and fonts may come from `blob:`: a book reader shows each section in a frame of its own with the book's stylesheets and fonts, and those frames inherit this policy, so a book's scripts still don't run.
/// `network` is `None` without the permission, else the hosts listed (none: every `https:` site).
/// WebAssembly may be compiled (`'wasm-unsafe-eval'`); JavaScript may not be made from text.
/// `fetch` may read `data:` and `blob:` addresses, which hold only what the page already has: a 3D model's embedded textures and buffers are read so.
pub fn page_csp(own: &str, kit: &str, network: Option<&[String]>, app_origin: &str) -> String {
    let base = format!("{own} {kit}");
    let (web, net) = match network {
        None => (String::new(), String::new()),
        Some([]) => (" https:".to_string(), " https: wss:".to_string()),
        Some(hosts) => {
            let ok: Vec<&String> = hosts.iter().filter(|h| network_host_ok(h)).collect();
            (ok.iter().map(|h| format!(" https://{h}")).collect(), ok.iter().map(|h| format!(" https://{h} wss://{h}")).collect())
        }
    };
    format!(
        "default-src 'none'; script-src {base} 'wasm-unsafe-eval'; style-src {base} 'unsafe-inline' blob:; img-src {base} data: blob:{web}; font-src {base} data: blob:; media-src {own} blob: data:{web}; connect-src {base} data: blob:{net}; worker-src {own} {kit} blob:; frame-src blob:; form-action 'none'; base-uri 'none'; frame-ancestors {app_origin}"
    )
}

/// `--plugin-<name>`: a plain name, and a value that can't end the declaration or load anything.
pub fn theme_variable(name: &str, value: &str) -> Option<(String, String)> {
    let name = name.trim_start_matches("--").trim_start_matches("plugin-").to_lowercase().replace(['.', '_'], "-");
    let name_ok = !name.is_empty() && name.len() <= 64 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    let value = value.trim();
    let mut depth = 0i32;
    let value_ok = !value.is_empty()
        && value.len() <= 200
        && !value.to_lowercase().contains("url(")
        && value.chars().all(|c| {
            depth += match c {
                '(' => 1,
                ')' => -1,
                _ => 0,
            };
            depth >= 0 && (c.is_ascii_alphanumeric() || " #%.,+-*/()".contains(c))
        })
        && depth == 0;
    (name_ok && value_ok).then(|| (format!("plugin-{name}"), value.to_string()))
}
