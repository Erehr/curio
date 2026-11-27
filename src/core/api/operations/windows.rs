//! The window's own operations, answered by the window showing the page.

use std::path::PathBuf;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::core::api::{library_named, read, same_path, with_library, ApiError, ApiResult, Ctx};
use crate::core::library::instance;
use crate::core::library::changes::Changed;
use crate::core::model::*;
use crate::core::thumbs;
use crate::core::api::operations::unreachable_library;
use crate::core::api::operations::file_missing;
use crate::core::api::operations::said;
use crate::core::api::operations::libraries::Nothing;
use crate::core::api::operations::done;
use crate::core::api::operations::item_reads::OneItem;
use crate::core::api::operations::uids;
use crate::core::api::operations::bad;
use crate::core::api::operations::one_id;
use crate::core::api::operations::item_reads::Items;
use crate::core::api::operations::ids_of;
use crate::core::api::operations::not_found;
use crate::core::api::operations::read_checked;
use crate::core::api::operations::ItemRef;

use crate::core::api::CallerKind;
use crate::core::api::operations::item_edits::tag_ids;
use crate::core::api::operations::locations::location_of;
use crate::core::api::operations::item_edits::one_tag;
use crate::core::api::operations::item_edits::folders_exist;
use crate::core::api::operations::item_edits::TagRef;
use crate::core::api::operations::item_json;
use crate::core::api::operations::assets_in_order;

/// Has the window do something and waits for its answer.
fn in_window(c: &Ctx, op: &str, args: Value, ask: bool) -> ApiResult {
    c.host.window(op, args, ask).unwrap_or_else(|| Err(ApiError::new("no_window", "This library has no window open")))
}

/// What a person is asked, with who asks when it isn't the window itself.
fn asking(c: &Ctx, mut args: Value) -> Value {
    if c.caller.kind != CallerKind::Window {
        args["from"] = json!(c.caller.name);
    }
    args
}

pub(super) fn short(what: &str, text: &str, max: usize) -> Result<(), ApiError> {
    if text.chars().count() > max {
        return Err(bad(format!("`{what}` is longer than {max} characters")));
    }
    Ok(())
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct SwitchTo {
    /// The library to show: its id from library.list, its folder or its name.
    to: String,
}

pub(super) fn library_switch(c: &Ctx, p: SwitchTo) -> ApiResult {
    let to = library_named(c.host, c.caller, &p.to)?;
    let me = c.host.me();
    if c.host.library().is_some_and(|m| same_path(&m, &to.path)) {
        return Ok(json!({"switched": false, "window": me.window}));
    }
    if let Some(h) = instance::holder(&to.path).filter(|h| !h.background && h.port != 0) {
        let _ = instance::tell(&h, "focus", &json!({}));
        return Ok(json!({"switched": false, "window": h.window}));
    }
    if !instance::is_library(&to.path) {
        return Err(unreachable_library(&to));
    }
    in_window(c, "library.switch", json!({"path": to.path}), false)?;
    Ok(json!({"switched": true, "window": me.window}))
}

pub(super) fn token_forget(c: &Ctx, _: Nothing) -> ApiResult {
    let id = c.caller.token.as_deref().ok_or_else(|| ApiError::new("bad_request", "Only a token can forget itself"))?;
    c.host.drop_token(id);
    Ok(done())
}

pub(super) fn app_theme(c: &Ctx, _: Nothing) -> ApiResult {
    c.host.theme().ok_or_else(|| ApiError::new("busy", "The window hasn't shown its theme yet"))
}

pub(super) fn selection_get(c: &Ctx, _: Nothing) -> ApiResult {
    let sel = c.host.selection();
    let items = read(c.host, |db| Ok(assets_in_order(db, &sel)?.iter().map(|a| item_json(c, db, a)).collect::<Vec<_>>()))?;
    Ok(Value::Array(items))
}

pub(super) fn selection_set(c: &Ctx, p: Items) -> ApiResult {
    let ids = ids_of(c, &p.ids)?;
    in_window(c, "selection.set", json!({"ids": ids}), false)
}

pub(super) fn view_get(c: &Ctx, _: Nothing) -> ApiResult {
    in_window(c, "view.get", json!({}), false)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct ViewShow {
    /// all, favorites, uncategorized, untagged, recent, random, trash, missing, folder, smartFolder, tag or location.
    what: String,
    /// The folder's, smart folder's or location's number, or the tag's name or number.
    id: Option<TagRef>,
}

pub(super) fn view_show(c: &Ctx, p: ViewShow) -> ApiResult {
    const VIEWS: [&str; 8] = ["all", "favorites", "uncategorized", "untagged", "recent", "random", "trash", "missing"];
    let number = |what: &str| match &p.id {
        Some(TagRef::Id(n)) => Ok(*n),
        _ => Err(bad(format!("{what} needs `id`: its number"))),
    };
    let (kind, id) = match p.what.as_str() {
        v if VIEWS.contains(&v) => (v, 0),
        "folder" => {
            let id = number("folder")?;
            folders_exist(c, &[id])?;
            ("folder", id)
        }
        "smartFolder" => {
            let id = number("smartFolder")?;
            if !read(c.host, |db| Ok(db.smart_folders()?.iter().any(|s| s.id == id)))? {
                return Err(not_found(format!("There is no smart folder {id}")));
            }
            ("smart", id)
        }
        "tag" => ("tag", one_tag(c, p.id.as_ref().ok_or_else(|| bad("tag needs `id`: its name or number"))?)?),
        "location" => {
            let id = number("location")?;
            location_of(c, id)?;
            ("root", id)
        }
        other => return Err(bad(format!("There is no view {other}; use all, favorites, uncategorized, untagged, recent, random, trash, missing, folder, smartFolder, tag or location"))),
    };
    in_window(c, "view.show", json!({"kind": kind, "id": id}), false)
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ShowWith {
    /// A tag's name.
    tag: Option<String>,
    /// A field's key, with `value`.
    field: Option<String>,
    /// The field's value: text, or a number for a number field.
    value: Option<Value>,
}

pub(super) fn view_show_with(c: &Ctx, p: ShowWith) -> ApiResult {
    match (p.tag, p.field, p.value) {
        (Some(tag), None, None) => {
            short("tag", &tag, 500)?;
            in_window(c, "view.showWith", json!({"tag": tag}), false)
        }
        (None, Some(field), Some(value)) => {
            if !read(c.host, |db| Ok(db.field_catalog().custom(&field).is_some()))? {
                return Err(not_found(format!("There is no field {field}")));
            }
            let value = match value {
                Value::String(s) => s,
                Value::Number(n) => n.to_string(),
                _ => return Err(bad("`value` is text or a number")),
            };
            short("value", &value, 1000)?;
            in_window(c, "view.showWith", json!({"field": field, "value": value}), false)
        }
        _ => Err(bad("Give `tag`, or `field` with `value`")),
    }
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct SearchText {
    /// Words, or a query such as `rating >= 4 AND tag = sky`; empty clears the search.
    text: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Sample {
    /// What the samples say; empty for the line each font's own script has. Left out, it stays as it is.
    text: Option<String>,
}

pub(super) fn view_set_sample(c: &Ctx, p: Sample) -> ApiResult {
    if let Some(t) = &p.text {
        short("text", t, 1000)?;
    }
    in_window(c, "view.setSample", json!({"text": p.text}), false)
}

pub(super) fn view_set_search(c: &Ctx, p: SearchText) -> ApiResult {
    short("text", &p.text, 10_000)?;
    in_window(c, "view.setSearch", json!({"text": p.text}), false)
}

/// The filters above the items; what is left out is cleared.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Filters {
    /// Items with these tags, by name or number.
    #[serde(default)]
    tags: Vec<TagRef>,
    /// How `tags` combine: any (the default), all, or exactly (these tags and no others).
    tag_rule: Option<String>,
    /// Items without any of these tags.
    #[serde(default)]
    tags_not: Vec<TagRef>,
    /// Items with no tags at all, besides those with `tags`.
    #[serde(default)]
    untagged: bool,
    /// Items in any of these folders, by number.
    #[serde(default)]
    folders: Vec<i64>,
    /// Items in none of these folders.
    #[serde(default)]
    folders_not: Vec<i64>,
    /// Star ratings, 0 to 5.
    #[serde(default)]
    ratings: Vec<i64>,
    /// True for favorites only, false to exclude favorites.
    favorite: Option<bool>,
    /// True for explicit items only, false to exclude explicit items.
    explicit: Option<bool>,
    /// image, video, audio, document, code, link, archive, font, model or other.
    #[serde(default)]
    kinds: Vec<String>,
    /// File extensions such as png or mp4.
    #[serde(default)]
    exts: Vec<String>,
    /// A colour as #rrggbb, or grayscale.
    color: Option<String>,
    /// landscape, portrait, square, wide, tall, 4:3 or 16:9.
    #[serde(default)]
    shapes: Vec<String>,
    /// Items added within this many days.
    added_within: Option<u32>,
    /// Items added on this day (YYYY-MM-DD) or later.
    added_from: Option<String>,
    /// Items added on this day (YYYY-MM-DD) or earlier.
    added_to: Option<String>,
    /// Items whose file was modified on this day (YYYY-MM-DD) or later.
    modified_from: Option<String>,
    /// Items whose file was modified on this day (YYYY-MM-DD) or earlier.
    modified_to: Option<String>,
    /// Items whose file was created on this day (YYYY-MM-DD) or later.
    created_from: Option<String>,
    /// Items whose file was created on this day (YYYY-MM-DD) or earlier.
    created_to: Option<String>,
}

/// A day as the window's filters keep it, checked to be one.
fn filter_day(name: &str, v: &Option<String>) -> Result<Option<String>, ApiError> {
    let Some(d) = v.as_deref().map(str::trim).filter(|d| !d.is_empty()) else { return Ok(None) };
    if chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_err() {
        return Err(bad(format!("`{name}` is a day as YYYY-MM-DD, not {d}")));
    }
    Ok(Some(d.to_string()))
}

pub(super) fn view_set_filters(c: &Ctx, p: Filters) -> ApiResult {
    const KINDS: [&str; 10] = ["image", "video", "audio", "document", "code", "link", "archive", "font", "model", "other"];
    const SHAPES: [&str; 7] = ["landscape", "portrait", "square", "wide", "tall", "4:3", "16:9"];
    let rule = match p.tag_rule.as_deref().unwrap_or("any") {
        "any" => "OR",
        "all" => "AND",
        "exactly" => "EQUAL",
        r => return Err(bad(format!("`tagRule` is any, all or exactly, not {r}"))),
    };
    let tags = read(c.host, |db| db.tags())?;
    let tag_names = |refs: &[TagRef]| -> Result<Vec<String>, ApiError> { Ok(tag_ids(c, refs)?.into_iter().filter_map(|id| tags.iter().find(|t| t.id == id).map(|t| t.name.clone())).collect()) };
    let folders = read(c.host, |db| db.folders())?;
    let folder_names = |ids: &[i64]| -> Result<Vec<String>, ApiError> { ids.iter().map(|id| folders.iter().find(|f| f.id == *id).map(|f| f.name.clone()).ok_or_else(|| not_found(format!("There is no folder {id}")))).collect() };
    if let Some(r) = p.ratings.iter().find(|r| !(0..=5).contains(*r)) {
        return Err(bad(format!("A rating is 0 to 5, not {r}")));
    }
    if let Some(k) = p.kinds.iter().find(|k| !KINDS.contains(&k.as_str())) {
        return Err(bad(format!("There is no kind {k}; use {}", KINDS.join(", "))));
    }
    if let Some(s) = p.shapes.iter().find(|s| !SHAPES.contains(&s.as_str())) {
        return Err(bad(format!("There is no shape {s}; use {}", SHAPES.join(", "))));
    }
    let color = match p.color.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(g) if g.eq_ignore_ascii_case("grayscale") => Some("grayscale".to_string()),
        Some(h) => {
            let hex = h.trim_start_matches('#');
            if hex.len() != 6 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return Err(bad(format!("`color` is #rrggbb or grayscale, not {h}")));
            }
            Some(format!("#{}", hex.to_uppercase()))
        }
    };
    let exts: Vec<String> = p.exts.iter().map(|e| e.trim().trim_start_matches('.').to_lowercase()).filter(|e| !e.is_empty()).collect();
    let f = json!({
        "tags": tag_names(&p.tags)?, "tagsNot": tag_names(&p.tags_not)?, "tagRule": rule, "untagged": p.untagged.then_some(true),
        "folders": folder_names(&p.folders)?, "foldersNot": folder_names(&p.folders_not)?,
        "ratings": p.ratings, "favorite": p.favorite, "explicit": p.explicit, "kinds": p.kinds, "exts": exts, "color": color, "shapes": p.shapes, "addedWithin": p.added_within,
        "addedFrom": filter_day("addedFrom", &p.added_from)?, "addedTo": filter_day("addedTo", &p.added_to)?,
        "modifiedFrom": filter_day("modifiedFrom", &p.modified_from)?, "modifiedTo": filter_day("modifiedTo", &p.modified_to)?,
        "createdFrom": filter_day("createdFrom", &p.created_from)?, "createdTo": filter_day("createdTo", &p.created_to)?,
    });
    in_window(c, "view.setFilters", f, false)
}

pub(super) fn viewer_open(c: &Ctx, p: OneItem) -> ApiResult {
    let id = one_id(c, &p.id)?;
    let item = read(c.host, |db| Ok(db.asset(id)?.and_then(|a| a.part_of)))?;
    in_window(c, "viewer.open", json!({"id": item.unwrap_or(id), "file": item.map(|_| id)}), false)?;
    Ok(done())
}

pub(super) fn viewer_close(c: &Ctx, _: Nothing) -> ApiResult {
    in_window(c, "viewer.close", json!({}), false)
}

pub(super) fn viewer_current(c: &Ctx, _: Nothing) -> ApiResult {
    let shown = in_window(c, "viewer.current", json!({}), false)?;
    let uid = |v: &Value| -> Result<Option<String>, ApiError> {
        Ok(match v.as_i64() {
            Some(i) => read(c.host, |db| Ok(uids(db, &[i])?))?.into_iter().next(),
            None => None,
        })
    };
    Ok(json!({"id": uid(&shown["id"])?, "file": uid(&shown["file"])?}))
}

/// Files that run code when opened; only the window opens them.
const RUNS_CODE: &[&str] = &[
    "exe", "com", "bat", "cmd", "ps1", "psm1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "hta", "msi", "msp", "scr", "pif", "cpl", "lnk", "jar", "reg", "inf", "scf", "msc", "application", "appref-ms",
    "settingcontent-ms", "library-ms", "searchconnector-ms", "sh", "bash", "command", "app", "appimage", "desktop", "run", "bin", "py", "pyw", "pl", "rb", "php",
];

fn file_of(c: &Ctx, id: AssetId) -> Result<(PathBuf, String), ApiError> {
    let (path, ext) = read_checked(c, |db| Ok(db.asset(id)?.map(|a| (db.abs_path(id), a.ext)).ok_or_else(|| not_found("No such item"))))?;
    match path.filter(|p| p.exists()) {
        Some(p) => Ok((p, ext.to_lowercase())),
        None => Err(file_missing()),
    }
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct OpenItem {
    id: ItemRef,
    /// viewer (the default): in the window's viewer; window: in a window of its own; app: in the app the system opens its type with.
    how: Option<String>,
}

pub(super) fn item_open(c: &Ctx, p: OpenItem) -> ApiResult {
    let id = one_id(c, &p.id)?;
    match p.how.as_deref().unwrap_or("viewer") {
        "viewer" => in_window(c, "viewer.open", json!({"id": id}), false)?,
        "window" => in_window(c, "item.window", json!({"ids": [id]}), false)?,
        "app" => {
            let (path, ext) = file_of(c, id)?;
            let outside = c.caller.kind != CallerKind::Window;
            if ext == "url" {
                // a saved link opens in the browser, and only a web address
                let url = thumbs::links::url_from_file(&path).filter(|u| u.starts_with("http://") || u.starts_with("https://")).ok_or_else(|| ApiError::new("not_allowed", "This link doesn't point to a web page"))?;
                opener::open(&url).map_err(|e| ApiError::new("failed", format!("Failed to open: {e}")))?;
            } else if outside && (RUNS_CODE.contains(&ext.as_str()) || path.is_dir()) {
                return Err(ApiError::new("not_allowed", format!("{} would run a program; only Curio's window opens such files", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())));
            } else {
                crate::core::system::filesystem::open_default(&path)?;
            }
            with_library(c.host, |lib| {
                lib.write(move |db, _| db.mark_used(id).map(|_| Changed::default()));
                Ok(())
            })?;
            done()
        }
        other => return Err(bad(format!("`how` is viewer, window or app, not {other}"))),
    };
    Ok(done())
}

pub(super) fn item_reveal(c: &Ctx, p: OneItem) -> ApiResult {
    let (path, _) = file_of(c, one_id(c, &p.id)?)?;
    crate::core::system::filesystem::reveal(&path)?;
    Ok(done())
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct Notify {
    text: String,
    /// info (the default) or error.
    kind: Option<String>,
}

pub(super) fn ui_notify(c: &Ctx, p: Notify) -> ApiResult {
    short("text", &p.text, 2000)?;
    let kind = match p.kind.as_deref().unwrap_or("info") {
        k @ ("info" | "error") => k,
        k => return Err(bad(format!("`kind` is info or error, not {k}"))),
    };
    in_window(c, "ui.notify", json!({"text": p.text, "kind": kind}), false)?;
    Ok(done())
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct Confirm {
    title: String,
    body: Option<String>,
    /// The confirming button's label; OK by default.
    ok_label: Option<String>,
}

pub(super) fn ui_confirm(c: &Ctx, p: Confirm) -> ApiResult {
    short("title", &p.title, 200)?;
    short("body", p.body.as_deref().unwrap_or(""), 4000)?;
    in_window(c, "ui.confirm", asking(c, json!({"title": p.title, "body": p.body, "okLabel": p.ok_label})), true)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct OpenUrl {
    url: String,
}

pub(super) fn app_open_url(c: &Ctx, p: OpenUrl) -> ApiResult {
    let url = p.url.trim();
    short("url", url, 2000)?;
    let lower = url.to_ascii_lowercase();
    if !["http://", "https://", "mailto:"].iter().any(|s| lower.starts_with(s)) || url.chars().any(char::is_control) {
        return Err(ApiError::new("bad_request", "Only http, https and mailto addresses open in the browser"));
    }
    // the person sees the whole address before anything leaves Curio
    let answer = in_window(c, "ui.confirm", asking(c, json!({"title": "Open this address in your browser?", "body": url, "okLabel": "Open"})), true)?;
    if answer["ok"] != json!(true) {
        return Ok(json!({"opened": false}));
    }
    opener::open(url).map_err(|e| ApiError::new("failed", format!("Failed to open: {e}")))?;
    Ok(json!({"opened": true}))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct Prompt {
    title: String,
    body: Option<String>,
    /// The text the box starts with.
    value: Option<String>,
    placeholder: Option<String>,
    ok_label: Option<String>,
}

pub(super) fn ui_prompt(c: &Ctx, p: Prompt) -> ApiResult {
    short("title", &p.title, 200)?;
    short("body", p.body.as_deref().unwrap_or(""), 4000)?;
    in_window(c, "ui.prompt", asking(c, json!({"title": p.title, "body": p.body, "value": p.value, "placeholder": p.placeholder, "okLabel": p.ok_label})), true)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct PickTags {
    title: Option<String>,
    /// Tags chosen to begin with, by name or number; names that aren't tags yet are left out.
    #[serde(default)]
    selected: Vec<TagRef>,
}

pub(super) fn ui_pick_tags(c: &Ctx, p: PickTags) -> ApiResult {
    let ids: Vec<i64> = p.selected.iter().filter_map(|r| tag_ids(c, std::slice::from_ref(r)).ok()).flatten().collect();
    let names: Vec<String> = read(c.host, |db| db.tags())?.into_iter().filter(|t| ids.contains(&t.id)).map(|t| t.name).collect();
    in_window(c, "ui.pickTags", asking(c, json!({"title": p.title, "selected": names})), true)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct PickFolders {
    title: Option<String>,
    /// Folders chosen to begin with, by number.
    #[serde(default)]
    selected: Vec<i64>,
}

pub(super) fn ui_pick_folders(c: &Ctx, p: PickFolders) -> ApiResult {
    folders_exist(c, &p.selected)?;
    in_window(c, "ui.pickFolders", asking(c, json!({"title": p.title, "selected": p.selected})), true)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct Titled {
    title: Option<String>,
}

pub(super) fn ui_pick_field(c: &Ctx, p: Titled) -> ApiResult {
    in_window(c, "ui.pickField", asking(c, json!({"title": p.title})), true)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct PickColor {
    title: Option<String>,
    /// The colour to begin with, as #rrggbb.
    value: Option<String>,
}

pub(super) fn ui_pick_color(c: &Ctx, p: PickColor) -> ApiResult {
    in_window(c, "ui.pickColor", asking(c, json!({"title": p.title, "value": p.value})), true)
}

/// One entry of a menu.
#[derive(Deserialize, JsonSchema, serde::Serialize)]
struct MenuEntry {
    /// What the answer names when this entry is chosen.
    id: Option<String>,
    label: Option<String>,
    /// A Tabler icon's name, such as tag or folder.
    icon: Option<String>,
    #[serde(default)]
    disabled: bool,
    /// A line between entries; the rest is ignored.
    #[serde(default)]
    separator: bool,
}

#[derive(Deserialize, JsonSchema)]
struct MenuItem {
    #[serde(flatten)]
    entry: MenuEntry,
    /// A submenu, one level deep.
    #[serde(default)]
    items: Vec<MenuEntry>,
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct Menu {
    items: Vec<MenuItem>,
}

pub(super) fn ui_menu(c: &Ctx, p: Menu) -> ApiResult {
    let check = |e: &MenuEntry, sub: bool| -> Result<(), ApiError> {
        if !e.separator && (e.label.as_deref().unwrap_or("").is_empty() || (e.id.is_none() && !sub)) {
            return Err(bad("Each menu entry needs `label` and `id`, unless it is a separator or has `items`"));
        }
        short("label", e.label.as_deref().unwrap_or(""), 200)
    };
    if p.items.is_empty() || p.items.len() + p.items.iter().map(|i| i.items.len()).sum::<usize>() > 200 {
        return Err(bad("A menu has 1 to 200 entries"));
    }
    for i in &p.items {
        check(&i.entry, !i.items.is_empty())?;
        for s in &i.items {
            check(s, false)?;
        }
    }
    let items: Vec<Value> = p.items.iter().map(|i| {
        let mut v = json!(i.entry);
        if !i.items.is_empty() {
            v["items"] = json!(i.items);
        }
        v
    }).collect();
    in_window(c, "ui.menu", json!({"items": items}), true)
}

/// Files a dialog shows: a name and the extensions it covers.
#[derive(Deserialize, JsonSchema)]
struct FileFilter {
    name: String,
    /// Without dots, such as png or jpg.
    extensions: Vec<String>,
}

fn filters_json(f: &[FileFilter]) -> Value {
    Value::Array(f.iter().map(|f| json!({"name": f.name, "extensions": f.extensions.iter().map(|e| e.trim_start_matches('.')).collect::<Vec<_>>()})).collect())
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct OpenFile {
    title: Option<String>,
    /// Several files at once.
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    filters: Vec<FileFilter>,
}

pub(super) fn dialog_open_file(c: &Ctx, p: OpenFile) -> ApiResult {
    in_window(c, "dialog.openFile", json!({"title": p.title, "multiple": p.multiple, "filters": filters_json(&p.filters)}), true)
}

pub(super) fn dialog_open_folder(c: &Ctx, p: Titled) -> ApiResult {
    in_window(c, "dialog.openFolder", json!({"title": p.title}), true)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct SaveFile {
    title: Option<String>,
    /// The name the dialog suggests.
    file_name: Option<String>,
    #[serde(default)]
    filters: Vec<FileFilter>,
}

pub(super) fn dialog_save_file(c: &Ctx, p: SaveFile) -> ApiResult {
    in_window(c, "dialog.saveFile", json!({"title": p.title, "fileName": p.file_name, "filters": filters_json(&p.filters)}), true)
}

pub(super) fn clipboard_read_text(_: &Ctx, _: Nothing) -> ApiResult {
    Ok(json!({"text": crate::core::import::clipboard::clipboard_text()}))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct ClipText {
    text: String,
}

pub(super) fn clipboard_write_text(_: &Ctx, p: ClipText) -> ApiResult {
    arboard::Clipboard::new().and_then(|mut cb| cb.set_text(p.text)).map_err(|e| ApiError::new("failed", e.to_string()))?;
    Ok(done())
}

pub(super) fn clipboard_copy_files(c: &Ctx, p: Items) -> ApiResult {
    let ids = ids_of(c, &p.ids)?;
    let files: Vec<PathBuf> = read(c.host, |db| Ok(ids.iter().filter_map(|i| db.abs_path(*i)).filter(|p| p.exists()).collect()))?;
    if files.is_empty() {
        return Err(said("not_found", "errors.files.noneReachable", Value::Null, "None of these items' files can be reached"));
    }
    arboard::Clipboard::new().and_then(|mut cb| cb.set().file_list(&files)).map_err(|e| ApiError::new("failed", e.to_string()))?;
    Ok(json!({"count": files.len()}))
}

/// A button a viewer page puts in Curio's toolbar.
#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ToolButton {
    /// What `plugin.tool` names when it is clicked.
    id: String,
    label: String,
    /// A Tabler icon's name.
    #[serde(default)]
    icon: Option<String>,
    /// Shown pressed, for a mode that is on.
    #[serde(default)]
    on: bool,
    /// Controls the button opens in a panel below it, rather than telling the page of a click: a list of controls, or tabs as `[{ tab, icon?, controls }]`.
    #[serde(default)]
    panel: Option<Vec<Value>>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ToolSlider {
    /// What `plugin.tool` names, with the `value` set.
    id: String,
    #[serde(default)]
    label: Option<String>,
    value: f64,
    min: f64,
    max: f64,
    #[serde(default)]
    step: Option<f64>,
    /// rtl puts the start on the right.
    #[serde(default)]
    dir: Option<String>,
}

/// One control in a button's panel: section (a heading), slider (min, max, step, unit), toggle, choice (a few options as segments), select (a list) or swatches (options with background and foreground colours).
#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ToolControl {
    #[serde(rename = "type")]
    kind: String,
    /// What `plugin.tool` names, with the `value` set.
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    min: Option<f64>,
    #[serde(default)]
    max: Option<f64>,
    #[serde(default)]
    step: Option<f64>,
    #[serde(default)]
    unit: Option<String>,
    /// { value, label, icon?, background?, foreground? }
    #[serde(default)]
    options: Vec<Value>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ToolLabel {
    id: String,
    text: String,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub(super) struct ViewerTools {
    /// Text labels shown in the toolbar, such as a page number or chapter title.
    #[serde(default)]
    labels: Vec<ToolLabel>,
    #[serde(default)]
    buttons: Vec<ToolButton>,
    /// A slider for the place in what is shown.
    #[serde(default)]
    slider: Option<ToolSlider>,
    /// The ids of the plugin's `viewer_keys` that do something now; all of them when left out.
    #[serde(default)]
    keys: Option<Vec<String>>,
}

/// A field's box in the page, in CSS pixels from the page's top left.
#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct FieldBox {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub(super) struct Choose {
    /// Values, or `{ value, label, icon, hint, heading, disabled }`: `hint` is faint text at the row's end, `heading` a title over the rows after it; up to 1000.
    options: Vec<Value>,
    /// The option ticked as the current one.
    #[serde(default)]
    value: Option<Value>,
    /// A search field at the list's top.
    #[serde(default)]
    search: bool,
    #[serde(default)]
    at: Option<FieldBox>,
}

pub(super) fn ui_choose(_: &Ctx, _: Choose) -> ApiResult {
    Err(ApiError::new("bad_request", "ui.choose is answered by the window showing the page"))
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub(super) struct Popover {
    /// A page in the plugin's folder.
    page: String,
    /// Told to that page as `plugin.run`.
    #[serde(default)]
    data: Option<Value>,
    /// 160 to 640 pixels; 300 when left out.
    #[serde(default)]
    width: Option<f64>,
    #[serde(default)]
    at: Option<FieldBox>,
}

pub(super) fn ui_popover(_: &Ctx, _: Popover) -> ApiResult {
    Err(ApiError::new("bad_request", "ui.popover is answered by the window showing the page"))
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub(super) struct PluginClose {
    /// The answer of the popover the page shows in.
    #[serde(default)]
    result: Option<Value>,
}

pub(super) fn plugin_close(_: &Ctx, _: PluginClose) -> ApiResult {
    Err(ApiError::new("bad_request", "plugin.close is answered by the window showing the page"))
}

pub(super) fn viewer_set_tools(_: &Ctx, _: ViewerTools) -> ApiResult {
    Err(ApiError::new("bad_request", "viewer.setTools is answered by the page showing a viewer plugin"))
}
