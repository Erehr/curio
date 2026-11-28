//! The operations. Parameters are typed structs: their doc comments and serde shape give the schema MCP and the docs show.

mod archives;
mod fields;
mod folders;
pub(crate) mod item_adds;
mod item_edits;
mod item_grouping;
mod item_reads;
mod jobs;
mod libraries;
mod library_data;
mod locations;
mod plugin_data;
mod smart_folders;
mod tags;
mod windows;

use std::collections::HashMap;
use std::time::Duration;

use base64::Engine;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::core::api::{read, with_library, ApiError, ApiResult, CallerKind, Ctx, KnownLibrary, Op, OpKind, Perm, Target};
use crate::core::db::{Db, DbResult};
use crate::core::library::instance;
use crate::core::library::{writer::reply_within, changes::Changed, Ctx as LibCtx};
use crate::core::model::*;
use crate::core::said::Said;
use crate::core::api::operations::windows::clipboard_copy_files;
use crate::core::api::operations::windows::clipboard_write_text;
use crate::core::api::operations::windows::clipboard_read_text;
use crate::core::api::operations::windows::dialog_save_file;
use crate::core::api::operations::windows::dialog_open_folder;
use crate::core::api::operations::windows::dialog_open_file;
use crate::core::api::operations::windows::ui_popover;
use crate::core::api::operations::windows::ui_choose;
use crate::core::api::operations::windows::ui_menu;
use crate::core::api::operations::windows::ui_pick_color;
use crate::core::api::operations::windows::ui_pick_field;
use crate::core::api::operations::windows::ui_pick_folders;
use crate::core::api::operations::windows::ui_pick_tags;
use crate::core::api::operations::windows::ui_prompt;
use crate::core::api::operations::windows::ui_confirm;
use crate::core::api::operations::windows::ui_notify;
use crate::core::api::operations::windows::app_open_url;
use crate::core::api::operations::windows::item_reveal;
use crate::core::api::operations::windows::item_open;
use crate::core::api::operations::windows::viewer_current;
use crate::core::api::operations::windows::viewer_close;
use crate::core::api::operations::windows::viewer_open;
use crate::core::api::operations::windows::view_show_with;
use crate::core::api::operations::windows::view_set_filters;
use crate::core::api::operations::windows::view_set_sample;
use crate::core::api::operations::windows::view_set_search;
use crate::core::api::operations::windows::view_show;
use crate::core::api::operations::windows::view_get;
use crate::core::api::operations::windows::selection_set;
use crate::core::api::operations::windows::selection_get;
use crate::core::api::operations::windows::library_switch;
use crate::core::api::operations::windows::app_theme;
use crate::core::api::operations::windows::plugin_close;
use crate::core::api::operations::windows::viewer_set_tools;
use crate::core::api::operations::item_reads::query_facets;
use crate::core::api::operations::item_reads::query_validate;
use crate::core::api::operations::locations::location_relocate;
use crate::core::api::operations::locations::location_set_look;
use crate::core::api::operations::locations::location_set_access;
use crate::core::api::operations::locations::location_rescan;
use crate::core::api::operations::locations::location_remove;
use crate::core::api::operations::locations::location_add;
use crate::core::api::operations::locations::location_list;
use crate::core::api::operations::fields::field_rescan;
use crate::core::api::operations::fields::field_cleanup;
use crate::core::api::operations::fields::field_used_by;
use crate::core::api::operations::fields::field_failures;
use crate::core::api::operations::fields::field_coverage;
use crate::core::api::operations::fields::field_facets;
use crate::core::api::operations::fields::field_delete;
use crate::core::api::operations::fields::field_define;
use crate::core::api::operations::fields::field_get;
use crate::core::api::operations::fields::field_list;
use crate::core::api::operations::tags::tag_group_remove_tags;
use crate::core::api::operations::tags::tag_group_add_tags;
use crate::core::api::operations::tags::tag_group_delete;
use crate::core::api::operations::tags::tag_group_update;
use crate::core::api::operations::tags::tag_group_create;
use crate::core::api::operations::tags::tag_group_list;
use crate::core::api::operations::tags::tag_set_color;
use crate::core::api::operations::tags::tag_delete;
use crate::core::api::operations::tags::tag_merge;
use crate::core::api::operations::tags::tag_rename;
use crate::core::api::operations::tags::tag_get;
use crate::core::api::operations::tags::tag_list;
use crate::core::api::operations::smart_folders::smart_items;
use crate::core::api::operations::smart_folders::smart_delete;
use crate::core::api::operations::smart_folders::smart_update;
use crate::core::api::operations::smart_folders::smart_create;
use crate::core::api::operations::smart_folders::smart_get;
use crate::core::api::operations::smart_folders::smart_list;
use crate::core::api::operations::folders::folder_delete;
use crate::core::api::operations::folders::folder_update;
use crate::core::api::operations::folders::folder_move;
use crate::core::api::operations::folders::folder_rename;
use crate::core::api::operations::folders::folder_create;
use crate::core::api::operations::folders::folder_items;
use crate::core::api::operations::folders::folder_recent;
use crate::core::api::operations::folders::folder_get;
use crate::core::api::operations::folders::folder_list;
use crate::core::api::operations::jobs::tool_list;
use crate::core::api::operations::library_data::app_fonts;
use crate::core::api::operations::library_data::library_stats;
use crate::core::api::operations::library_data::duplicates_unignore;
use crate::core::api::operations::library_data::duplicates_ignore;
use crate::core::api::operations::library_data::duplicates_resolve;
use crate::core::api::operations::library_data::duplicates_find;
use crate::core::api::operations::library_data::item_similar;
use crate::core::api::operations::archives::archive_create;
use crate::core::api::operations::archives::archive_entry_url;
use crate::core::api::operations::archives::archive_entry;
use crate::core::api::operations::archives::archive_list;
use crate::core::api::operations::jobs::process_run;
use crate::core::api::operations::jobs::job_get;
use crate::core::api::operations::jobs::job_list;
use crate::core::api::operations::jobs::job_cancel;
use crate::core::api::operations::jobs::job_finish;
use crate::core::api::operations::jobs::job_update;
use crate::core::api::operations::jobs::job_start;
use crate::core::api::operations::plugin_data::plugin_log;
use crate::core::api::operations::plugin_data::plugin_settings_set;
use crate::core::api::operations::plugin_data::plugin_settings_get;
use crate::core::api::operations::plugin_data::plugin_item_state_set;
use crate::core::api::operations::plugin_data::plugin_item_state_get_many;
use crate::core::api::operations::plugin_data::plugin_item_state_get;
use crate::core::api::operations::plugin_data::plugin_state_set;
use crate::core::api::operations::plugin_data::plugin_state_get;
use crate::core::api::operations::item_adds::item_write_url;
use crate::core::api::operations::item_adds::item_add_beside;
use crate::core::api::operations::item_adds::item_add_bookmark;
use crate::core::api::operations::item_adds::item_add_from_bytes;
use crate::core::api::operations::item_adds::item_add_from_urls;
use crate::core::api::operations::item_adds::item_add_from_paths;
use crate::core::api::operations::item_edits::item_file_url;
use crate::core::api::operations::item_edits::item_preview_url;
use crate::core::api::operations::item_edits::item_file;
use crate::core::api::operations::item_edits::item_thumbnail;
use crate::core::api::operations::item_edits::item_export;
use crate::core::api::operations::item_edits::item_move_to_library;
use crate::core::api::operations::item_edits::item_copy_to_library;
use crate::core::api::operations::item_edits::item_duplicate;
use crate::core::api::operations::item_edits::item_refresh_palette;
use crate::core::api::operations::item_edits::item_refresh_thumbnail;
use crate::core::api::operations::item_edits::item_set_thumbnail;
use crate::core::api::operations::item_edits::item_replace_file;
use crate::core::api::operations::item_edits::item_rotate_file;
use crate::core::api::operations::item_edits::item_rotate;
use crate::core::api::operations::item_edits::item_purge;
use crate::core::api::operations::item_edits::item_restore;
use crate::core::api::operations::item_edits::item_trash;
use crate::core::api::operations::item_grouping::library_grouping_split;
use crate::core::api::operations::item_grouping::library_grouping_apply;
use crate::core::api::operations::item_grouping::library_grouping_survey;
use crate::core::api::operations::item_grouping::library_grouping;
use crate::core::api::operations::item_grouping::item_allow_grouping;
use crate::core::api::operations::item_grouping::item_forget_file;
use crate::core::api::operations::item_grouping::item_detach;
use crate::core::api::operations::item_grouping::item_split;
use crate::core::api::operations::item_grouping::item_group;
use crate::core::api::operations::item_edits::item_move_to_folder;
use crate::core::api::operations::item_edits::item_remove_from_folders;
use crate::core::api::operations::item_edits::item_add_to_folders;
use crate::core::api::operations::item_edits::item_remove_tags;
use crate::core::api::operations::item_edits::item_add_tags;
use crate::core::api::operations::item_edits::item_rename;
use crate::core::api::operations::item_edits::item_update;
use crate::core::api::operations::item_reads::item_get_many;
use crate::core::api::operations::item_reads::item_exif;
use crate::core::api::operations::item_reads::item_png_text;
use crate::core::api::operations::fields::item_read_fields;
use crate::core::api::operations::item_reads::item_get;
use crate::core::api::operations::item_reads::item_count;
use crate::core::api::operations::item_reads::item_query;
use crate::core::api::operations::libraries::library_set_look;
use crate::core::api::operations::libraries::library_place;
use crate::core::api::operations::libraries::library_forget;
use crate::core::api::operations::libraries::library_rename;
use crate::core::api::operations::libraries::library_create;
use crate::core::api::operations::libraries::library_open;
use crate::core::api::operations::libraries::library_info;
use crate::core::api::operations::libraries::library_list;
use crate::core::api::operations::windows::token_forget;
use crate::core::api::operations::libraries::app_operations;
use crate::core::api::operations::libraries::app_info;

const WRITE_WAIT: Duration = Duration::from_secs(10);
const URL_WAIT: Duration = Duration::from_secs(60);
/// Copying, turning or replacing large files.
const FILE_WAIT: Duration = Duration::from_secs(120);
const MAX_LIMIT: usize = 1000;

fn schema<P: JsonSchema>() -> Value {
    let gen = schemars::gen::SchemaSettings::draft07()
        .with(|s| {
            s.inline_subschemas = true;
            s.option_add_null_type = false;
        })
        .into_generator();
    let mut v = serde_json::to_value(gen.into_root_schema_for::<P>()).unwrap_or_default();
    if let Some(o) = v.as_object_mut() {
        o.remove("$schema");
        o.remove("title");
        o.remove("definitions");
        o.insert("type".into(), json!("object"));
        o.entry("properties").or_insert_with(|| json!({}));
    }
    v
}

fn op<P: DeserializeOwned + JsonSchema + 'static>(name: &'static str, perm: Perm, kind: OpKind, doc: &'static str, f: fn(&Ctx, P) -> ApiResult) -> Op {
    let params = schema::<P>();
    let allowed: std::collections::HashSet<String> = params.get("properties").and_then(|p| p.as_object()).map(|m| m.keys().cloned().collect()).unwrap_or_default();
    Op {
        name,
        doc,
        perm,
        kind,
        target: Target::Library,
        callers: CallerKind::ALL,
        mcp: true,
        bytes: false,
        pages_only: false,
        params,
        returns: "",
        run: Box::new(move |c, v| {
            if let Some(obj) = v.as_object() {
                if let Some(k) = obj.keys().find(|k| !allowed.contains(*k)) {
                    return Err(ApiError::new("bad_request", format!("{name}: unknown parameter `{k}`")));
                }
            }
            let p: P = serde_json::from_value(v).map_err(|e| ApiError::new("bad_request", format!("{name}: {e}")))?;
            f(c, p)
        }),
    }
}

impl Op {
    fn app(mut self) -> Op {
        self.target = Target::App;
        self
    }

    fn entry(mut self) -> Op {
        self.target = Target::Entry;
        self
    }

    fn window(mut self) -> Op {
        self.target = Target::Window;
        self
    }

    /// The answer's TypeScript type, for `curio.d.ts` and the operation reference.
    fn returns(mut self, ts: &'static str) -> Op {
        self.returns = ts;
        self
    }

    /// For plugin pages only: the page showing the plugin answers it, or it needs the page's own address.
    fn pages(mut self) -> Op {
        self.pages_only = true;
        self.plugins()
    }

    /// For plugins only, their pages and programs: what it keeps is the calling plugin's own.
    fn plugins(mut self) -> Op {
        self.callers = &[CallerKind::Plugin];
        self.mcp = false;
        self
    }

    fn callers(mut self, callers: &'static [CallerKind]) -> Op {
        self.callers = callers;
        self.mcp = callers.contains(&CallerKind::Mcp);
        self
    }

    fn bytes(mut self, for_mcp: bool) -> Op {
        self.bytes = true;
        self.mcp = for_mcp;
        self
    }
}

pub(super) fn all() -> Vec<Op> {
    use OpKind::{Read as R, Window as W, Write as Wr};
    use Perm::*;
    vec![
        op("app.info", LibraryRead, R, "Curio's version, the API version, the display language, and the system it runs on: `platform` as `windows`, `linux` or `macos`, `arch` as `x86_64` or `aarch64`", app_info).app().returns("{ name: string; version: string; apiVersion: number; platform: 'windows' | 'linux' | 'macos'; arch: string; language: string }"),
        op("app.operations", LibraryRead, R, "The operations this caller may use, with their parameters", app_operations).app().returns("Operation[]"),
        op("token.forget", LibraryRead, Wr, "Delete the token this call was made with, as a client does when it disconnects; it works for nothing after this answer", token_forget).app().callers(&[CallerKind::Web]).returns("Done"),
        op("library.list", LibraryRead, R, "The libraries on this computer's list, with their ids and whether a window has them open", library_list).app().returns("Library[]"),
        op("library.info", LibraryRead, R, "A library's id, name and item counts", library_info).returns("LibraryInfo"),
        op("library.open", LibraryManage, Wr, "Open a library in a window of its own; a window that already has it comes forward", library_open).entry().returns("Done"),
        op("library.create", LibraryManage, Wr, "Make a new library and add it to the list", library_create).app().returns("{ success: true; id: string; name: string; path?: string }"),
        op("library.rename", LibraryManage, Wr, "Rename a library's folder, which is its name; only while no window has it open", library_rename).entry().returns("{ success: true; name: string; path?: string }"),
        op("library.forget", LibraryManage, Wr, "Take a library off the list; its folder stays as it is", library_forget).entry().returns("Done"),
        op("library.place", LibraryManage, Wr, "Move a library in the list", library_place).entry().returns("Done"),
        op("library.setLook", LibraryManage, Wr, "Set a library's icon or colour in the lists", library_set_look).entry().returns("Done"),
        op("item.query", MetadataRead, R, "Find items with the query language, e.g. `tag = sky AND rating >= 4` or `prompt CONTAINS \"blue hair\"`", item_query).returns("Page<Item>"),
        op("item.count", MetadataRead, R, "Count the items a query finds", item_count).returns("{ count: number }"),
        op("item.get", MetadataRead, R, "One item with its custom fields", item_get).returns("ItemWithFields"),
        op("item.readFields", FilesRead, R, "Reads an item's file for fields read from files without storing what it finds: by default the fields switched off, which hold no values. Curio's readers and plugins' declared ones run; plugins' programs don't", item_read_fields).returns("{ fields: Record<string, unknown> }"),
        op("item.getMany", MetadataRead, R, "Several items with their custom fields", item_get_many).returns("ItemWithFields[]"),
        op("item.update", MetadataWrite, Wr, "Change items' names, notes, source addresses, ratings or custom fields", item_update).returns("Done"),
        op("item.rename", MetadataWrite, Wr, "Give several items names of their own in one change", item_rename).returns("Done"),
        op("item.addTags", MetadataWrite, Wr, "Add tags to items; new tag names are created", item_add_tags).returns("Done"),
        op("item.removeTags", MetadataWrite, Wr, "Remove tags from items, or all their tags", item_remove_tags).returns("Done"),
        op("item.addToFolders", MetadataWrite, Wr, "Add items to folders", item_add_to_folders).returns("Done"),
        op("item.removeFromFolders", MetadataWrite, Wr, "Take items out of folders; they stay in the library", item_remove_from_folders).returns("Done"),
        op("item.moveToFolder", MetadataWrite, Wr, "Put items in a folder and take them out of another, or out of every other folder", item_move_to_folder).returns("Done"),
        op("item.group", MetadataWrite, Wr, "Group files into one item: `members` become files of `id`, which takes their tags, folders and notes; splitting gives them back", item_group).returns("Done"),
        op("item.split", MetadataWrite, Wr, "Split items into their files, each an item again with what it had; rules don't group them again", item_split).returns("{ ids: string[] }"),
        op("item.detach", MetadataWrite, Wr, "Take one file out of the item it is grouped into, an item again", item_detach).returns("Done"),
        op("item.forgetFile", FilesWrite, Wr, "Forget a missing or offline file grouped into an item: it leaves the item and the library, and a scan doesn't bring it back unchanged", item_forget_file).returns("Done"),
        op("item.allowGrouping", MetadataWrite, Wr, "Let rules group items split by hand again", item_allow_grouping).returns("Done"),
        op("library.grouping", MetadataRead, R, "The rules grouping files into one item, in the order they are tried, each with how many files it grouped", library_grouping).returns("{ rules: { id: string; owner: string; role: string; grouped: number; exts?: string[]; files?: string[] }[] }"),
        op("library.grouping.survey", MetadataRead, R, "How many files already in the library each rule would group; nothing is grouped", library_grouping_survey).returns("{ rules: { id: string; count: number }[] }"),
        op("library.grouping.apply", MetadataWrite, Wr, "Group the files already in the library as the rules say, or only these `rules`; when that takes over two minutes the answer is `queued` and the work carries on", library_grouping_apply).returns("{ count: number } | { queued: true }"),
        op("library.grouping.split", MetadataWrite, Wr, "Split off every file a rule grouped, each an item again with what it had; what other rules grouped stays, and rules may group these files again; when that takes over two minutes the answer is `queued` and the work carries on", library_grouping_split).returns("{ count: number } | { queued: true }"),
        op("item.trash", MetadataWrite, Wr, "Move items to the trash; files on disk stay until the trash is emptied", item_trash).returns("Done"),
        op("item.restore", MetadataWrite, Wr, "Bring items back from the trash, into the folders they were in", item_restore).returns("Done"),
        op("item.purge", FilesWrite, Wr, "Delete items in the trash for good, or empty the trash; copies the library made go to the Recycle Bin", item_purge).returns("Done"),
        op("item.rotate", MetadataWrite, Wr, "Turn pictures as they are shown; the files stay as they are", item_rotate).returns("Done"),
        op("item.rotateFile", FilesWrite, Wr, "Turn a picture's file itself by quarter turns clockwise (JPG, PNG, BMP, still WebP)", item_rotate_file).returns("Done"),
        op("item.replaceFile", FilesWrite, Wr, "Put another file of the same type in an item's place; the old file goes to the Recycle Bin. A `path` needs files.outside, unless a run of the plugin's own made it", item_replace_file).returns("Done"),
        op("item.setThumbnail", FilesWrite, Wr, "Give items a thumbnail from a picture file or picture bytes. A `path` needs files.outside, unless a run of the plugin's own made it", item_set_thumbnail).returns("Done"),
        op("item.refreshThumbnail", MetadataWrite, Wr, "Make items' thumbnails again", item_refresh_thumbnail).returns("Done"),
        op("item.refreshPalette", MetadataWrite, Wr, "Read items' colours again from their thumbnails", item_refresh_palette).returns("{ success: true; queued?: true }"),
        op("item.duplicate", FilesWrite, Wr, "New items with their own copies of the files, the same metadata and folders", item_duplicate).returns("{ success: true; ids: string[] }"),
        op("item.copyToLibrary", FilesWrite, Wr, "Copy items to another library, open or not, with their tags, rating, notes and matching fields", item_copy_to_library).returns("{ success: true; count: number }"),
        op("item.moveToLibrary", FilesWrite, Wr, "Copy items to another library, then move them to this library's trash", item_move_to_library).returns("{ success: true; count: number }"),
        op("item.export", FilesOutside, Wr, "Copy items' files into a folder on this computer", item_export).returns("{ success: true; count: number }"),
        op("item.thumbnail", FilesRead, R, "An item's thumbnail; over HTTP the answer is the image itself", item_thumbnail).bytes(true).returns("Blob"),
        op("item.file", FilesRead, R, "An item's file; over HTTP the answer is the file itself", item_file).bytes(false).returns("Blob"),
        op("item.exif", FilesRead, R, "The EXIF tags of an item's picture, by name: text as text, other values as EXIF shows them; empty without EXIF", item_exif).returns("{ tags: Record<string, string> }"),
        op("item.pngText", FilesRead, R, "The text chunks of an item's PNG file (tEXt, zTXt, iTXt), by keyword; empty for other files", item_png_text).returns("{ chunks: Record<string, string> }"),
        op("item.previewUrl", FilesRead, R, "An address a plugin's page shows an item's picture from at `size` pixels on its long edge, made once by Curio and kept, as its own viewer does when zooming: JPEGs are read at the reduced size, so a large page never decodes in full. Works for every format Curio can picture, camera raw and PSD included", item_preview_url).pages().returns("{ url: string }"),
        op("item.fileUrl", FilesRead, R, "An address a plugin's page reads an item's file from, in parts as it needs (for `<video>`, pdf.js or a reader); for the formats the plugin declares, files beside it resolve too, as an OBJ's materials or a glTF's textures", item_file_url).pages().returns("{ url: string }"),
        op("item.add.fromPaths", FilesWrite, Wr, "Add files or folders from this computer, copied into the library, a folder as a library folder with its subfolders; without `copy` a folder becomes a location instead, and a file already in a location is that location's item. Needs files.outside too, unless a run of the plugin's own made the files", item_add_from_paths).returns("{ success: true; queued: true } | { success: true; ids: string[] }"),
        op("item.add.fromUrls", FilesWrite, Wr, "Download images and files into the library; web pages and videos are saved as links. Curio downloads only from the internet, not from this computer or its network", item_add_from_urls).returns("Added"),
        op("item.add.fromBytes", FilesWrite, Wr, "Add a file sent in the call, as base64", item_add_from_bytes).returns("Added"),
        op("item.add.bookmark", FilesWrite, Wr, "Save a web address as a link, whatever it points to", item_add_bookmark).returns("Added"),
        op("item.add.beside", FilesWrite, Wr, "Add a file as a new item next to an item, in its folders and with its tags, rating, notes, source address and fields: a converted or edited copy. A `path` needs files.outside, unless a run of the plugin's own made it", item_add_beside).returns("Added"),
        op("item.writeUrl", FilesWrite, Wr, "An address a plugin's page sends a file to with PUT, once: `replace` puts it in the item's place (same type; the old file goes to the Recycle Bin), `beside` makes it a new item next to the item, as item.add.beside. The PUT answers as those calls do", item_write_url).pages().returns("{ url: string }"),
        op("plugin.state.get", LibraryRead, R, "A value the plugin keeps for this library under a key; null when none is", plugin_state_get).plugins().returns("{ value: unknown }"),
        op("plugin.state.set", LibraryRead, Wr, "Keep any JSON value for this library under a key, up to 1 MB; null removes it", plugin_state_set).plugins().returns("Done"),
        op("plugin.itemState.get", LibraryRead, R, "What the plugin keeps for one item, such as a reading position; null when nothing is", plugin_item_state_get).plugins().returns("{ value: unknown }"),
        op("plugin.itemState.getMany", LibraryRead, R, "What the plugin keeps for several items, by id; items with nothing kept are left out", plugin_item_state_get_many).plugins().returns("Record<string, unknown>"),
        op("plugin.itemState.set", LibraryRead, Wr, "Keep a value for one item, up to 64 KB, removed with the item; null removes it. Unlike a field nobody sees or searches it, and no items.changed is sent, so saving a place every few seconds wakes no window", plugin_item_state_set).plugins().returns("Done"),
        op("plugin.settings.get", LibraryRead, R, "The plugin's settings on this computer, the same in every library", plugin_settings_get).app().plugins().returns("Record<string, unknown>"),
        op("plugin.settings.set", LibraryRead, Wr, "Change the plugin's settings on this computer: each key given is set, null removes it. The plugin's pages in the same window hear plugin.settingsChanged", plugin_settings_set).app().plugins().returns("Record<string, unknown>"),
        op("plugin.log", LibraryRead, Wr, "Write a line to Curio's log, marked with the plugin's id", plugin_log).app().plugins().returns("Done"),
        op("job.start", LibraryRead, Wr, "Start a job: long work the plugin or one of Curio's tools does itself, shown in Curio's status area with its progress and a way to cancel it, so a window can be closed while it runs. Report with job.update and end it with job.finish", job_start).app().callers(&[CallerKind::Window, CallerKind::Plugin]).returns("{ job: string }"),
        op("job.update", LibraryRead, Wr, "Tell how far a job has got; the answer says whether it was cancelled meanwhile, so the work can stop", job_update).app().callers(&[CallerKind::Window, CallerKind::Plugin]).returns("{ cancelled: boolean }"),
        op("job.finish", LibraryRead, Wr, "End a job with its result, or as failed with `error`", job_finish).app().callers(&[CallerKind::Window, CallerKind::Plugin]).returns("Done"),
        op("job.cancel", LibraryRead, Wr, "Cancel a running job: a program process.run started is stopped with everything it started; a plugin doing the work itself hears job.finished", job_cancel).app().callers(&[CallerKind::Window, CallerKind::Plugin]).returns("Done"),
        op("job.list", LibraryRead, R, "Running jobs and those that ended in the last ten minutes; a plugin gets its own", job_list).app().returns("Job[]"),
        op("job.get", LibraryRead, R, "One job: its progress, and once it ends its result or error", job_get).app().returns("Job"),
        op("process.run", Exec, Wr, "Run one of the plugin's programs, or a tool it lists in `tools`, as a job: its output comes as job.output events, cancelling stops it and everything it started, and it works in a folder of its own in Curio's temporary space. Answers at once with the job, or with `wait` when it ends", process_run).app().plugins().returns("ProcessRun"),
        op("archive.list", FilesRead, R, "What a ZIP, CBZ, EPUB, 7z or CB7 item holds, in reading order (folder by folder, `page2` before `page10`), each entry with its size and whether it is a picture; pictures with the size they show at and whether they move, read from their first bytes (in a 7z, once it has been unpacked for reading), unless `sizes` is false", archive_list).returns("{ entries: { name: string; size: number; picture: boolean; width?: number; height?: number; animated?: boolean }[] }"),
        op("archive.entry", FilesRead, R, "One entry of an archive item; over HTTP the answer is the entry itself", archive_entry).bytes(false).returns("Blob"),
        op("archive.entryUrl", FilesRead, R, "An address a plugin's page shows an archive's entry from, as `<img>` takes it; with `size` a picture comes scaled to that long edge, as item.previewUrl does", archive_entry_url).pages().returns("{ url: string }"),
        op("archive.create", FilesWrite, Wr, "Pack items' files into a new CBZ (or ZIP) item; 7z is read, not written, in the order given: pictures become a comic's pages", archive_create).returns("Added"),
        op("item.similar", MetadataRead, R, "Pictures that look like an item, most alike first, from the fingerprint Curio keeps of every thumbnail", item_similar).returns("{ id: string; similarity: number }[]"),
        op("duplicates.find", MetadataRead, Wr, "Look for duplicates as a job: identical files, and with other `ways` pictures, videos, sounds and archives that look or sound alike; the hashes and fingerprints it works out are kept for the next search. The job's result is the groups, each member matching every other one, and what couldn't be compared; job.finished tells when it is there. A search already running in the library stops", duplicates_find).returns("{ job: string }"),
        op("duplicates.resolve", MetadataWrite, Wr, "Keep one item of each group and move the others to the trash, the kept one first taking their tags, folders, best rating, notes, source address and user fields it hasn't any of. Each group is checked first: one whose kept item is gone, or a copy said to be identical that changed, is left as it is", duplicates_resolve).returns("{ removed: number; kept: number; skipped: { id: string; reason: string }[] }"),
        op("duplicates.ignore", MetadataWrite, Wr, "Mark items as not duplicates of each other: no search groups any two of them again", duplicates_ignore).returns("{ pairs: number }"),
        op("duplicates.unignore", MetadataWrite, Wr, "Take back marks of not being duplicates: those of some items, or all of them", duplicates_unignore).returns("{ pairs: number }"),
        op("library.stats", MetadataRead, R, "How many items and bytes there are by kind, extension, folder (with its subfolders), tag or month added, among the items a query finds", library_stats).returns("{ by: string; total: { count: number; bytes: number }; groups: { key: string; name: string; count: number; bytes: number }[] }"),
        op("app.fonts", LibraryRead, R, "The font families installed on this computer, for a reader's choice of font", app_fonts).app().returns("string[]"),
        op("tool.list", LibraryRead, R, "The programs Curio finds for plugins (FFmpeg, yt-dlp …): whether each is found, its version, and where to get it", tool_list).app().returns("Tool[]"),
        op("folder.list", MetadataRead, R, "Every folder as a flat list with parent ids and item counts", folder_list).returns("Folder[]"),
        op("folder.get", MetadataRead, R, "One folder with the ids of its subfolders", folder_get).returns("Folder & { children: number[] }"),
        op("folder.recent", MetadataRead, R, "The folders used last in this library's window, newest first", folder_recent).returns("Folder[]"),
        op("folder.items", MetadataRead, R, "The items in a folder, its subfolders' too unless asked otherwise", folder_items).returns("Page<Item>"),
        op("folder.create", Organize, Wr, "Make a folder", folder_create).returns("{ success: true; id: number }"),
        op("folder.rename", Organize, Wr, "Rename a folder", folder_rename).returns("Done"),
        op("folder.move", Organize, Wr, "Move a folder under another, or to the top level", folder_move).returns("Done"),
        op("folder.update", Organize, Wr, "Set a folder's colour, icon or automatic tags", folder_update).returns("Done"),
        op("folder.delete", Organize, Wr, "Delete a folder and its subfolders; their items stay in the library", folder_delete).returns("Done"),
        op("smartFolder.list", MetadataRead, R, "Every smart folder with its query", smart_list).returns("SmartFolder[]"),
        op("smartFolder.get", MetadataRead, R, "One smart folder", smart_get).returns("SmartFolder"),
        op("smartFolder.create", Organize, Wr, "Make a smart folder from a query; inside another one it narrows that one", smart_create).returns("{ success: true; id: number }"),
        op("smartFolder.update", Organize, Wr, "Rename a smart folder, change its query or icon", smart_update).returns("Done"),
        op("smartFolder.delete", Organize, Wr, "Delete a smart folder", smart_delete).returns("Done"),
        op("smartFolder.items", MetadataRead, R, "The items a smart folder shows", smart_items).returns("Page<Item>"),
        op("tag.list", MetadataRead, R, "Every tag with its item count and groups", tag_list).returns("Tag[]"),
        op("tag.get", MetadataRead, R, "One tag with its item count, colour and groups", tag_get).returns("Tag"),
        op("tag.rename", Organize, Wr, "Rename a tag; renaming onto an existing tag merges them", tag_rename).returns("Done"),
        op("tag.merge", Organize, Wr, "Merge several tags into one", tag_merge).returns("Done"),
        op("tag.delete", Organize, Wr, "Delete a tag from every item", tag_delete).returns("Done"),
        op("tag.setColor", Organize, Wr, "Set or clear a tag's own colour", tag_set_color).returns("Done"),
        op("tagGroup.list", MetadataRead, R, "Every tag group", tag_group_list).returns("TagGroup[]"),
        op("tagGroup.create", Organize, Wr, "Make a tag group", tag_group_create).returns("{ success: true; id: number }"),
        op("tagGroup.update", Organize, Wr, "Rename a tag group or set its colour, icon or description", tag_group_update).returns("Done"),
        op("tagGroup.delete", Organize, Wr, "Delete a tag group; its tags stay", tag_group_delete).returns("Done"),
        op("tagGroup.addTags", Organize, Wr, "Put tags into a group; new tag names are created", tag_group_add_tags).returns("Done"),
        op("tagGroup.removeTags", Organize, Wr, "Take tags out of a group", tag_group_remove_tags).returns("Done"),
        op("field.list", MetadataRead, R, "Custom metadata fields: key, label, type and owner", field_list).returns("Field[]"),
        op("field.get", MetadataRead, R, "One custom field", field_get).returns("Field"),
        op("field.define", Organize, Wr, "Define a custom field, or change one; properties left out keep their values", field_define).returns("Done"),
        op("field.delete", Organize, Wr, "Delete a field defined by the user, with its values", field_delete).returns("Done"),
        op("field.facets", MetadataRead, R, "A field's values with how many items have each, among the items a query finds", field_facets).returns("FieldFacets"),
        op("field.coverage", MetadataRead, R, "How far the fields read from files are read: for each, the items with a value, the files its readers apply to, those waiting to be read and those a reader failed on", field_coverage).returns("{ fields: FieldCoverage[] }"),
        op("field.failures", MetadataRead, R, "The files readers failed on, newest first: for some fields or a plugin's readers, or all", field_failures).returns("{ failures: { id: string; name: string; reader: string; error: string; at: number }[] }"),
        op("field.usedBy", MetadataRead, R, "What names a field: smart folders, and plugins' filters, panels and sorts", field_used_by).returns("{ smartFolders: { id: number; name: string }[]; filters: { plugin: string; label: string }[]; sorts: { plugin: string; label: string }[] }"),
        op("field.cleanup", MetadataWrite, Wr, "Remove the stored values of fields read from files that are switched off, all but those set by hand; with dryRun, only count them. Definitions and what names the fields stay", field_cleanup).returns("{ fields: { key: string; values: number; pluginGone: boolean }[]; removed: number }"),
        op("field.rescan", MetadataWrite, Wr, "Read fields from files again: those of some fields or of a plugin's readers, for some items or every file they apply to; answers the job that follows the reading", field_rescan).returns("{ job: string | null; files: number }"),
        op("location.list", MetadataRead, R, "The folders on this computer the library indexes in place", location_list).returns("Location[]"),
        op("location.add", FilesOutside, Wr, "Index a folder in place: its files join the library where they are. An Eagle library (a `.library` folder) is indexed as its items, with its folders and tags, and the other options don't apply to it", location_add).returns("{ success: true; id: number }"),
        op("location.remove", Organize, Wr, "Stop indexing a folder; its items leave the library, its files stay", location_remove).returns("Done"),
        op("location.rescan", Organize, Wr, "Look through a location again for new, changed and missing files", location_rescan).returns("{ success: true; queued: true }"),
        op("location.setAccess", Organize, Wr, "What Curio may do to a location's files: read_only, full (edit files) or organize (edit and move them)", location_set_access).returns("Done"),
        op("location.setLook", Organize, Wr, "Set a location's icon or colour", location_set_look).returns("Done"),
        op("location.relocate", FilesOutside, Wr, "Point a location at the folder's new place after it was moved", location_relocate).returns("Done"),
        op("query.validate", MetadataRead, R, "Check a query: whether it parses, and how many items it finds", query_validate).returns("{ valid: true; count: number } | { valid: false; error: string }"),
        op("query.facets", MetadataRead, R, "Ratings, shapes and kinds with their counts among the items a query finds", query_facets).returns("Facets"),
        op("viewer.setTools", LibraryRead, W, "A viewer page's texts, slider and buttons in Curio's own toolbar, each drawn by the widget its plugin declares under the same id (`viewer_tools`); a click comes to the page as `plugin.tool` with the button's id, a right-click with `menu: true` as well, and a value set with the slider or a control in a button's panel with `value`. Answered by the page showing the plugin; an empty list clears them", viewer_set_tools).app().pages().returns("Done"),
        op("plugin.close", LibraryRead, W, "Close the plugin's window the page is in, or the popover it shows in (`ui.popover`) with `result` as that popover's answer. Answered by that page's window", plugin_close).app().pages().returns("Done"),
        op("app.theme", LibraryRead, W, "The theme a window shows: its name, whether it is dark, and every colour, spacing and font token as CSS variables take them", app_theme).window().returns("Theme"),
        op("library.switch", LibraryManage, W, "Show another library in a window; if another window has it, that window comes forward instead", library_switch).window().returns("{ switched: boolean; window: string }"),
        op("selection.get", Ui, W, "The items selected in a window", selection_get).window().returns("Item[]"),
        op("selection.set", Ui, W, "Select items in a window; items its grid doesn't show are left out, unless it shows a Tab plugin's page", selection_set).window().returns("{ selected: number; notShown: number }"),
        op("view.get", Ui, W, "What a window shows: the folder, tag or view, its search and filters as a query, and how many items", view_get).window().returns("View"),
        op("view.show", Ui, W, "Show a folder, smart folder, tag, location, or all, uncategorized, untagged, recent, random, trash or missing items in a window; its search and filters are cleared unless its filter lock is on", view_show).window().returns("View"),
        op("view.setSearch", Ui, W, "Search in a window: words, or a query such as `rating >= 4`", view_set_search).window().returns("View"),
        op("view.setSample", Ui, W, "The text a window's font samples are written in (the list's Font sample column, the Fonts viewer); an empty text goes back to their own", view_set_sample).window().returns("View"),
        op("view.setFilters", Ui, W, "Set a window's filters: tags, folders, ratings, kinds, extensions, colour, shapes, how recently items were added and the days they were added, modified or created", view_set_filters).window().returns("View"),
        op("view.showWith", Ui, W, "Show all items in a window through one filter, as a click on a tag or a field's value does in Curio's own widgets: a tag, or a field's value set as that field's filter asks (the value, a range of just it, or the text it contains); the filter chip shows it, to be widened or taken off", view_show_with).window().returns("View"),
        op("viewer.open", Ui, W, "Open an item in a window's viewer; a window that doesn't show it switches to all items. A file grouped into an item opens that item, showing the file when it was grouped in by hand. An item whose file is missing or whose storage is offline is refused (`failed`)", viewer_open).window().returns("Done"),
        op("viewer.close", Ui, W, "Close a window's viewer", viewer_close).window().returns("{ closed: boolean }"),
        op("viewer.current", Ui, W, "The item open in a window's viewer, and `file`, the file it shows: the item's own or one grouped into it", viewer_current).window().returns("{ id: string | null; file: string | null }"),
        op("item.open", Ui, W, "Open an item in its library's window, in a window of its own, or in the app the system opens its type with; programs and scripts open only from Curio's own window, saved links only when they point to a web page", item_open).returns("Done"),
        op("item.reveal", Ui, W, "Show an item's file in the system's file manager", item_reveal).returns("Done"),
        op("app.openUrl", Ui, W, "Open a web address in the system's browser, once the person at the window agrees to it; http, https and mailto only", app_open_url).window().returns("{ opened: boolean }"),
        op("ui.notify", Ui, W, "Show a short message in a window", ui_notify).window().returns("Done"),
        op("ui.confirm", Ui, W, "Ask the person at a window to confirm something", ui_confirm).window().returns("{ ok: boolean }"),
        op("ui.prompt", Ui, W, "Ask the person at a window for a line of text", ui_prompt).window().returns("{ value: string | null }"),
        op("ui.pickTags", Ui, W, "Let the person at a window choose tags", ui_pick_tags).window().returns("{ tags: string[] | null }"),
        op("ui.pickFolders", Ui, W, "Let the person at a window choose folders", ui_pick_folders).window().returns("{ folders: number[] | null }"),
        op("ui.pickField", Ui, W, "Let the person at a window choose a custom field", ui_pick_field).window().returns("{ key: string | null }"),
        op("ui.pickColor", Ui, W, "Let the person at a window choose a colour", ui_pick_color).window().returns("{ color: string | null }"),
        op("ui.menu", Ui, W, "Show a menu at the pointer in a window and answer the entry chosen", ui_menu).window().returns("{ id: string | null }"),
        op("ui.choose", LibraryRead, R, "A list of options under one of the page's fields, as Curio's menus are, drawn by the window so the page's frame doesn't cut it off; `at` is the field's box in the page. Answered by the window showing the page", ui_choose).app().pages().returns("{ chosen: boolean; value?: string | number | boolean | null }"),
        op("ui.popover", LibraryRead, R, "Another page of the plugin in a popover under one of the page's fields, drawn by the window; that page hears `data` as `plugin.run` and closes it with `plugin.close({ result })`. Answered by the window showing the page", ui_popover).app().pages().returns("{ result: unknown }"),
        op("dialog.openFile", Ui, W, "Let the person at a window choose files with the system's dialog", dialog_open_file).window().returns("{ paths: string[] | null }"),
        op("dialog.openFolder", Ui, W, "Let the person at a window choose a folder with the system's dialog", dialog_open_folder).window().returns("{ path: string | null }"),
        op("dialog.saveFile", Ui, W, "Let the person at a window choose where to save a file with the system's dialog; nothing is written", dialog_save_file).window().returns("{ path: string | null }"),
        op("clipboard.readText", Ui, R, "The text on the clipboard", clipboard_read_text).app().returns("{ text: string | null }"),
        op("clipboard.writeText", Ui, Wr, "Put text on the clipboard", clipboard_write_text).app().returns("Done"),
        op("clipboard.copyFiles", Ui, Wr, "Put items' files on the clipboard, to paste in a file manager", clipboard_copy_files).returns("{ count: number }"),
    ]
}

// ------------------------------------------------------------------ helpers

fn write_for<T: Send + 'static>(c: &Ctx, f: impl FnOnce(&mut Db) -> Result<(T, Changed), String> + Send + 'static) -> Result<T, String> {
    let rx = with_library(c.host, |lib| Ok(lib.write_reply(move |db, _| f(db))))?;
    reply_within(&rx, WRITE_WAIT)
}

fn write(c: &Ctx, f: impl FnOnce(&mut Db) -> Result<Changed, String> + Send + 'static) -> ApiResult {
    write_for(c, move |db| f(db).map(|ch| ((), ch)))?;
    Ok(done())
}

/// File work on the scanner thread, waited for.
fn job_for<T: Send + 'static>(c: &Ctx, wait: Duration, f: impl FnOnce(&mut Db, &LibCtx) -> Result<(T, Changed), String> + Send + 'static) -> Result<T, ApiError> {
    Ok(reply_within(&start_job(c, f)?, wait)?)
}

/// As `job_for`, but work still running after `wait` answers None and carries on.
fn job_or_queued<T: Send + 'static>(c: &Ctx, wait: Duration, f: impl FnOnce(&mut Db, &LibCtx) -> Result<(T, Changed), String> + Send + 'static) -> Result<Option<T>, ApiError> {
    match start_job(c, f)?.recv_timeout(wait) {
        Ok(r) => Ok(Some(r?)),
        Err(crossbeam_channel::RecvTimeoutError::Timeout) => Ok(None),
        Err(_) => Err(String::from("The library closed before the work was done").into()),
    }
}

fn start_job<T: Send + 'static>(c: &Ctx, f: impl FnOnce(&mut Db, &LibCtx) -> Result<(T, Changed), String> + Send + 'static) -> Result<crossbeam_channel::Receiver<Result<T, String>>, ApiError> {
    let (tx, rx) = crossbeam_channel::bounded(1);
    with_library(c.host, |lib| {
        lib.job(move |db, ctx| {
            let r = f(db, ctx);
            let changed = r.as_ref().map(|(_, ch)| ch.clone()).unwrap_or_default();
            let _ = tx.send(r.map(|(t, _)| t));
            Ok(changed)
        });
        lib.kick_thumbs();
        Ok(())
    })?;
    Ok(rx)
}

fn done() -> Value {
    json!({"success": true})
}

fn bad(m: impl Into<String>) -> ApiError {
    ApiError::new("bad_request", m)
}

fn not_found(m: impl Into<String>) -> ApiError {
    ApiError::new("not_found", m)
}

/// An error the window shows in its own language by `id`.
fn said(code: &str, id: &str, args: Value, text: impl Into<String>) -> ApiError {
    ApiError::said(code, Said::new(id, args, text))
}

fn file_missing() -> ApiError {
    said("not_found", "errors.files.missing", Value::Null, "This item's file is missing or offline")
}

fn no_file(path: &dyn std::fmt::Display) -> ApiError {
    said("bad_request", "errors.files.none", json!({"path": path.to_string()}), format!("There is no file at {path}"))
}

fn unreachable_library(l: &KnownLibrary) -> ApiError {
    let name = instance::library_name(&l.path);
    said("library_offline", "errors.library.unreachable", json!({"name": name, "path": l.path.display().to_string()}), format!("{name} can't be reached at {}", l.path.display()))
}

/// Runs a read that may also find the request at fault.
fn read_checked<T>(c: &Ctx, f: impl FnOnce(&Db) -> Result<Result<T, ApiError>, String>) -> Result<T, ApiError> {
    read(c.host, f)?
}

/// An item's stable id, or its row number inside this library.
#[derive(Deserialize, JsonSchema, Clone)]
#[serde(untagged)]
enum ItemRef {
    Uid(String),
    Row(i64),
}

fn item_ids(db: &Db, refs: &[ItemRef]) -> Result<Vec<AssetId>, ApiError> {
    let mut out = Vec::with_capacity(refs.len());
    for r in refs {
        let id = match r {
            ItemRef::Row(n) => db.conn.query_row("SELECT id FROM assets WHERE id = ?", [n], |r| r.get(0)).ok(),
            ItemRef::Uid(s) => match s.parse::<i64>() {
                Ok(n) => db.conn.query_row("SELECT id FROM assets WHERE id = ?", [n], |r| r.get(0)).ok(),
                Err(_) => db.conn.query_row("SELECT id FROM assets WHERE uid = uid_bytes(?)", [s], |r| r.get(0)).ok(),
            },
        };
        match id {
            Some(i) => out.push(i),
            None => {
                let shown = match r {
                    ItemRef::Uid(s) => s.clone(),
                    ItemRef::Row(n) => n.to_string(),
                };
                return Err(not_found(format!("No item has the id {shown}")));
            }
        }
    }
    Ok(out)
}

fn ids_of(c: &Ctx, refs: &[ItemRef]) -> Result<Vec<AssetId>, ApiError> {
    read_checked(c, |db| Ok(item_ids(db, refs)))
}

/// As `ids_of`, a file grouped into an item standing for its item: for writes, which go to the item.
fn items_of(c: &Ctx, refs: &[ItemRef]) -> Result<Vec<AssetId>, ApiError> {
    read_checked(c, |db| Ok(match item_ids(db, refs) {
        Ok(ids) => Ok(db.items_of(&ids)?),
        Err(e) => Err(e),
    }))
}

fn one_id(c: &Ctx, r: &ItemRef) -> Result<AssetId, ApiError> {
    Ok(ids_of(c, std::slice::from_ref(r))?[0])
}

/// `id` for one item or `ids` for several.
fn either(id: Option<ItemRef>, ids: Option<Vec<ItemRef>>) -> Result<Vec<ItemRef>, ApiError> {
    match (id, ids) {
        (Some(one), None) => Ok(vec![one]),
        (None, Some(many)) if !many.is_empty() => Ok(many),
        (Some(one), Some(mut many)) => {
            many.insert(0, one);
            Ok(many)
        }
        _ => Err(bad("Give `id`, or `ids` for several items")),
    }
}

fn uids(db: &Db, ids: &[AssetId]) -> DbResult<Vec<String>> {
    let by: HashMap<AssetId, String> = db.assets(ids)?.into_iter().map(|a| (a.id, a.uid)).collect();
    Ok(ids.iter().filter_map(|i| by.get(i).cloned()).collect())
}

fn item_json(c: &Ctx, db: &Db, a: &Asset) -> Value {
    let tags: Vec<String> = db.asset_tags(a.id).unwrap_or_default().into_iter().map(|t| t.name).collect();
    let folders: Vec<Value> = db.asset_folders(a.id).unwrap_or_default().into_iter().map(|f| json!({"id": f.id, "name": f.name})).collect();
    let mut v = json!({
        "id": a.uid, "name": a.title(), "fileName": a.name, "ext": a.ext, "kind": a.kind.as_str(), "size": a.size,
        "width": a.width, "height": a.height, "duration": a.duration, "rating": a.rating, "favorite": a.favorite, "explicit": a.explicit, "annotation": a.annotation,
        "url": a.source_url, "tags": tags, "folders": folders, "status": a.status.label(),
        "imported": a.added_at, "modified": a.mtime, "parts": a.parts,
    });
    if let Some(item) = a.part_of {
        v["partOf"] = json!(uids(db, &[item]).ok().and_then(|u| u.into_iter().next()));
        v["partRole"] = json!(a.part_role);
    }
    if c.caller.may(Perm::FilesRead) {
        v["path"] = json!(a.abs_path);
    }
    v
}

/// Items in the order they were asked for.
fn assets_in_order(db: &Db, ids: &[AssetId]) -> DbResult<Vec<Asset>> {
    let at: HashMap<AssetId, usize> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let mut assets = db.assets(ids)?;
    assets.sort_by_key(|a| at.get(&a.id).copied().unwrap_or(usize::MAX));
    Ok(assets)
}

fn items_with_fields(c: &Ctx, ids: &[AssetId]) -> Result<Vec<Value>, String> {
    read(c.host, |db| {
        let assets = assets_in_order(db, ids)?;
        let defs: HashMap<i64, FieldDef> = db.fields()?.into_iter().map(|f| (f.id, f)).collect();
        let full = crate::core::export::assets_json(db, &assets, &defs);
        Ok(assets.iter().zip(full).map(|(a, f)| {
            let mut v = item_json(c, db, a);
            v["fields"] = f["fields"].clone();
            if a.parts > 0 {
                v["parts"] = json!(db.members_of(a.id).unwrap_or_default().into_iter().map(|p| json!({"id": p.uid, "role": p.role, "name": p.name, "ext": p.ext, "size": p.size, "status": p.status.label()})).collect::<Vec<_>>());
            }
            v
        }).collect())
    })
}

/// A model struct as an answer, its keys in camelCase like every other answer.
fn to_json<T: serde::Serialize>(v: T) -> ApiResult {
    fn camel(v: Value) -> Value {
        match v {
            Value::Object(m) => Value::Object(m.into_iter().map(|(k, v)| (camel_key(&k), camel(v))).collect()),
            Value::Array(a) => Value::Array(a.into_iter().map(camel).collect()),
            v => v,
        }
    }
    fn camel_key(k: &str) -> String {
        let mut parts = k.split('_');
        let mut out = parts.next().unwrap_or_default().to_string();
        for p in parts {
            let mut c = p.chars();
            if let Some(f) = c.next() {
                out.extend(f.to_uppercase());
                out.push_str(c.as_str());
            }
        }
        out
    }
    serde_json::to_value(v).map(camel).map_err(|e| ApiError::new("failed", e.to_string()))
}

fn base64_bytes(data: &str) -> Result<Vec<u8>, ApiError> {
    // a data: address carries the bytes after its comma
    let b64 = data.split_once(";base64,").map_or(data, |(_, b)| b);
    base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|e| bad(format!("`data` isn't base64: {e}")))
}

/// What a thumbnail's first bytes say it is; thumbnails are stored without an extension.
fn image_type(head: &[u8]) -> &'static str {
    match head {
        [0xFF, 0xD8, ..] => "image/jpeg",
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "image/webp",
        _ => "application/octet-stream",
    }
}

