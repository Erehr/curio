//! Plain data types shared by every layer. No I/O here.

use serde::{Deserialize, Serialize};

pub type AssetId = i64;

/// Where an asset's bytes live.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RootKind {
    /// Files indexed in place (default). Never moved or modified.
    Linked,
    /// Files copied into the library's partitioned `managed/` store.
    Managed,
    /// A kind written by a newer version of Curio; kept offline and untouched.
    Unknown,
}

impl RootKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RootKind::Linked => "linked",
            RootKind::Managed => "managed",
            RootKind::Unknown => "unknown",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "managed" => RootKind::Managed,
            "linked" => RootKind::Linked,
            _ => RootKind::Unknown,
        }
    }
}

/// Whether Curio may write files inside a root. The index is always editable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootAccess {
    /// Curio may write files here (turn, replace, frames, sidecars) but never renames, moves or removes them.
    Full,
    ReadOnly,
    /// `Full`, and an item's file is renamed with the item and may be moved out of the location.
    Organize,
}

impl RootAccess {
    pub fn as_str(self) -> &'static str {
        match self {
            RootAccess::Full => "full",
            RootAccess::ReadOnly => "read_only",
            RootAccess::Organize => "organize",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "full" => RootAccess::Full,
            "organize" => RootAccess::Organize,
            _ => RootAccess::ReadOnly,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Root {
    pub id: i64,
    pub path: String,
    pub kind: RootKind,
    pub label: String,
    pub enabled: bool,
    pub watch: bool,
    /// Only files explicitly added are tracked (used for loose files outside folder roots, and an Eagle library's items).
    pub explicit_only: bool,
    /// Shown among the locations: a folder added as one, or an Eagle library's, but not the one made for files linked one by one.
    pub listed: bool,
    pub online: bool,
    pub last_scan: Option<i64>,
    pub asset_count: i64,
    pub access: RootAccess,
    /// `network`, `removable` or `read_only_media`; filled only where the UI lists roots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<&'static str>,
    /// chosen in the sidebar; without one the icon follows the kind of drive
    pub icon: Option<String>,
    pub color: Option<String>,
}

/// File availability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Availability {
    Ok = 0,
    /// The storage (drive, share) holding the file is not reachable right now.
    Offline = 1,
    /// Storage is reachable but the file is gone and was not found elsewhere.
    Missing = 2,
}

impl Availability {
    pub fn from_i64(v: i64) -> Self {
        match v {
            1 => Availability::Offline,
            2 => Availability::Missing,
            _ => Availability::Ok,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Availability::Ok => "Available",
            Availability::Offline => "Storage offline",
            Availability::Missing => "Missing",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    Image,
    Video,
    Audio,
    Document,
    Code,
    Link,
    Archive,
    Font,
    #[serde(rename = "model")]
    Model3d,
    Other,
}

impl AssetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AssetKind::Image => "image",
            AssetKind::Video => "video",
            AssetKind::Audio => "audio",
            AssetKind::Document => "document",
            AssetKind::Code => "code",
            AssetKind::Link => "link",
            AssetKind::Archive => "archive",
            AssetKind::Font => "font",
            AssetKind::Model3d => "model",
            AssetKind::Other => "other",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "image" => AssetKind::Image,
            "video" => AssetKind::Video,
            "audio" => AssetKind::Audio,
            "document" => AssetKind::Document,
            "code" => AssetKind::Code,
            "link" => AssetKind::Link,
            "archive" => AssetKind::Archive,
            "font" => AssetKind::Font,
            "model" => AssetKind::Model3d,
            _ => AssetKind::Other,
        }
    }
    pub const ALL: [AssetKind; 10] = [
        AssetKind::Image,
        AssetKind::Video,
        AssetKind::Audio,
        AssetKind::Document,
        AssetKind::Code,
        AssetKind::Link,
        AssetKind::Archive,
        AssetKind::Font,
        AssetKind::Model3d,
        AssetKind::Other,
    ];
}

/// One colour of an asset's palette. `ratio` is a percentage (0..100).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaletteColor {
    pub color: [u8; 3],
    pub ratio: f32,
}

/// A full asset record as shown in the Inspector.
#[derive(Clone, Debug, Serialize)]
pub struct Asset {
    pub id: AssetId,
    pub uid: String,
    pub root_id: i64,
    pub rel_path: String,
    pub abs_path: String,
    pub name: String,
    pub display_name: String,
    pub ext: String,
    pub mime: String,
    pub kind: AssetKind,
    pub size: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub aspect: f32,
    pub duration: Option<f64>,
    pub ctime: Option<i64>,
    pub mtime: Option<i64>,
    pub added_at: i64,
    pub modified_at: i64,
    pub rating: i64,
    pub favorite: bool,
    pub explicit: bool,
    pub annotation: String,
    pub source_url: String,
    pub palette: Vec<PaletteColor>,
    pub quick_hash: Option<String>,
    pub content_hash: Option<String>,
    pub status: Availability,
    pub deleted: bool,
    pub thumb_state: i64,
    pub thumb_ver: i64,
    /// The item this file is grouped into; none for an item.
    pub part_of: Option<AssetId>,
    pub part_role: Option<String>,
    /// How many files are grouped into this item.
    pub parts: i64,
}

impl Asset {
    pub fn title(&self) -> &str {
        if self.display_name.is_empty() {
            &self.name
        } else {
            &self.display_name
        }
    }
}

/// The light record the grid needs per visible cell.
#[derive(Clone, Debug, Serialize)]
pub struct GridItem {
    pub id: AssetId,
    pub uid: String,
    pub title: String,
    pub ext: String,
    pub kind: AssetKind,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub aspect: f32,
    pub size: i64,
    pub rating: i64,
    pub favorite: bool,
    pub explicit: bool,
    pub status: Availability,
    pub thumb_state: i64,
    pub thumb_ver: i64,
    pub mtime: Option<i64>,
    pub animated: bool,
    /// degrees the viewer turns it by, without touching the file
    pub rotation: i64,
    pub tags_preview: String,
    /// its root is read-only, so actions that write the file are hidden
    pub read_only: bool,
    /// copied into the library's store
    pub managed: bool,
    /// its location lets Curio rename and move its files
    pub organize: bool,
    /// the start of its note, for the card's caption
    pub note: String,
    /// files grouped into it, and the role the tile's badge shows
    pub parts: i64,
    pub part_role: Option<String>,
    /// split by hand: rules leave it alone
    pub solo: bool,
    /// a Live Photo's clip, played on hover
    pub motion: Option<AssetId>,
    /// the other file of a RAW and JPEG pair, which the viewer can show instead
    pub pair: Option<AssetId>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Folder {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i64,
    pub count: i64,
    pub icon: Option<String>,
    pub auto_tags: Vec<String>,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub group_id: Option<i64>,
    pub color: Option<String>,
    pub count: i64,
    pub starred: bool,
    /// every group the tag belongs to; the first one gives it its colour
    pub group_ids: Vec<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TagGroup {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SmartFolder {
    pub id: i64,
    pub name: String,
    pub query: String,
    pub sort: Option<String>,
    pub icon: Option<String>,
    pub group_by: Option<String>,
    pub parent_id: Option<i64>,
    pub sort_order: i64,
    /// items it finds, filled in for the sidebar
    pub count: Option<i64>,
}

/// One virtual subfolder of a smart folder grouped by a property or field.
#[derive(Clone, Debug, Serialize)]
pub struct SmartBucket {
    pub value: String,
    pub label: String,
    pub count: i64,
}

// ---------------------------------------------------------------- custom fields

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldType {
    Text,
    Integer,
    Float,
    Boolean,
    Date,
    DateTime,
    SingleSelect,
    MultiSelect,
    Json,
    Url,
    FileRef,
    Unknown(String),
}

impl Serialize for FieldType {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for FieldType {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(FieldType::parse(&s).unwrap_or(FieldType::Unknown(s)))
    }
}

impl FieldType {
    pub fn as_str(&self) -> &str {
        match self {
            FieldType::Text => "text",
            FieldType::Integer => "integer",
            FieldType::Float => "float",
            FieldType::Boolean => "boolean",
            FieldType::Date => "date",
            FieldType::DateTime => "datetime",
            FieldType::SingleSelect => "single_select",
            FieldType::MultiSelect => "multi_select",
            FieldType::Json => "json",
            FieldType::Url => "url",
            FieldType::FileRef => "file_ref",
            FieldType::Unknown(s) => s.as_str(),
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "text" | "string" => FieldType::Text,
            "integer" | "int" => FieldType::Integer,
            "float" | "number" | "real" => FieldType::Float,
            "boolean" | "bool" => FieldType::Boolean,
            "date" => FieldType::Date,
            "datetime" => FieldType::DateTime,
            "single_select" | "select" => FieldType::SingleSelect,
            "multi_select" | "multiselect" => FieldType::MultiSelect,
            "json" => FieldType::Json,
            "url" => FieldType::Url,
            "file_ref" | "file" => FieldType::FileRef,
            _ => return None,
        })
    }
    /// Values stored in `num_value` (comparisons use the numeric index).
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            FieldType::Integer | FieldType::Float | FieldType::Boolean | FieldType::Date | FieldType::DateTime
        )
    }
}

/// Validation rules a schema can declare.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Validation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default)]
    pub required: bool,
}

/// A custom metadata field definition (user- or plugin-owned).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FieldDef {
    #[serde(default)]
    pub id: i64,
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub ftype: FieldType,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub validation: Validation,
    #[serde(default)]
    pub format: String,
    /// Presentation: "row" (default), "block" or "code".
    #[serde(default)]
    pub widget: String,
    /// "user" or the id of the plugin that registered it.
    #[serde(default = "user_owner")]
    pub owner: String,
    /// Other plugins that declare the same field with the same type: each writes it for the files of its own formats.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_with: Vec<String>,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default)]
    pub sort_order: i64,
    /// a filter chip and a smart folder rule of its own; any field can still be filtered as a "Custom field"
    #[serde(default)]
    pub own_filter: bool,
    /// how that filter asks: "list", "range", "text" or "has"; empty picks by type
    #[serde(default)]
    pub filter_kind: String,
    /// Curio keeps what readers find for it, and it takes part in filters, sorts, columns and search; a field no reader fills is always indexed.
    #[serde(default = "yes")]
    pub indexed: bool,
    /// The field this one only makes sense with (`lora_strength` with `lora`); switching one switches both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pair: Option<String>,
    /// Number fields of one owner with the same name here are one filter of that name, with a range for each (a 3D model's width, depth and height).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_group: Option<String>,
    /// Switched on without reading the files it applies to.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub needs_scan: bool,
    /// Unknown keys from a newer version of Curio, preserved on save.
    #[serde(flatten, default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

fn yes() -> bool {
    true
}
fn user_owner() -> String {
    "user".into()
}

impl FieldDef {
    /// Whether a plugin declared this field, first or as one sharing it.
    pub fn declared_by(&self, plugin: &str) -> bool {
        self.owner == plugin || self.shared_with.iter().any(|s| s == plugin)
    }
}

impl FieldDef {
    pub fn new(key: &str, label: &str, ftype: FieldType, owner: &str) -> Self {
        FieldDef {
            id: 0,
            key: key.into(),
            label: label.into(),
            ftype,
            description: String::new(),
            default_value: None,
            options: vec![],
            validation: Validation::default(),
            format: String::new(),
            widget: String::new(),
            owner: owner.into(),
            shared_with: vec![],
            visible: true,
            sort_order: 0,
            own_filter: false,
            filter_kind: String::new(),
            indexed: true,
            pair: None,
            filter_group: None,
            needs_scan: false,
            extra: std::collections::BTreeMap::new(),
        }
    }
    pub fn multi(&self) -> bool {
        matches!(self.ftype, FieldType::MultiSelect)
    }
    /// Items can be sorted by it: an indexed field of one value that orders.
    pub fn sorts(&self) -> bool {
        self.indexed && !matches!(self.ftype, FieldType::Json | FieldType::MultiSelect | FieldType::Unknown(_))
    }
    /// Its values are in the full-text index, so plain words find them.
    pub fn in_text_search(&self) -> bool {
        self.indexed && matches!(self.ftype, FieldType::Text | FieldType::SingleSelect | FieldType::MultiSelect | FieldType::Url)
    }
}

/// One stored value. Multi-valued fields have several, ordered by `idx`.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldValue {
    pub field_id: i64,
    pub idx: i64,
    pub text: Option<String>,
    pub num: Option<f64>,
}

/// Value as given by an extractor or the user before it is typed and validated.
#[derive(Clone, Debug, PartialEq)]
pub enum RawValue {
    Text(String),
    Num(f64),
    Bool(bool),
    List(Vec<RawValue>),
}

impl RawValue {
    pub fn text(s: impl Into<String>) -> Self {
        RawValue::Text(s.into())
    }
}

// ---------------------------------------------------------------- sorting

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SortKey {
    Manual,
    Imported,
    Modified,
    Created,
    Name,
    Ext,
    Size,
    Resolution,
    Rating,
    Duration,
    Random,
    Field(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sort {
    pub key: SortKey,
    pub descending: bool,
}

impl Default for Sort {
    fn default() -> Self {
        Sort { key: SortKey::Imported, descending: true }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GroupKey {
    None,
    Kind,
    Ext,
    Imported,
    Modified,
    Created,
    Rating,
    Size,
    Name,
    Location,
    Field(String),
}

/// Local calendar boundaries (epoch ms) for date grouping.
#[derive(Clone, Debug, Default)]
pub struct DateBounds {
    pub tomorrow: i64,
    pub today: i64,
    pub yesterday: i64,
    pub this_week: i64,
    pub last_week: i64,
    pub this_month: i64,
    pub last_month: i64,
    pub this_year: i64,
    pub tz_offset_ms: i64,
}

#[derive(Clone, Debug)]
pub struct GroupBy {
    pub key: GroupKey,
    pub descending: bool,
    pub dates: DateBounds,
}

impl Default for GroupBy {
    fn default() -> Self {
        GroupBy { key: GroupKey::None, descending: false, dates: DateBounds::default() }
    }
}

/// One contiguous group run in a grouped search result.
#[derive(Clone, Debug, Serialize)]
pub struct GroupSpan {
    pub key: String,
    pub kind: String,
    pub val: String,
    pub start: u32,
    pub count: u32,
}

/// Which part of the library the grid shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Scope {
    All,
    Favorites,
    Uncategorized,
    Untagged,
    Recent,
    Random,
    Trash,
    Missing,
    Folder(i64),
    SmartFolder(i64),
    SmartBucket(i64, String),
    Tag(i64),
    Root(i64),
    /// A folder inside a location: everything below `rel` (relative, `/` between levels).
    RootDir(i64, String),
    /// Several sidebar folders selected together.
    Folders(Vec<i64>),
    Roots(Vec<i64>),
}

/// A file action recorded in `file_ops` before it starts.
/// `recycle`: send a purged managed copy to the Recycle Bin. `replace`: swap `tmp` (a file name beside the item) in for the item's file.
/// `size` and `mtime` are what the file at `rel_path` was when recorded; a file that differs is never touched.
#[derive(Clone, Debug, Default, Serialize)]
pub struct FileOp {
    pub id: i64,
    pub kind: String,
    pub root_id: i64,
    pub rel_path: String,
    pub tmp: Option<String>,
    pub asset_uid: Option<String>,
    pub size: Option<i64>,
    pub mtime: Option<i64>,
    /// `pending`, `copying` or `staged` (replace), `failed`
    pub state: String,
    pub error: Option<String>,
    pub created_at: i64,
    /// Target path (same root) of store ops that create, move or rename something.
    pub dest: Option<String>,
    /// Kind-specific JSON, e.g. the placement and folder a store op updates when it finishes.
    pub detail: Option<String>,
}

/// One file entry of a managed item in the store: an item in several library folders has one per folder, hard links to the same data where the filesystem allows.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Placement {
    pub id: i64,
    pub asset_id: AssetId,
    /// `None` = the store root (an item in no library folder).
    pub folder_id: Option<i64>,
    pub rel_path: String,
    /// false = an independent copy, on a filesystem without hard links.
    pub linked: bool,
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

static INSTALLATION_ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Set once at startup from the app config; empty in tools.
pub fn set_installation_id(id: String) {
    let _ = INSTALLATION_ID.set(id);
}

pub fn installation_id() -> &'static str {
    INSTALLATION_ID.get().map(String::as_str).unwrap_or("")
}

/// A new id: a UUIDv7 (RFC 9562), so ids sort by when they were made. The database keeps its 16 bytes (`uid_bytes`); everything else sees the standard text form.
pub fn new_uid() -> String {
    uuid::Uuid::now_v7().hyphenated().to_string()
}

/// An id as the database keeps it: a UUID's 16 bytes.
pub fn uid_bytes(s: &str) -> Option<[u8; 16]> {
    if s.len() != 36 {
        return None;
    }
    uuid::Uuid::try_parse(s).ok().map(|u| *u.as_bytes())
}

/// The text form of an id as the database keeps it.
pub fn uid_text(b: &[u8]) -> Option<String> {
    uuid::Uuid::from_slice(b).ok().map(|u| u.hyphenated().to_string())
}

/// Classify an extension into an asset kind.
pub fn kind_for_ext(ext: &str) -> AssetKind {
    crate::core::formats::kind_for_ext(ext)
}

pub fn mime_for_ext(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" | "jfif" | "jpe" => "image/jpeg",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "tif" | "tiff" => "image/tiff",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "heic" | "heif" => "image/heif",
        "jxl" => "image/jxl",
        "psd" => "image/vnd.adobe.photoshop",
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "ogg" | "oga" | "opus" => "audio/ogg",
        "ogv" | "ogm" => "video/ogg",
        "m4a" => "audio/x-m4a",
        "aac" => "audio/aac",
        "3gp" => "video/3gpp",
        "pdf" => "application/pdf",
        "txt" | "md" => "text/plain",
        "json" => "application/json",
        "html" | "htm" => "text/html",
        "zip" => "application/zip",
        "cbz" => "application/vnd.comicbook+zip",
        "7z" => "application/x-7z-compressed",
        "rar" | "cbr" => "application/vnd.rar",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttc" | "otc" => "font/collection",
        "glb" => "model/gltf-binary",
        "stl" => "model/stl",
        "obj" => "model/obj",
        _ => "application/octet-stream",
    }
}
