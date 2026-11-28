//! Plugins found and loaded: core plugins, approval, manifests read, families adopted, dependencies resolved.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use crate::core::plugins::pages::page_id_ok;
use crate::core::plugins::RESERVED_ID;
use crate::core::plugins::manifest::Source;
use crate::core::plugins::manifest::FormatDecl;
use crate::core::plugins::PLUGIN_API_VERSION;
use crate::core::plugins::manifest::WidgetDecl;
use crate::core::plugins::manifest::Manifest;
use crate::core::plugins::manifest::Plugin;

/// Shown for a plugin that arrived inside a library folder and hasn't been allowed on this computer.
pub const WAITING_APPROVAL: &str = "Came with this library. Allow it to run on this computer.";

fn plugin_files(dir: &Path, root: Option<&Path>, rel: &str, visited: &mut HashSet<PathBuf>, out: &mut Vec<(String, PathBuf, bool)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Ok(ft) = e.file_type() else { continue };
        let rel = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
        let path = e.path();
        if ft.is_symlink() {
            out.push((rel.clone(), path.clone(), true));
            if let (Some(root), Ok(real)) = (root, path.canonicalize()) {
                if real.starts_with(root) && real.is_dir() && visited.insert(real) {
                    plugin_files(&path, Some(root), &rel, visited, out);
                }
            }
        } else if ft.is_dir() {
            plugin_files(&path, root, &rel, visited, out);
        } else {
            out.push((rel, path, false));
        }
    }
}

/// What an approval covers: this plugin, in this folder, with these permissions and exactly these files, every byte of them (Python's bytecode too, which runs in place of its source), so anything swapped in after approval needs approving again.
pub fn approval_print(p: &Plugin) -> String {
    let mut granted = p.granted.clone();
    granted.sort();
    let mut h = blake3::Hasher::new();
    h.update(format!("plugin|{}|{}|{}|", p.manifest.id, p.dir.display(), granted.join(",")).as_bytes());
    let root = p.dir.canonicalize().ok();
    let mut visited = HashSet::new();
    if let Some(r) = &root {
        visited.insert(r.clone());
    }
    let mut files = Vec::new();
    plugin_files(&p.dir, root.as_deref(), "", &mut visited, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0).then(a.2.cmp(&b.2)));
    for (rel, path, is_link) in files {
        h.update(format!("|{rel}|").as_bytes());
        if is_link {
            h.update(b"symlink|");
            match std::fs::read_link(&path) {
                Ok(t) => {
                    h.update(t.to_string_lossy().as_bytes());
                    let escapes = root.as_deref().is_none_or(|r| path.canonicalize().map_or(true, |real| !real.starts_with(r)));
                    if escapes {
                        h.update(b"|outside");
                        continue;
                    }
                }
                Err(_) => {
                    h.update(b"unreadable");
                    continue;
                }
            }
            if path.is_dir() {
                continue;
            }
        }
        match std::fs::File::open(&path) {
            Ok(f) => {
                if h.update_reader(f).is_err() {
                    h.update(b"|read-error");
                }
            }
            Err(_) => {
                h.update(b"unreadable");
            }
        }
    }
    h.finalize().to_hex().to_string()
}

pub fn hold_until_approved(p: &mut Plugin, library_plugins: &Path, approved: &HashSet<String>) {
    if p.enabled && p.problem.is_none() && p.dir.starts_with(library_plugins) && !approved.contains(&approval_print(p)) {
        p.problem = Some(WAITING_APPROVAL.into());
    }
}

pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let mut m: Manifest = serde_json::from_str(text).map_err(|e| format!("plugin.json: {e}"))?;
    if m.id.trim().is_empty() || m.name.trim().is_empty() {
        return Err("plugin.json needs an id and a name".into());
    }
    // pages, layouts and calls find each of a plugin's widgets, windows, tabs and the like by its id
    let kinds: [(&str, Vec<&str>); 9] = [
        ("widgets", m.widgets.iter().map(|x| x.id.as_str()).collect()),
        ("columns", m.columns.iter().map(|x| x.id.as_str()).collect()),
        ("windows", m.windows.iter().map(|x| x.id.as_str()).collect()),
        ("tabs", m.tabs.iter().map(|x| x.id.as_str()).collect()),
        ("panels", m.panels.iter().map(|x| x.id.as_str()).collect()),
        ("actions", m.actions.iter().map(|x| x.id.as_str()).collect()),
        ("filters", m.filters.iter().map(|x| x.id.as_str()).collect()),
        ("viewer_tools", m.viewer_tools.iter().map(|x| x.id.as_str()).collect()),
        ("viewer_keys", m.viewer_keys.iter().map(|x| x.id.as_str()).collect()),
    ];
    for (kind, ids) in kinds {
        let mut seen = HashSet::new();
        if let Some(twice) = ids.into_iter().find(|i| !seen.insert(*i)) {
            return Err(format!("plugin.json: two {kind} have the id {twice}"));
        }
    }
    let id = m.id.clone();
    for f in &mut m.formats {
        let exts = f.exts.clone();
        f.companions.retain(|c| match crate::core::grouping::companions::Rule::from_decl(&id, &exts, c) {
            Ok(_) => true,
            Err(e) => {
                crate::core::system::logging::warn(&format!("plugin {id}: {e}"));
                false
            }
        });
    }
    Ok(m)
}

/// The built-in extractors presented as plugins so users see where fields come from.
pub fn builtin_plugins() -> Vec<Plugin> {
    let schemas = crate::core::metadata::extract::builtin_schemas();
    let pick = |owner: &str| schemas.iter().filter(|f| f.owner == owner).cloned().collect::<Vec<_>>();
    let widget = |id: &str, title: &str, icon: &str, kind: &str, fields: &[&str]| WidgetDecl {
        id: id.into(),
        title: title.into(),
        icon: Some(icon.into()),
        fields: fields.iter().map(|f| f.to_string()).collect(),
        kinds: vec![kind.into()],
        default_visible: true,
        ..Default::default()
    };
    // one entry for each kind of file: its formats, the fields read from them and the widgets showing those; switched off, Curio leaves its files alone
    let families = crate::core::formats::FAMILIES.iter().map(|f| {
        let id = crate::core::formats::plugin_id(f);
        let (author, widgets) = match f.id {
            "image" => ("", vec![widget("exif.camera", "Camera", "camera", "image", &["camera", "lens", "iso", "aperture", "shutter", "focal_length", "taken_at", "artist", "copyright", "gps_lat", "gps_lon", "gps_alt"])]),
            "audio" => ("lofty (MIT or Apache-2.0)", vec![widget("audio.tags", "Song", "music", "audio", &["title", "artist", "album", "album_artist", "track", "disc", "year", "genre", "composer"]), widget("audio.info", "Audio", "wave-sine", "audio", &["audio_codec", "bit_rate", "channels", "sample_rate"])]),
            "video" => ("", vec![widget("video.info", "Video", "movie", "video", &["video_codec", "frame_rate", "audio_codec", "channels", "sample_rate", "bit_rate"])]),
            _ => ("", vec![]),
        };
        let mut m = Manifest {
            fields: pick(&id),
            id,
            name: f.name.into(),
            version: env!("CURIO_VERSION").into(),
            api: PLUGIN_API_VERSION,
            author: author.into(),
            description: f.description.into(),
            formats: vec![FormatDecl { exts: f.exts.iter().map(|e| e.to_string()).collect(), kind: f.id.into(), ..Default::default() }],
            widgets,
            ..Default::default()
        };
        if f.id == "font" {
            font_entry(&mut m);
        }
        m
    });
    families.map(|m| Plugin { manifest: m, dir: PathBuf::new(), source: Source::Builtin, enabled: true, configured: true, granted: vec![], problem: None }).collect()
}

/// The Fonts entry shows its files in the window's own viewer (`web/src/formats/FontView.vue`), with its tools in Curio's toolbar, and draws their thumbnails there: what a Format plugin declares, with no page of its own.
fn font_entry(m: &mut Manifest) {
    let kinds = vec!["font".to_string()];
    let decl = serde_json::json!({
        "id": m.id, "name": m.name,
        "formats": [{ "exts": m.formats[0].exts, "kind": "font", "viewer": "", "thumbnail_page": { "page": "", "size": 512 },
            "screen": { "id": "fonts", "title": "Fonts", "icon": "typography", "toolbar": ["v.back", "v.count", "v.group", "breadcrumb", "spacer", "sample", "waterfall", "glyphs", "details", "size", "face", "look", "v.strip", "nav"], "inspector": ["curio.fonts.info"] } }],
        "viewer_tools": [
            { "id": "sample", "title": "Sample text", "icon": "align-left", "kind": "button" },
            { "id": "waterfall", "title": "Waterfall", "icon": "text-size", "kind": "button" },
            { "id": "glyphs", "title": "Glyphs", "icon": "layout-grid", "kind": "button" },
            { "id": "details", "title": "Details", "icon": "info-circle", "kind": "button" },
            { "id": "size", "title": "Text size", "icon": "text-resize", "kind": "slider" },
            { "id": "face", "title": "Face", "icon": "stack-2", "kind": "button" },
            { "id": "look", "title": "Appearance", "icon": "palette", "kind": "button" }
        ],
        "viewer_keys": [
            { "id": "sample", "title": "Sample text", "key": "S" },
            { "id": "waterfall", "title": "Waterfall", "key": "W" },
            { "id": "glyphs", "title": "Glyphs", "key": "Y" },
            { "id": "details", "title": "Details", "key": "I" }
        ],
        "columns": [{ "id": "sample", "label": "Font sample", "render": "font", "width": 360, "kinds": kinds }],
        "sorts": [{ "label": "Font weight", "field": "font_weight" }]
    });
    let d: Manifest = serde_json::from_value(decl).expect("the Fonts entry's declarations");
    m.icon = Some("typography".into());
    m.formats = d.formats;
    m.viewer_tools = d.viewer_tools;
    m.viewer_keys = d.viewer_keys;
    m.columns = d.columns;
    m.sorts = d.sorts;
    m.widgets.push(WidgetDecl { id: "curio.fonts.info".into(), title: "Font".into(), icon: Some("typography".into()), fields: crate::core::metadata::extract::builtin_schemas().into_iter().filter(|f| f.owner == m.id).map(|f| f.key).collect(), kinds, default_visible: true, ..Default::default() });
}

/// Discover plugins in the given folders: a folder holding `plugin.json` loads that plugin alone; otherwise each subfolder with `plugin.json` is loaded.
pub fn discover(dirs: &[PathBuf]) -> Vec<Plugin> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut take = |p: &Path| {
        let key = crate::core::system::filesystem::canonical(p);
        if seen.insert(key) {
            if let Some(pl) = load_dir(p) {
                out.push(pl);
            }
        }
    };
    for d in dirs {
        if d.join("plugin.json").exists() {
            take(d);
            continue;
        }
        let Ok(rd) = std::fs::read_dir(d) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() && p.join("plugin.json").exists() {
                take(&p);
            }
        }
    }
    out
}

pub fn load_dir(p: &Path) -> Option<Plugin> {
    let native = p.join("plugin.json");
    if !native.exists() {
        return None;
    }
    let (res, source) = (std::fs::read_to_string(&native).map_err(|e| e.to_string()).and_then(|t| parse_manifest(&t)), Source::Native);
    Some(match res {
        Ok(m) => Plugin { manifest: m, dir: p.to_path_buf(), source, enabled: false, configured: false, granted: vec![], problem: None },
        Err(err) => Plugin {
            manifest: Manifest { id: format!("broken:{}", p.display()), name: crate::core::system::filesystem::name_of(p), ..Default::default() },
            dir: p.to_path_buf(),
            source,
            enabled: false,
            configured: false,
            granted: vec![],
            problem: Some(err),
        },
    })
}

/// Semver-ish check: "1.2", ">=1.0.0", "^1", "*".
pub fn version_satisfies(have: &str, req: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> { s.trim().split('.').map(|x| x.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0)).collect() };
    let req = req.trim();
    if req.is_empty() || req == "*" {
        return true;
    }
    let h = parse(have);
    let cmp = |a: &[u64], b: &[u64]| {
        for i in 0..a.len().max(b.len()) {
            let x = a.get(i).copied().unwrap_or(0);
            let y = b.get(i).copied().unwrap_or(0);
            if x != y {
                return x.cmp(&y);
            }
        }
        std::cmp::Ordering::Equal
    };
    if let Some(r) = req.strip_prefix(">=") {
        return cmp(&h, &parse(r)) != std::cmp::Ordering::Less;
    }
    if let Some(r) = req.strip_prefix('^') {
        let r = parse(r);
        return h.first() == r.first() && cmp(&h, &r) != std::cmp::Ordering::Less;
    }
    if let Some(r) = req.strip_prefix('=') {
        return cmp(&h, &parse(r)) == std::cmp::Ordering::Equal;
    }
    let r = parse(req);
    h.first() == r.first() && cmp(&h, &r) != std::cmp::Ordering::Less
}

/// Whether `have` meets a `min_curio` floor (e.g. `"1.2.0"`, `"1.2"`, or `">=1.2.0"`).
pub fn min_curio_satisfied(have: &str, min: &str) -> bool {
    let min = min.trim();
    if min.is_empty() || min == "*" {
        return true;
    }
    let bare = min.strip_prefix(">=").unwrap_or(min).trim();
    let bare = bare.strip_prefix(['v', 'V']).unwrap_or(bare);
    if !bare.starts_with(|c: char| c.is_ascii_digit()) {
        return false;
    }
    version_satisfies(have, &format!(">={bare}"))
}

/// Mark plugins whose api version, min_curio, or dependencies aren't satisfied.
pub fn resolve_dependencies(plugins: &mut [Plugin]) {
    let app_ver = env!("CURIO_VERSION");
    for p in plugins.iter_mut() {
        if p.manifest.api > PLUGIN_API_VERSION {
            p.problem = Some(format!("needs plugin API v{} (this app has v{PLUGIN_API_VERSION})", p.manifest.api));
        } else if p.manifest.min_curio.as_deref().is_some_and(|m| !min_curio_satisfied(app_ver, m)) {
            let min = p.manifest.min_curio.as_deref().unwrap_or("").trim().trim_start_matches(">=").trim();
            p.problem = Some(format!("needs Curio {min} or newer (this app is {app_ver})"));
        } else if p.source == Source::Native && p.manifest.id.eq_ignore_ascii_case(RESERVED_ID) {
            p.problem = Some(format!("the id '{RESERVED_ID}' belongs to Curio itself"));
        } else if p.manifest.has_pages() && !page_id_ok(&p.manifest.id) {
            p.problem = Some("a plugin with pages needs an id of letters, digits, dots, dashes and underscores".into());
        }
    }
    // iterate until stable: a plugin is blocked if a dependency is missing, inactive or too old
    loop {
        let active: HashMap<String, String> =
            plugins.iter().filter(|p| p.active()).map(|p| (p.manifest.id.clone(), p.manifest.version.clone())).collect();
        let mut changed = false;
        for p in plugins.iter_mut() {
            if !p.active() {
                continue;
            }
            for (dep, req) in &p.manifest.dependencies {
                let ok = active.get(dep).map(|v| version_satisfies(v, req)).unwrap_or(false);
                if !ok {
                    p.problem = Some(format!("needs plugin '{dep}' {req}"));
                    changed = true;
                    break;
                }
            }
        }
        if !changed {
            break;
        }
    }
    // duplicate ids: keep the first
    let mut seen = HashSet::new();
    for p in plugins.iter_mut() {
        if !seen.insert(p.manifest.id.clone()) && p.problem.is_none() {
            p.problem = Some("another plugin already uses this id".into());
        }
    }
}
