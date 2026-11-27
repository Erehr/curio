//! What a plugin's `plugin.json` declares, and the plugin as loaded.

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::core::model::FieldDef;

fn one() -> u32 {
    1
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExtractorDecl {
    #[serde(default)]
    pub exts: Vec<String>,
    /// PNG text chunk name → field key (copies the chunk text).
    #[serde(default)]
    pub png_text: HashMap<String, String>,
    /// Regex rules over a PNG text chunk: first capture group becomes the field value.
    #[serde(default)]
    pub regex: Vec<RegexRule>,
    /// Command printing a JSON object {field_key: value, ...}. {input} = file path, {parts} = the files grouped into it as a JSON array of {role, path}.
    #[serde(default)]
    pub command: Vec<String>,
    /// Values taken from a JSON file grouped into the item.
    #[serde(default)]
    pub json: Option<JsonSource>,
    /// A page of the plugin's that reads the file in the window, told `plugin.extract`: docs/plugins/manifest/extractors.md.
    #[serde(default)]
    pub page: Option<String>,
    /// The reader's name after the plugin's id; `x<n>`, its place in the list, without one.
    #[serde(default)]
    pub id: Option<String>,
    /// Raised when what it reads from the same file changes, so each library reads its files again once.
    #[serde(default)]
    pub version: u32,
    /// The fields the command fills: those it no longer prints for a file are cleared when the file is read again. Without them nothing it wrote is cleared.
    #[serde(default)]
    pub fields: Vec<String>,
    /// the plugin's folder, where its programs run
    #[serde(skip)]
    pub dir: Option<PathBuf>,
}

/// The first file of `role` grouped into the item whose name ends with `file`, read as JSON: each field takes the value at its JSON Pointer.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsonSource {
    pub role: String,
    #[serde(default)]
    pub file: String,
    /// field key → JSON Pointer (`/civitai/trainedWords`)
    pub fields: HashMap<String, String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RegexRule {
    pub chunk: String,
    pub pattern: String,
    pub field: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WidgetDecl {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub fields: Vec<String>,
    /// asset kinds the widget applies to; empty = all
    #[serde(default)]
    pub kinds: Vec<String>,
    /// file extensions the widget applies to, lower case; empty = all
    #[serde(default)]
    pub exts: Vec<String>,
    #[serde(default = "yes")]
    pub default_visible: bool,
    /// A page of the plugin's own in place of the fields, relative to its folder.
    #[serde(default)]
    pub page: Option<String>,
    /// The page's height to begin with, in pixels.
    #[serde(default)]
    pub height: Option<u32>,
    /// When a page widget shows: `one` item selected (the default), `many` (2+), `some` (1+), or `any` selection, none included.
    #[serde(default)]
    pub selection: String,
    /// A Tabler icon name: the widget's button where it is too narrow to show (a toolbar, a sidebar of icons).
    #[serde(default)]
    pub icon: Option<String>,
    /// Shown without a heading of its own (and so never folded), as Curio's name and notes fields are.
    #[serde(default)]
    pub headless: bool,
}

/// A command in Curio's item menu, toolbar or viewer, or a step of Curio's own actions: it opens one of the plugin's pages with the items, or runs a program on them.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ActionDecl {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub icon: String,
    /// `item-menu` (the default), `toolbar`, `viewer` (beside an opened item) and `step` (in Curio's actions; programs only).
    #[serde(default, rename = "where")]
    pub places: Vec<String>,
    /// The shortcut it comes with, as `Ctrl+Shift+R`; the person can change it in Settings › Shortcuts.
    #[serde(default)]
    pub key: Option<String>,
    /// When it applies: `one` item, `many`, `some` (one or more, the default) or `any` selection, none included.
    #[serde(default)]
    pub selection: String,
    /// Kinds and extensions the items must all have; empty for any.
    #[serde(default)]
    pub kinds: Vec<String>,
    #[serde(default)]
    pub exts: Vec<String>,
    /// One of the plugin's windows by id, or `tab:<id>`, opened with the items in `plugin.run`.
    #[serde(default)]
    pub opens: Option<String>,
    /// A program, given the items as JSON on its input.
    #[serde(default)]
    pub run: Vec<String>,
    #[serde(skip)]
    pub disabled_reason: Option<String>,
    /// the plugin's folder, where its programs run
    #[serde(skip)]
    pub dir: Option<PathBuf>,
}

/// Addresses a plugin takes over when they are dropped or pasted into Curio or sent to it (`item.add.fromUrls`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UrlDecl {
    pub id: String,
    pub title: String,
    /// Whole addresses with `*` for any run of characters, such as `https://www.youtube.com/watch*`.
    pub patterns: Vec<String>,
    /// A window by id, or `tab:<id>`, opened with the addresses in `plugin.run`.
    #[serde(default)]
    pub opens: Option<String>,
    /// A program, given the addresses as JSON on its input.
    #[serde(default)]
    pub run: Vec<String>,
    #[serde(skip)]
    pub dir: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FilterDecl {
    pub id: String,
    pub label: String,
    pub query: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SortDecl {
    pub label: String,
    pub field: String,
}

/// A list view column the window draws with one of its own renderers (`COLUMN_RENDERERS`), for the items of `kinds`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ColumnDecl {
    pub id: String,
    pub label: String,
    pub render: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub kinds: Vec<String>,
}

/// What a plugin's column can draw: `font`, the item's font writing the window's sample text.
pub const COLUMN_RENDERERS: &[&str] = &["font"];

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PanelDecl {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub filters: Vec<FilterDecl>,
    #[serde(default)]
    pub text: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CommandDecl {
    pub command: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FormatDecl {
    pub exts: Vec<String>,
    #[serde(default)]
    pub kind: String,
    /// Optional custom type label shown in Group by › Type, Types filter, and the list Type column (e.g. "Books", "Models").
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub format_type: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<CommandDecl>,
    /// A page that shows these files in the viewer, in place of Curio's own.
    #[serde(default)]
    pub viewer: Option<String>,
    /// A page that makes their thumbnails while a window is open.
    #[serde(default)]
    pub thumbnail_page: Option<PageThumbDecl>,
    /// A layout of their own in the layout editor while the viewer page shows them; without one, the plugin's files share one named after it.
    #[serde(default)]
    pub screen: Option<ScreenDecl>,
    /// Files beside one of these that belong to it, grouped into one item.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub companions: Vec<CompanionDecl>,
    /// What opening one of these does instead of showing it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<OpenDecl>,
}

/// Opening a file whose name other items record in one of these fields shows All through that field set to its name, the field more items record it in winning.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OpenDecl {
    #[serde(default)]
    pub show_with: Vec<String>,
}

/// Files named after a primary, in its directory, that become members of its item with this role.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CompanionDecl {
    pub role: String,
    /// `{stem}` or `{name}` then literal text, in order of preference
    pub files: Vec<String>,
    /// the item's thumbnail is made from the first of these files there is
    #[serde(default)]
    pub cover: bool,
    /// a check of Curio's own the pair has to pass: `live-photo`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify: Option<String>,
}

/// A screen of the layout editor for a format's files: its name, and the toolbar it starts with (Curio's widgets and the plugin's `viewer_tools`; all of them when empty).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ScreenDecl {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub toolbar: Vec<String>,
    /// Widgets put in the Inspector for these files, below Curio's: the plugin's own page widgets or Curio's.
    #[serde(default)]
    pub inspector: Vec<String>,
}

/// One of a viewer page's controls as a widget of the toolbar, drawn from what the page gives `viewer.setTools` under the same id.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ViewerToolDecl {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub icon: Option<String>,
    /// `button` (the default), `slider` or `text`: how the layout editor shows it.
    #[serde(default)]
    pub kind: String,
}

/// A key for a viewer page, told to it as `plugin.tool` with this id while the page shows and lists it as working now.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ViewerKeyDecl {
    pub id: String,
    pub title: String,
    /// The shortcut it comes with, as `Shift+]`; the person can change it in Settings › Shortcuts.
    #[serde(default)]
    pub key: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PageThumbDecl {
    pub page: String,
    #[serde(default)]
    pub size: Option<u32>,
}

/// A Tab plugin: a page in the middle of the main window, beside the grid and All tags.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TabDecl {
    pub id: String,
    pub title: String,
    pub page: String,
    #[serde(default)]
    pub icon: String,
}

/// A Window plugin: a page of the plugin's own, shown in a Curio window of its own.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WindowDecl {
    pub id: String,
    pub title: String,
    /// The page, relative to the plugin's folder.
    pub page: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    /// Several windows of it at once; otherwise opening it again brings the one open forward.
    #[serde(default)]
    pub multiple: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default = "one")]
    pub api: u32,
    /// Minimum Curio version required (e.g. `"1.2.0"`), for features added within the same `api` version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_curio: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    /// A Tabler icon's name, for the Plugins menu and Settings.
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub extractors: Vec<ExtractorDecl>,
    /// one plugin can register any number of widgets
    #[serde(default)]
    pub widgets: Vec<WidgetDecl>,
    #[serde(default)]
    pub actions: Vec<ActionDecl>,
    #[serde(default)]
    pub urls: Vec<UrlDecl>,
    #[serde(default)]
    pub filters: Vec<FilterDecl>,
    #[serde(default)]
    pub sorts: Vec<SortDecl>,
    #[serde(default)]
    pub panels: Vec<PanelDecl>,
    #[serde(default)]
    pub columns: Vec<ColumnDecl>,
    #[serde(default)]
    pub formats: Vec<FormatDecl>,
    #[serde(default)]
    pub windows: Vec<WindowDecl>,
    #[serde(default)]
    pub tabs: Vec<TabDecl>,
    /// Its viewer pages' controls, as widgets for the toolbar of its formats' screens.
    #[serde(default)]
    pub viewer_tools: Vec<ViewerToolDecl>,
    /// Keys for its viewer pages.
    #[serde(default)]
    pub viewer_keys: Vec<ViewerKeyDecl>,
    /// Programs Curio finds for it, by their names in `tools::TOOLS`, for `process.run`.
    #[serde(default)]
    pub tools: Vec<String>,
    /// The hosts its pages may reach with the `network` permission; none listed means any `https:` site.
    #[serde(default)]
    pub network: Vec<String>,
    /// A page shown under Settings › Plugins for the plugin's own settings.
    #[serde(default)]
    pub settings: Option<String>,
    #[serde(default)]
    pub theme_variables: HashMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Source {
    Builtin,
    Native,
}

#[derive(Clone, Debug)]
pub struct Plugin {
    pub manifest: Manifest,
    pub dir: PathBuf,
    pub source: Source,
    pub enabled: bool,
    pub configured: bool,
    pub granted: Vec<String>,
    /// why it is not active (bad api version, missing dependency, too many failures…)
    pub problem: Option<String>,
}

impl Manifest {
    /// Whether it shows pages of its own, whose addresses carry its id.
    pub fn has_pages(&self) -> bool {
        !self.windows.is_empty() || !self.tabs.is_empty() || self.settings.is_some() || self.widgets.iter().any(|w| w.page.is_some()) || self.formats.iter().any(|f| f.viewer.is_some() || f.thumbnail_page.is_some()) || self.extractors.iter().any(|e| e.page.is_some())
    }
}

impl Plugin {
    pub fn active(&self) -> bool {
        self.enabled && self.problem.is_none()
    }

    pub fn has(&self, perm: &str) -> bool {
        self.source == Source::Builtin || self.granted.iter().any(|g| g == perm)
    }

    /// Resolve a command whose program may be relative to the plugin folder.
    pub fn resolve(&self, argv: &[String]) -> Vec<String> {
        let mut v = argv.to_vec();
        if let Some(first) = v.first_mut() {
            let candidate = self.dir.join(&*first);
            if candidate.exists() {
                *first = candidate.to_string_lossy().to_string();
            }
        }
        v
    }
}

impl Plugin {
    /// What the plugin's pages may do through the API: what was granted, and reading what Curio and its libraries are.
    pub fn api_perms(&self) -> Vec<crate::core::api::Perm> {
        use crate::core::api::Perm;
        let mut v = vec![Perm::LibraryRead];
        v.extend(self.granted.iter().filter_map(|g| Perm::parse(g)).filter(|p| *p != Perm::LibraryManage && *p != Perm::LibraryRead));
        v
    }
}
