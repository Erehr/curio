//! Zip packages of Sketch, XMind, Keynote and IDML, and SketchUp models saved as zips, read through `archive` as comics are.
//! Their preview is a known entry; the first picture in reading order is usually one the document contains instead.

use std::path::Path;

use image::DynamicImage;

use crate::core::archive;

pub fn handles(ext: &str) -> bool {
    matches!(ext, "sketch" | "xmind" | "key" | "idml")
}

/// How well an entry, by its lowercase name, serves as the package's preview: lower is better, `None` not at all.
fn rank(ext: &str, name: &str) -> Option<u32> {
    let file = name.rsplit('/').next().unwrap_or(name);
    let picture = [".png", ".jpg", ".jpeg"].iter().any(|e| file.ends_with(e));
    // Keynote packages may sit in a `<name>.key/` folder of their own
    let inner = match name.split_once('/') {
        Some((top, rest)) if ext == "key" && top.ends_with(".key") => rest,
        _ => name,
    };
    match ext {
        "sketch" => (name == "previews/preview.png").then_some(0),
        "xmind" => (name == "thumbnails/thumbnail.png").then_some(0),
        "key" => match inner {
            "preview.jpg" | "preview.png" => Some(0),
            "quicklook/thumbnail.jpg" | "quicklook/thumbnail.png" => Some(1),
            // slide pictures, in slide order by name
            _ if picture && inner.strip_prefix("data/").is_some_and(|f| f.starts_with("st-") && !f.contains('/')) => Some(2),
            _ => None,
        },
        "idml" | "skp" => (picture && (file.starts_with("thumbnail") || file.starts_with("preview"))).then_some(0),
        _ => None,
    }
}

pub fn preview(path: &Path, ext: &str) -> Option<DynamicImage> {
    let entries = archive::list(path).ok()?;
    let mut ranked: Vec<(u32, String, &str)> = entries.iter().filter(|e| e.size <= super::MAX_PICTURE).filter_map(|e| {
        let lower = e.name.to_lowercase();
        rank(ext, &lower).map(|r| (r, lower, e.name.as_str()))
    }).collect();
    ranked.sort();
    let tried = ranked.iter().take(8).find_map(|(_, _, name)| archive::read(path, name).ok().and_then(|b| super::picture(&b)));
    if tried.is_some() || ext != "idml" {
        return tried;
    }
    // IDML keeps its thumbnail in the XMP metadata, as InDesign documents do
    let meta = entries.iter().find(|e| e.name.eq_ignore_ascii_case("META-INF/metadata.xml") && e.size <= super::MAX_PICTURE * 2)?;
    let xml = archive::read(path, &meta.name).ok()?;
    let open = super::position(&xml, b"<xmpGImg:image>")? + b"<xmpGImg:image>".len();
    let len = super::position(&xml[open..], b"</xmpGImg:image>")?;
    super::picture(&super::xml_base64(&xml[open..open + len])?)
}
