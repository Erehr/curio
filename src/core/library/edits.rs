//! Edits shared by the window's own commands and the API's operations, so both behave the same.
//! Each runs inside a writer job and returns what changed.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use serde_json::{json, Value};

use crate::core::db::{Db, DbResult};
use crate::core::library::paths::LibraryPaths;
use crate::core::library::{changes::Aspect, changes::Changed, LibEvent, Library, changes::Parts};
use crate::core::system::logging;
use crate::core::model::AssetId;
use crate::core::said::Said;
use crate::core::plugins::run::json_to_raw;
use crate::core::store::OnClash;
use crate::core::thumbs::{self, CustomThumb};

/// `what` is `annotation`, `url` or `name` (see `Db::rename_item`).
pub fn set_text(db: &Db, ids: &[AssetId], what: &str, value: &str) -> DbResult<Changed> {
    // a file grouped into an item stands for its item
    let ids = &db.items_of(ids)?;
    match what {
        "annotation" => db.set_annotation(ids, value)?,
        "url" => db.set_source_url(ids, value)?,
        "name" => {
            for id in ids {
                db.rename_item(*id, value)?;
            }
        }
        other => return Err(format!("unknown property {other}")),
    }
    Ok(Changed::items(ids))
}

/// A null or blank value clears the field; on a field readers fill, that takes the file's value again.
pub fn set_field(db: &Db, ids: &[AssetId], key: &str, value: &Value) -> DbResult<Changed> {
    // a file grouped into an item stands for its item
    let ids = &db.items_of(ids)?;
    let cat = db.field_catalog();
    let f = cat.custom(key).ok_or(format!("unknown field {key}"))?;
    let clear = value.is_null() || value.as_str().is_some_and(|s| s.trim().is_empty());
    db.set_field_by_hand(ids, f, (!clear).then(|| json_to_raw(value)).as_ref())?;
    db.touch(ids)?;
    Ok(Changed::items(ids))
}

pub fn set_rating(db: &Db, ids: &[AssetId], rating: i64) -> DbResult<Changed> {
    // a file grouped into an item stands for its item
    let ids = &db.items_of(ids)?;
    db.set_rating(ids, rating)?;
    Ok(Changed::items(ids))
}

pub fn set_favorite(db: &Db, ids: &[AssetId], on: bool) -> DbResult<Changed> {
    let ids = &db.items_of(ids)?;
    db.set_favorite(ids, on)?;
    Ok(Changed::items(ids).with(Parts::SIDEBAR))
}

pub fn set_explicit(db: &Db, ids: &[AssetId], on: bool) -> DbResult<Changed> {
    let ids = &db.items_of(ids)?;
    db.set_explicit(ids, on)?;
    Ok(Changed::items(ids))
}

/// Whole items to the trash, or back from it with every folder entry of theirs that was there.
pub fn trash(db: &Db, ids: &[AssetId], deleted: bool) -> DbResult<Changed> {
    // a file grouped into an item stands for its item
    let ids = &db.items_of(ids)?;
    db.set_deleted(ids, deleted)?;
    if !deleted {
        db.restore_entries(ids, None)?;
        db.restore_folders(ids)?;
        return Ok(Changed::added(ids, "restore"));
    }
    Ok(Changed::removed(ids, false))
}

/// Delete in a folder view: see `Db::trash_entries`. The folder's subfolders count, as the view may show their contents.
pub fn trash_in_folder(db: &Db, ids: &[AssetId], folder: i64) -> DbResult<Changed> {
    // a file grouped into an item stands for its item
    let ids = &db.items_of(ids)?;
    let whole = db.trash_entries(ids, folder, true)?;
    Ok(entries_left(ids, &whole))
}

/// Items that left a folder: those in no other folder went to the trash.
fn entries_left(ids: &[AssetId], whole: &[AssetId]) -> Changed {
    let kept: Vec<AssetId> = ids.iter().filter(|i| !whole.contains(i)).copied().collect();
    Changed::items(&kept).and(Changed::removed(whole, false))
}

/// Items dragged out of the trash onto a folder: they come back in that folder only, and the folders they were in before stay as they are.
pub fn restore_to_folder(db: &Db, ids: &[AssetId], folder: i64) -> DbResult<Changed> {
    // a file grouped into an item stands for its item
    let ids = &db.items_of(ids)?;
    let back = db.trashed_among(ids)?;
    db.restore_into(ids, folder)?;
    Ok(returned(ids, &back))
}

/// Items back from the trash, and others that only got a folder entry back.
fn returned(ids: &[AssetId], back: &[AssetId]) -> Changed {
    let rest: Vec<AssetId> = ids.iter().filter(|i| !back.contains(i)).copied().collect();
    Changed::items(&rest).and(Changed::added(back, "restore"))
}

/// Undo of `trash_in_folder`.
pub fn restore_in_folder(db: &Db, ids: &[AssetId], folder: i64) -> DbResult<Changed> {
    // a file grouped into an item stands for its item
    let ids = &db.items_of(ids)?;
    let back = db.trashed_among(ids)?;
    db.set_deleted(ids, false)?;
    db.restore_entries(ids, Some(folder))?;
    Ok(returned(ids, &back))
}

pub fn add_tags(db: &Db, ids: &[AssetId], names: &[String]) -> DbResult<Changed> {
    // a file grouped into an item stands for its item
    let ids = &db.items_of(ids)?;
    let before = db.list_sizes()?;
    db.add_tags(ids, names)?;
    Ok(Changed::items(ids).with(new_lists(db, before)?))
}

/// The tag and folder lists, when an edit added to them.
pub fn new_lists(db: &Db, before: (i64, i64)) -> DbResult<Parts> {
    let now = db.list_sizes()?;
    let mut p = Parts::default();
    if now.0 != before.0 {
        p = p | Parts::TAGS;
    }
    if now.1 != before.1 {
        p = p | Parts::FOLDERS;
    }
    Ok(p)
}

pub fn remove_tags(db: &Db, ids: &[AssetId], tag_ids: &[i64]) -> DbResult<Changed> {
    // a file grouped into an item stands for its item
    let ids = &db.items_of(ids)?;
    for t in tag_ids {
        db.remove_tag(ids, *t)?;
    }
    Ok(Changed::items(ids))
}

/// A thumbnail chosen by the user, for every item in `ids`.
pub fn custom_thumbnail(db: &Db, thumbs_dir: &Path, ids: &[AssetId], t: &CustomThumb) -> DbResult<Changed> {
    for id in ids {
        let (uid, ver, _) = db.next_thumb_version(*id)?;
        thumbs::write_bytes(&t.bytes, &thumbs::thumb_path(thumbs_dir, &uid, ver))?;
        thumbs::remove_older(thumbs_dir, &uid, ver);
        db.set_thumb_done(*id, true, None, Some(&t.palette), Some(&t.look))?;
        if t.aspect.is_finite() && t.aspect > 0.0 {
            db.conn
                .execute("UPDATE assets SET aspect = ? WHERE id = ? AND kind NOT IN ('image', 'video')", rusqlite::params![t.aspect, *id])
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(Changed::shown(ids, Aspect::Thumbnail))
}

/// How often an import under way refreshes what's on screen.
const IMPORT_REFRESH: std::time::Duration = std::time::Duration::from_millis(400);

/// Runs on what an import brought in, in the same job.
pub type AfterImport = Box<dyn FnOnce(&Db, &[AssetId]) -> DbResult<()> + Send>;

/// Files and folders dropped or sent to the library, imported on the scanner thread with progress; `after` then runs on the new items, and `how` says where they came from (see `Changed::how`).
pub fn add_paths(lib: &Library, paths: Vec<PathBuf>, copy: bool, folder: Option<i64>, clash: OnClash, after: Option<AfterImport>, how: &'static str) -> DbResult<()> {
    // the first file is checked: very long folder names arrive as odd paths
    if let Some(first) = paths.first() {
        if !first.exists() {
            return Err(format!("Couldn't add {}: the path is too long or the file is gone", first.display()));
        }
    }
    let transfer = crate::core::library::instance::read_transfer(&crate::core::system::app_dirs::data_dir());
    let style = lib.settings().name_suffix;
    let lp = lib.paths().clone();
    let links = lib.hard_links();
    // an Eagle library is brought in as one, with its folders and tags, never as its per-item folders and their thumbnails
    let (eagle, paths): (Vec<PathBuf>, Vec<PathBuf>) = paths.into_iter().partition(|p| crate::core::import::eagle::is_eagle_library(p));
    lib.job(move |db, ctx| {
        let opts = ctx.scan_options();
        let started = chrono::Utc::now().timestamp_millis();
        let before = db.list_sizes()?;
        // shown as they arrive, a few times a second: every file would be too many refreshes
        let batch = std::cell::RefCell::new((Vec::new(), std::time::Instant::now(), 0usize));
        // locations made whose scans haven't said they ended: an import that fails says it for them
        let scanning = std::cell::RefCell::new(std::collections::HashSet::new());
        let step = |s: crate::core::import::ImportStep| {
            match s.location {
                Some(crate::core::import::Located::Started(id)) => {
                    scanning.borrow_mut().insert(id);
                    ctx.send(LibEvent::Changed(Changed::looks(Parts::LOCATIONS)));
                    ctx.scan_started(id);
                }
                Some(crate::core::import::Located::Scanned(id, stats)) => {
                    scanning.borrow_mut().remove(&id);
                    ctx.scan_ended(id, stats);
                }
                None => {}
            }
            *ctx.import.lock() = Some(crate::core::library::ImportState { done: s.done, total: s.total, name: s.name.to_string(), started });
            let mut b = batch.borrow_mut();
            b.0.extend_from_slice(s.new);
            if !b.0.is_empty() && b.1.elapsed() >= IMPORT_REFRESH {
                ctx.send(LibEvent::Changed(Changed::added(&b.0, how)));
                ctx.thumbs_wanted.store(true, Ordering::Relaxed);
                b.2 += b.0.len();
                b.0.clear();
                b.1 = std::time::Instant::now();
            }
        };
        let mut told = vec![];
        let mut eagle_ids = vec![];
        for lib in &eagle {
            let s = if copy {
                let got = crate::core::import::eagle::import_dot_library_copy(db, &lp, lib, folder, style, links, &step, &ctx.cancel);
                if got.is_err() {
                    *ctx.import.lock() = None;
                }
                let (s, ids) = got?;
                eagle_ids.extend(ids);
                s
            } else {
                let s = crate::core::import::eagle::import_dot_library(db, lib, folder)?;
                ctx.send(LibEvent::Changed(Changed::looks(Parts::LOCATIONS)));
                s
            };
            told.push(crate::core::import::eagle::eagle_report(&s, copy));
        }
        let got = if paths.is_empty() { Ok(vec![]) } else { crate::core::import::bring_in_stepped(db, &lp, &paths, copy, folder, style, clash, &transfer, &opts, &step, &ctx.cancel) };
        *ctx.import.lock() = None;
        for id in scanning.take() {
            ctx.scan_ended(id, Default::default());
        }
        let rest_ids = got?;
        let rest = rest_ids.len();
        let mut ids = eagle_ids;
        ids.extend(rest_ids);
        let announced = batch.borrow().2.min(ids.len());
        let placed = after.is_some();
        if let Some(f) = after {
            db.tx(|db| f(db, &ids))?;
        }
        ctx.thumbs_wanted.store(true, Ordering::Relaxed);
        logging::info(&format!("added {} item(s), folder {folder:?}", ids.len()));
        let target = folder.and_then(|f| db.folders().ok()?.into_iter().find(|x| x.id == f).map(|x| x.name));
        if !paths.is_empty() || eagle.is_empty() {
            let s = if rest == 1 { "" } else { "s" };
            ctx.send(LibEvent::Done(match target {
                Some(name) => Said::new("results.add.addedTo", json!({"count": rest, "folder": name}), format!("Added {rest} item{s} to {name}")),
                None => Said::new("results.add.added", json!({"count": rest}), format!("Added {rest} item{s}")),
            }));
        }
        for t in told {
            ctx.send(LibEvent::Done(t));
        }
        // items announced while the import ran got their folders and tags only now
        let (earlier, last) = ids.split_at(announced);
        let c = Changed::added(last, how);
        let c = if placed { Changed::items(earlier).and(c) } else { c };
        let c = if eagle.is_empty() { c } else { c.with(Parts::ALL) };
        Ok(c.with(new_lists(db, before)?))
    });
    lib.kick_thumbs();
    Ok(())
}

/// What an add does with a file whose bytes the library already holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnDuplicate {
    #[default]
    Keep,
    Skip,
    Tell,
}

/// One file into the store, compared with what the library holds unless `dup` is Keep: the item made (none when skipped as a duplicate) and the item it duplicates.
#[allow(clippy::too_many_arguments)]
pub fn import_checked(db: &mut Db, lp: &LibraryPaths, src: crate::core::import::Incoming, name: &str, url: Option<&str>, folder: Option<i64>, style: crate::core::store::SuffixStyle, dup: OnDuplicate) -> DbResult<(Option<AssetId>, Option<AssetId>)> {
    let found = if dup == OnDuplicate::Keep {
        None
    } else {
        let (size, hash) = match &src {
            crate::core::import::Incoming::Bytes(b) => (b.len() as u64, blake3::hash(b).to_hex().to_string()),
            crate::core::import::Incoming::File(p) | crate::core::import::Incoming::Linked(p) | crate::core::import::Incoming::Moved(p) => (std::fs::metadata(p).map_err(|e| e.to_string())?.len(), crate::core::system::filesystem::content_hash(p).map_err(|e| e.to_string())?),
        };
        let of = same_content(db, size, &hash)?;
        Some((hash, of))
    };
    if let (Some((_, Some(of))), OnDuplicate::Skip) = (&found, dup) {
        return Ok((None, Some(*of)));
    }
    let (id, _) = crate::core::import::import_one(db, lp, src, name, folder, crate::core::store::OnClash::Keep, style, url)?;
    if let Some((hash, _)) = &found {
        db.set_content_hash(id, hash)?;
    }
    Ok((Some(id), found.and_then(|f| f.1)))
}

/// The item, not in the trash, whose file has these bytes; hashes of items of the same size are worked out and kept on the way, as the duplicate finder would.
pub fn same_content(db: &Db, size: u64, hash: &str) -> DbResult<Option<AssetId>> {
    if size == 0 {
        return Ok(None);
    }
    for a in db.assets(&db.ids_of_size(size as i64)?)? {
        let h = match &a.content_hash {
            Some(h) => h.clone(),
            None => match crate::core::system::filesystem::content_hash(std::path::Path::new(&a.abs_path)) {
                Ok(h) => {
                    db.set_content_hash(a.id, &h)?;
                    h
                }
                Err(_) => continue,
            },
        };
        if h == hash {
            return Ok(Some(a.id));
        }
    }
    Ok(None)
}

#[derive(Default)]
pub struct FolderArgs {
    pub id: Option<i64>,
    pub ids: Vec<AssetId>,
    pub name: Option<String>,
    pub parent: Option<i64>,
    pub value: Option<String>,
    pub before: Option<i64>,
}

pub fn folder_op(db: &Db, op: &str, mut a: FolderArgs) -> DbResult<Changed> {
    a.ids = db.items_of(&a.ids)?;
    let id = || a.id.ok_or("id");
    match op {
        "create" => {
            let fid = db.create_folder(a.name.as_deref().unwrap_or("New folder"), a.parent)?;
            if !a.ids.is_empty() {
                db.add_to_folder(&a.ids, fid)?;
            }
        }
        "rename" => db.rename_folder(id()?, a.name.as_deref().unwrap_or(""))?,
        "delete" => return Ok(Changed::removed(&db.delete_folder(id()?)?, false).with(Parts::FOLDERS)),
        "move" => db.move_folder(id()?, a.parent)?,
        // into the same-named folder `parent`
        "merge" => db.merge_folder(id()?, a.parent.ok_or("parent")?)?,
        "place" => db.place_folder(id()?, a.parent, a.before)?,
        "sort" => {
            // value: asc or desc, by name, or count-asc or count-desc by the items held; with -all for every level
            let v = a.value.as_deref().unwrap_or("asc");
            let by_count = v.starts_with("count-");
            let v = v.trim_start_matches("count-");
            db.sort_folders(a.parent, v.starts_with("desc"), v.ends_with("-all"), by_count)?
        }
        "clone" => {
            db.clone_folder(id()?)?;
        }
        "clear" => db.clear_folders(&a.ids)?,
        "auto_tags" => {
            let tags: Vec<String> = a.value.as_deref().unwrap_or("").split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
            db.set_folder_auto_tags(id()?, &tags)?
        }
        "color" => db.set_folder_color(id()?, a.value.as_deref())?,
        "icon" => db.set_folder_icon(id()?, a.value.as_deref())?,
        "add" => db.add_to_folder(&a.ids, id()?)?,
        // `value`: the folder being viewed, which they leave with its subfolders (it may show their contents), or `all` for every folder they are in
        "move_items" => {
            let target = id()?;
            match a.value.as_deref().and_then(|v| v.parse::<i64>().ok()) {
                Some(from) if from == target => {}
                Some(from) => db.remove_from_folder_tree(&a.ids, from)?,
                None => db.clear_folders(&a.ids)?,
            }
            db.add_to_folder(&a.ids, target)?;
        }
        // through the trash, like Delete in that folder
        "remove" => return Ok(entries_left(&a.ids, &db.trash_entries(&a.ids, id()?, false)?)),
        // undo of a fresh `add`
        "unadd" => db.remove_from_folder(&a.ids, id()?)?,
        "reorder" => db.reorder_in_folder(id()?, &a.ids, a.before)?,
        other => return Err(format!("unknown folder op {other}")),
    }
    let items = Changed::items(&a.ids);
    Ok(match op {
        "add" | "move_items" | "unadd" | "clear" | "reorder" => items,
        // the folders' order and looks: a search can't ask for them (a place may take a folder to another parent, which a view with subfolders sees)
        "sort" | "icon" | "auto_tags" => Changed::looks(Parts::FOLDERS),
        "create" if a.ids.is_empty() => Changed::looks(Parts::FOLDERS),
        _ => items.with(Parts::FOLDERS),
    })
}

#[derive(Default)]
pub struct TagArgs {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub group: Option<i64>,
    pub color: Option<String>,
}

pub fn tag_op(db: &mut Db, op: &str, a: TagArgs) -> DbResult<Changed> {
    let id = || a.id.ok_or("id");
    let group = || a.group.ok_or("group");
    match op {
        "rename" => db.rename_tag(id()?, a.name.as_deref().unwrap_or(""))?,
        "delete" => db.delete_tag(id()?)?,
        "rename_group" => db.rename_tag_group(id()?, a.name.as_deref().unwrap_or("Group"))?,
        // name carries the icon
        "group_look" => db.set_tag_group_look(id()?, a.color.as_deref(), a.name.as_deref())?,
        "delete_group" => db.delete_tag_group(id()?)?,
        "star" => db.set_tag_starred(id()?, true)?,
        "unstar" => db.set_tag_starred(id()?, false)?,
        "group" => db.set_tag_group(id()?, a.group)?,
        "add_to_group" => db.add_tag_to_group(id()?, group()?)?,
        "remove_from_group" => db.remove_tag_from_group(id()?, group()?)?,
        "group_description" => db.set_tag_group_description(id()?, a.name.as_deref().unwrap_or(""))?,
        "group_order" => {
            let list: Vec<i64> = a.name.as_deref().unwrap_or("").split(',').filter_map(|x| x.parse().ok()).collect();
            db.set_tag_group_order(&list)?
        }
        "new_tag" => {
            let t = db.get_or_create_tag(a.name.as_deref().unwrap_or("New tag"))?;
            if let Some(g) = a.group {
                db.add_tag_to_group(t, g)?;
            }
        }
        "new_group" => {
            db.create_tag_group(a.name.as_deref().unwrap_or("New group"), a.color.as_deref())?;
        }
        "color" => {
            db.conn.execute("UPDATE tags SET color = ? WHERE id = ?", rusqlite::params![a.color, a.id]).map_err(|e| e.to_string())?;
        }
        other => return Err(format!("unknown tag op {other}")),
    }
    // a search names tags, never their groups, stars or colours
    Ok(match op {
        "rename_group" | "group_look" | "delete_group" | "group_description" | "group_order" | "new_group" => Changed::looks(Parts::TAG_GROUPS),
        "delete" => Changed::parts(Parts::TAGS | Parts::TAG_GROUPS),
        "group" | "add_to_group" | "remove_from_group" => Changed::looks(Parts::TAGS | Parts::TAG_GROUPS),
        "new_tag" if a.group.is_some() => Changed::looks(Parts::TAGS | Parts::TAG_GROUPS),
        "rename" => Changed::parts(Parts::TAGS),
        _ => Changed::looks(Parts::TAGS),
    })
}

#[derive(Default)]
pub struct SmartArgs {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub query: Option<String>,
    pub icon: Option<String>,
    pub group_by: Option<String>,
    /// `create` inside it, or `place` under it (none: top level).
    pub parent: Option<i64>,
    /// `place` just before this sibling (none: last).
    pub before: Option<i64>,
}

/// `update` keeps the current name or query when one is not given.
pub fn smart_op(db: &Db, op: &str, a: SmartArgs) -> DbResult<Changed> {
    let mut moved = false;
    match op {
        "create" => {
            db.create_smart_folder_in(a.name.as_deref().unwrap_or("Smart folder"), a.query.as_deref().unwrap_or(""), a.parent, a.group_by.as_deref())?;
        }
        "place" => {
            let id = a.id.ok_or("id")?;
            moved = db.smart_folders()?.into_iter().find(|f| f.id == id).is_some_and(|f| f.parent_id != a.parent);
            db.place_smart_folder(id, a.parent, a.before)?
        }
        "update" => {
            let id = a.id.ok_or("id")?;
            let cur = db.smart_folders()?.into_iter().find(|f| f.id == id).ok_or("smart folder not found")?;
            let gb = match &a.group_by {
                Some(s) => Some(s.as_str()),
                None => cur.group_by.as_deref(),
            };
            db.update_smart_folder(id, a.name.as_deref().unwrap_or(&cur.name), a.query.as_deref().unwrap_or(&cur.query), gb)?
        }
        "delete" => db.delete_smart_folder(a.id.ok_or("id")?)?,
        "icon" => {
            db.conn.execute("UPDATE smart_folders SET icon = ? WHERE id = ?", rusqlite::params![a.icon, a.id]).map_err(|e| e.to_string())?;
        }
        other => return Err(format!("unknown smart folder op {other}")),
    }
    // what a smart folder shows changes with its query, grouping, or subfolders
    Ok(if a.query.is_some() || a.group_by.is_some() || moved || matches!(op, "create" | "delete") { Changed::parts(Parts::SMART) } else { Changed::looks(Parts::SMART) })
}
