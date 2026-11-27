//! What the active plugins add to the app, gathered in one place.

use std::collections::{HashMap, HashSet};
use crate::core::model::FieldDef;
use crate::core::thumbs::ExternalHandler;
use crate::core::plugins::pages::theme_variable;
use crate::core::plugins::manifest::ExtractorDecl;
use crate::core::plugins::manifest::COLUMN_RENDERERS;
use crate::core::plugins::manifest::Source;
use crate::core::plugins::manifest::Plugin;
use crate::core::plugins::manifest::Manifest;
use crate::core::plugins::manifest::ViewerKeyDecl;
use crate::core::plugins::manifest::ViewerToolDecl;
use crate::core::plugins::manifest::ScreenDecl;
use crate::core::plugins::manifest::TabDecl;
use crate::core::plugins::manifest::ColumnDecl;
use crate::core::plugins::manifest::PanelDecl;
use crate::core::plugins::manifest::SortDecl;
use crate::core::plugins::manifest::FilterDecl;
use crate::core::plugins::manifest::UrlDecl;
use crate::core::plugins::manifest::ActionDecl;
use crate::core::plugins::manifest::WidgetDecl;

/// Whether `url` fits `pattern`, where `*` stands for any run of characters; letters match in either case.
pub fn url_matches(pattern: &str, url: &str) -> bool {
    let (p, u): (Vec<char>, Vec<char>) = (pattern.to_lowercase().chars().collect(), url.to_lowercase().chars().collect());
    let (mut pi, mut ui, mut star, mut mark) = (0, 0, None, 0);
    while ui < u.len() {
        if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ui;
            pi += 1;
        } else if pi < p.len() && p[pi] == u[ui] {
            pi += 1;
            ui += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ui = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

/// Everything the rest of the app needs from active plugins, flattened.
#[derive(Clone, Default)]
pub struct Contributions {
    pub fields: Vec<FieldDef>,
    pub widgets: Vec<(String, WidgetDecl)>,
    pub actions: Vec<(String, ActionDecl)>,
    pub urls: Vec<(String, UrlDecl)>,
    pub filters: Vec<(String, FilterDecl)>,
    pub sorts: Vec<(String, SortDecl)>,
    pub panels: Vec<(String, PanelDecl)>,
    pub columns: Vec<(String, ColumnDecl)>,
    pub thumb_handlers: Vec<ExternalHandler>,
    /// Everything that reads fields from files and is on, Curio's own readers first.
    pub readers: Vec<crate::core::metadata::readers::Reader>,
    /// Every reader known, on or off, for telling which fields are read from files.
    pub known_readers: Vec<crate::core::metadata::readers::ReaderInfo>,
    pub tabs: Vec<(String, TabDecl)>,
    /// Extensions whose viewer is a plugin's page: plugin, extensions, page, its screen in the layout editor.
    pub viewers: Vec<(String, Vec<String>, String, ScreenDecl)>,
    pub viewer_tools: Vec<(String, ViewerToolDecl)>,
    pub viewer_keys: Vec<(String, ViewerKeyDecl)>,
    /// Extensions whose thumbnails a plugin's page makes.
    pub page_thumbs: Vec<PageThumb>,
    /// Extensions that open as a filter: plugin, extensions, the fields tried.
    pub opens_as_filter: Vec<(String, Vec<String>, Vec<String>)>,
    pub theme_variables: HashMap<String, String>,
    pub extra_exts: HashSet<String>,
    /// Extensions whose AssetKind comes from an active Format plugin's `formats[].kind`.
    pub ext_kinds: HashMap<String, crate::core::model::AssetKind>,
    /// Custom type labels declared by active Format plugins (`formats[].type`): `(label, exts)`.
    pub format_types: Vec<(String, Vec<String>)>,
    /// from built-in plugins that were switched off
    pub disabled_exts: HashSet<String>,
    /// Rules grouping files into items, in the order they are tried: those of Curio's own kinds of files, then the plugins'.
    pub companions: Vec<crate::core::grouping::companions::Rule>,
}

#[derive(Clone, Debug)]
pub struct PageThumb {
    pub plugin: String,
    pub exts: Vec<String>,
    pub page: String,
    pub size: u32,
}

impl Contributions {
    /// The plugin page that makes thumbnails for this extension, if one does.
    pub fn page_thumb(&self, ext: &str) -> Option<&PageThumb> {
        self.page_thumbs.iter().find(|t| t.exts.iter().any(|e| e.eq_ignore_ascii_case(ext)))
    }

    /// The handler an address goes to, if a plugin takes it; `besides` leaves that plugin's own out, so a plugin adding an address it handles doesn't hand it to itself.
    pub fn url_handler(&self, url: &str, besides: Option<&str>) -> Option<&(String, UrlDecl)> {
        self.urls.iter().filter(|(p, _)| Some(p.as_str()) != besides).find(|(_, u)| u.patterns.iter().any(|pat| url_matches(pat, url)))
    }
}

/// What an action or URL handler opens, when it is one of the plugin's pages: its window, or its tab as `tab:<id>`.
fn opens_ok(m: &Manifest, opens: &str) -> bool {
    match opens.strip_prefix("tab:") {
        Some(t) => m.tabs.iter().any(|x| x.id == t),
        None => m.windows.iter().any(|w| w.id == opens),
    }
}

/// What a plugin adds, for the tabs under Settings > Plugins. Something that only reads metadata
/// counts as a format.
pub fn plugin_types(m: &Manifest) -> Vec<&'static str> {
    // a kind of file is one entry, under Formats, whatever widgets it brings
    if m.id.starts_with("builtin.format.") {
        return vec!["format"];
    }
    let mut t = Vec::new();
    if !m.widgets.is_empty() {
        t.push("widget");
    }
    if !m.panels.is_empty() || !m.actions.is_empty() || !m.windows.is_empty() || !m.tabs.is_empty() {
        t.push("window");
    }
    if !m.formats.is_empty() || !m.extractors.is_empty() {
        t.push("format");
    }
    if t.is_empty() {
        t.push(if m.fields.is_empty() { "widget" } else { "format" });
    }
    t
}

pub fn contributions(plugins: &[Plugin]) -> Contributions {
    let mut c = Contributions::default();
    let ours = |p: &Plugin| p.source == Source::Builtin;
    let exts_of = |p: &Plugin| -> HashSet<String> { p.manifest.formats.iter().flat_map(|f| f.exts.iter().map(|e| e.trim_start_matches('.').to_lowercase())).collect() };
    // an extension another entry of Curio's that is on also claims stays that entry's
    let claimed: HashSet<String> = plugins.iter().filter(|p| p.active() && ours(p)).flat_map(exts_of).collect();
    for p in plugins.iter().filter(|p| !p.active() && ours(p)) {
        c.disabled_exts.extend(exts_of(p).into_iter().filter(|e| !claimed.contains(e)));
    }
    // a switched-off entry's readers stop, except for the extensions of its formats another entry that is on claims; one of no formats of its own stops for every file
    for mut r in crate::core::metadata::readers::builtin() {
        match plugins.iter().find(|p| p.manifest.id == r.owner).filter(|p| !p.active()) {
            None => {
                c.known_readers.push(r.info(true));
                c.readers.push(r);
            }
            Some(p) => {
                c.known_readers.push(r.info(false));
                let own = exts_of(p);
                r.exts.retain(|e| own.contains(e) && claimed.contains(e));
                if !r.exts.is_empty() {
                    c.readers.push(r);
                }
            }
        }
    }
    for p in plugins.iter().filter(|p| !p.active()) {
        c.known_readers.extend(crate::core::metadata::readers::extractor_readers(&p.manifest.id, &p.manifest.extractors).iter().map(|r| r.info(false)));
    }
    for p in plugins.iter().filter(|p| p.active()) {
        let id = p.manifest.id.clone();
        for f in &p.manifest.fields {
            let mut f = f.clone();
            f.owner = id.clone();
            c.fields.push(f);
        }
        // layouts place a widget by its id alone, so the first plugin to declare an id keeps it
        for w in &p.manifest.widgets {
            if let Some((other, _)) = c.widgets.iter().find(|(_, x)| x.id == w.id) {
                crate::core::system::logging::warn(&format!("plugin {id}: widget {} left out, {other} has one of that id", w.id));
                continue;
            }
            c.widgets.push((id.clone(), w.clone()));
        }
        for a in &p.manifest.actions {
            let mut a = a.clone();
            match (&a.opens, a.run.is_empty()) {
                (Some(o), true) if opens_ok(&p.manifest, o) => {
                    let listed = !a.places.is_empty();
                    a.places.retain(|w| w != "step");
                    if listed && a.places.is_empty() {
                        crate::core::system::logging::warn(&format!("plugin {id}: action {} opens a page, which can't be a step", a.id));
                        continue;
                    }
                }
                (None, false) if !p.has("exec") => a.disabled_reason = Some("needs the 'exec' permission".into()),
                (None, false) => {}
                (Some(o), true) => {
                    crate::core::system::logging::warn(&format!("plugin {id}: action {} opens {o}, which it doesn't have", a.id));
                    continue;
                }
                _ => {
                    crate::core::system::logging::warn(&format!("plugin {id}: action {} needs either `opens` or `run`", a.id));
                    continue;
                }
            }
            a.run = p.resolve(&a.run);
            a.dir = Some(p.dir.clone());
            c.actions.push((id.clone(), a));
        }
        for u in &p.manifest.urls {
            let mut u = u.clone();
            let ok = match (&u.opens, u.run.is_empty()) {
                (Some(o), true) => opens_ok(&p.manifest, o),
                (None, false) => p.has("exec"),
                _ => false,
            };
            if !ok || u.patterns.is_empty() {
                crate::core::system::logging::warn(&format!("plugin {id}: URL handler {} needs patterns and either a page it has in `opens` or `run` with the 'exec' permission", u.id));
                continue;
            }
            u.run = p.resolve(&u.run);
            u.dir = Some(p.dir.clone());
            c.urls.push((id.clone(), u));
        }
        for f in &p.manifest.filters {
            c.filters.push((id.clone(), f.clone()));
        }
        for s in &p.manifest.sorts {
            c.sorts.push((id.clone(), s.clone()));
        }
        for pn in &p.manifest.panels {
            c.panels.push((id.clone(), pn.clone()));
        }
        for col in &p.manifest.columns {
            if COLUMN_RENDERERS.contains(&col.render.as_str()) {
                c.columns.push((id.clone(), col.clone()));
            } else {
                crate::core::system::logging::warn(&format!("plugin {id}: column {} left out, the window has no renderer {:?}", col.id, col.render));
            }
        }
        for t in &p.manifest.tabs {
            c.tabs.push((id.clone(), t.clone()));
        }
        for t in &p.manifest.viewer_tools {
            c.viewer_tools.push((id.clone(), t.clone()));
        }
        if !p.manifest.viewer_keys.is_empty() && !p.manifest.formats.iter().any(|f| f.viewer.is_some()) {
            crate::core::system::logging::warn(&format!("plugin {id}: viewer_keys left out, it has no viewer page"));
        } else {
            c.viewer_keys.extend(p.manifest.viewer_keys.iter().map(|k| (id.clone(), k.clone())));
        }
        for f in &p.manifest.formats {
            for e in &f.exts {
                c.extra_exts.insert(e.clone());
            }
            for d in &f.companions {
                if let Ok(r) = crate::core::grouping::companions::Rule::from_decl(&id, &f.exts, d) {
                    c.companions.push(r);
                }
            }
            let exts: Vec<String> = f.exts.iter().map(|e| e.trim_start_matches('.').to_lowercase()).collect();
            if p.source != Source::Builtin && !f.kind.is_empty() {
                let kind = crate::core::model::AssetKind::parse(&f.kind);
                if kind != crate::core::model::AssetKind::Other {
                    for ext in &exts {
                        if crate::core::formats::family_of(ext).is_none() {
                            c.ext_kinds.entry(ext.clone()).or_insert(kind);
                        }
                    }
                }
            }
            if p.source != Source::Builtin {
                if let Some(label) = f.format_type.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    let claimed_type: HashSet<String> = c.format_types.iter().flat_map(|(_, es)| es.iter().cloned()).collect();
                    let fresh: Vec<String> = exts.iter().filter(|e| !e.is_empty() && !claimed_type.contains(*e)).cloned().collect();
                    if !fresh.is_empty() {
                        if let Some((_, existing)) = c.format_types.iter_mut().find(|(l, _)| l.eq_ignore_ascii_case(label)) {
                            existing.extend(fresh);
                        } else {
                            c.format_types.push((label.to_string(), fresh));
                        }
                    }
                }
            }
            if let Some(v) = &f.viewer {
                // a format without a screen shares the first one the plugin declares, so one plugin can show all its formats on one screen
                let screen = f.screen.clone().or_else(|| p.manifest.formats.iter().find_map(|g| g.screen.clone())).unwrap_or_else(|| ScreenDecl { id: "files".into(), title: p.manifest.name.clone(), icon: p.manifest.icon.clone(), toolbar: Vec::new(), inspector: Vec::new() });
                c.viewers.push((id.clone(), exts.clone(), v.clone(), screen));
            }
            if let Some(o) = f.open.as_ref().filter(|o| !o.show_with.is_empty()) {
                c.opens_as_filter.push((id.clone(), exts.clone(), o.show_with.clone()));
            }
            // the page reads the file through the API, which needs files.read; a built-in family's thumbnails are the window's own
            if let Some(t) = f.thumbnail_page.as_ref().filter(|_| p.has("files.read") || p.source == Source::Builtin) {
                c.page_thumbs.push(PageThumb { plugin: id.clone(), exts, page: t.page.clone(), size: t.size.unwrap_or(crate::core::consts::THUMBNAIL_SIZE).clamp(64, 2048) });
            }
            if let Some(t) = &f.thumbnail {
                if p.has("exec") && p.has("files.read") {
                    c.thumb_handlers.push(ExternalHandler {
                        plugin_id: id.clone(),
                        exts: f.exts.clone(),
                        command: p.resolve(&t.command),
                        dir: Some(p.dir.clone()),
                        report: Default::default(),
                    });
                }
            }
        }
        let decls: Vec<ExtractorDecl> = p.manifest.extractors.iter().map(|x| ExtractorDecl { command: p.resolve(&x.command), dir: Some(p.dir.clone()), ..x.clone() }).collect();
        for r in crate::core::metadata::readers::extractor_readers(&id, &decls) {
            let runs = match &r.source {
                crate::core::metadata::readers::Source::Extractor(x) => (x.command.is_empty() || (p.has("exec") && p.has("files.read"))) && (x.page.is_none() || p.has("files.read")),
                crate::core::metadata::readers::Source::Builtin(_) => false,
            };
            c.known_readers.push(r.info(runs));
            if runs {
                c.readers.push(r);
            }
        }
        for (k, v) in &p.manifest.theme_variables {
            match theme_variable(k, v) {
                Some((k, v)) => {
                    c.theme_variables.insert(k, v);
                }
                None => crate::core::system::logging::warn(&format!("plugin {id}: theme variable {k} ignored (not a plain name and value)")),
            }
        }
    }
    let on: HashSet<String> = crate::core::formats::FAMILIES.iter().filter(|f| plugins.iter().any(|p| p.active() && p.manifest.id == crate::core::formats::plugin_id(f))).map(|f| f.id.to_string()).collect();
    let mut rules = crate::core::grouping::companions::builtin_rules(&on);
    rules.append(&mut c.companions);
    c.companions = rules;
    c
}
