//! Adding items: from files, bytes and addresses, as jobs a caller follows.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::core::api::{with_library, ApiError, ApiResult, CallerKind, Ctx};
use crate::core::db::{Db, DbResult};
use crate::core::library::{changes::Changed, changes::Parts, Ctx as LibCtx, LibEvent};
use crate::core::model::*;
use crate::core::said::Said;
use crate::core::library::edits::{self};
use crate::core::store::OnClash;
use crate::core::import::downloads;
use crate::core::api::operations::item_edits::writable;
use crate::core::api::operations::job_for;
use crate::core::api::operations::no_file;
use crate::core::api::operations::not_found;
use crate::core::api::operations::read_checked;
use crate::core::api::operations::one_id;
use crate::core::api::operations::ItemRef;
use crate::core::api::operations::WRITE_WAIT;
use crate::core::api::operations::base64_bytes;
use crate::core::api::operations::URL_WAIT;
use crate::core::api::operations::uids;
use crate::core::api::operations::FILE_WAIT;
use crate::core::api::operations::item_edits::folders_exist;
use crate::core::api::operations::jobs::may_name;
use crate::core::api::operations::bad;

/// Where new items go and what they get.
#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct Placing {
    /// Folders to put the new items in.
    #[serde(default)]
    folders: Vec<i64>,
    /// Tags for the new items; new names are created.
    #[serde(default)]
    tags: Vec<String>,
    /// Notes for the new items.
    #[serde(default)]
    annotation: Option<String>,
    /// Where the new items came from, such as the page a video was downloaded from.
    #[serde(default)]
    source_url: Option<String>,
    /// 0 to 5.
    #[serde(default)]
    rating: Option<i64>,
    /// Mark the new items as favorites.
    #[serde(default)]
    favorite: Option<bool>,
    /// Mark the new items as explicit.
    #[serde(default)]
    explicit: Option<bool>,
    /// Custom field values by key.
    #[serde(default)]
    fields: Option<serde_json::Map<String, Value>>,
    /// Names this add: a second call with the same id within ten minutes answers what the first did (`queued` while it runs), with `repeat: true`, and adds nothing. Up to 128 characters.
    #[serde(default)]
    request_id: Option<String>,
}

impl Placing {
    /// The import itself puts items in the first folder, so their files land in its place in the library's store.
    fn first(&self) -> Option<i64> {
        self.folders.first().copied()
    }

    fn apply(&self, db: &Db, ids: &[AssetId]) -> DbResult<()> {
        if ids.is_empty() {
            return Ok(());
        }
        for f in self.folders.iter().skip(1) {
            db.add_to_folder(ids, *f)?;
        }
        if !self.tags.is_empty() {
            db.add_tags(ids, &self.tags)?;
        }
        if let Some(a) = self.annotation.as_deref().filter(|a| !a.is_empty()) {
            db.set_annotation(ids, a)?;
        }
        if let Some(u) = self.source_url.as_deref().filter(|u| !u.is_empty()) {
            edits::set_text(db, ids, "url", u)?;
        }
        if let Some(r) = self.rating {
            edits::set_rating(db, ids, r)?;
        }
        if let Some(v) = self.favorite {
            edits::set_favorite(db, ids, v)?;
        }
        if let Some(v) = self.explicit {
            edits::set_explicit(db, ids, v)?;
        }
        for (k, v) in self.fields.iter().flatten() {
            edits::set_field(db, ids, k, v)?;
        }
        Ok(())
    }

    /// For a URL handler's program, which adds the items itself.
    fn json(&self) -> Value {
        json!({ "folders": self.folders, "tags": self.tags, "annotation": self.annotation, "sourceUrl": self.source_url, "rating": self.rating, "favorite": self.favorite, "explicit": self.explicit, "fields": self.fields })
    }
}

fn saved(ctx: &LibCtx, names: &[String]) {
    match names {
        [] => {}
        [one] => ctx.send(LibEvent::Done(Said::new("results.saved.one", json!({"name": one}), format!("Saved {one}")))),
        many => ctx.send(LibEvent::Done(Said::new("results.saved.many", json!({"count": many.len()}), format!("Saved {} items", many.len())))),
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct PathsParams {
    /// Files or folders on this computer.
    pub(super) paths: Vec<String>,
    /// Copy folders into the library too, instead of making them locations; files outside the locations are always copied.
    #[serde(default)]
    pub(super) copy: bool,
    /// Take the files into the library instead of copying them, as a downloader does with its temporary files; files only, not folders.
    #[serde(default, rename = "move")]
    pub(super) take: bool,
    /// When a copied file's name is taken in its folder: keep both (default), replace or skip.
    #[serde(default)]
    pub(super) on_clash: Option<String>,
    /// Answer once the items are in, with their ids, waiting up to two minutes, instead of at once.
    #[serde(default)]
    pub(super) wait: bool,
    #[serde(flatten)]
    pub(super) place: Placing,
}

pub(super) fn item_add_from_paths(c: &Ctx, p: PathsParams) -> ApiResult {
    if p.paths.is_empty() {
        return Err(bad("No paths given"));
    }
    may_name(c, &p.paths)?;
    once(c, p.place.request_id.clone(), |req| add_paths_once(c, p, req))
}

fn add_paths_once(c: &Ctx, p: PathsParams, req: Option<crate::core::api::requests::Request>) -> ApiResult {
    let clash: OnClash = match &p.on_clash {
        Some(s) => serde_json::from_value(json!(s)).map_err(|_| bad(format!("Unknown onClash {s}: keep, replace or skip")))?,
        None => OnClash::Keep,
    };
    folders_exist(c, &p.place.folders)?;
    crate::core::api::operations::item_edits::check_plugin_fields(c, p.place.fields.as_ref())?;
    let paths: Vec<PathBuf> = p.paths.iter().map(PathBuf::from).collect();
    if p.take {
        if let Some(d) = paths.iter().find(|f| !f.is_file()) {
            return Err(bad(format!("`move` takes files; {} isn't one", d.display())));
        }
        return import_job(c, FILE_WAIT, p.place, req, move |db, _, lp, style, folder| {
            let mut made = Made::default();
            for f in &paths {
                let name = crate::core::system::filesystem::name_of(f);
                match crate::core::import::import_one(db, lp, crate::core::import::Incoming::Moved(f), &name, folder, clash, style, None) {
                    Ok((_, crate::core::import::Outcome::Skipped)) => {}
                    Ok((id, _)) => made.items.push((id, name)),
                    Err(e) => made.errors.push(format!("{}: {e}", f.display())),
                }
            }
            made
        });
    }
    let first = p.place.first();
    let place = p.place;
    let how = how(c);
    let (tx, rx) = crossbeam_channel::bounded::<Vec<String>>(1);
    let wait = p.wait;
    let after = Box::new(move |db: &Db, ids: &[AssetId]| {
        let r = place.apply(db, ids);
        if let Some(req) = &req {
            req.done(&json!({"success": true, "ids": uids(db, ids).unwrap_or_default()}));
        }
        if wait {
            let _ = tx.send(uids(db, ids).unwrap_or_default());
        }
        r
    });
    with_library(c.host, |lib| edits::add_paths(lib, paths, p.copy, first, clash, Some(after), how))?;
    if !wait {
        return Ok(json!({"success": true, "queued": true}));
    }
    match rx.recv_timeout(FILE_WAIT) {
        Ok(ids) => Ok(json!({"success": true, "ids": ids})),
        Err(_) => Ok(json!({"success": true, "queued": true})),
    }
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct UrlsParams {
    /// Addresses of images, files or web pages.
    urls: Vec<String>,
    /// A name for the item, when only one address is given.
    #[serde(default)]
    name: Option<String>,
    /// Hand addresses a plugin takes over (its `urls` in plugin.json) to that plugin; true unless set to false. A plugin's own addresses never come back to it.
    #[serde(default)]
    handlers: Option<bool>,
    /// Sent as the `Referer` of each download, as the page that showed the file would; http and https only.
    #[serde(default)]
    referer: Option<String>,
    /// `auto` (unless set): a file, except that an address that looks like a page is asked first and kept as a link unless it answers with a picture, video, sound, PDF or archive. `file` always downloads, `link` never does.
    #[serde(default, rename = "as")]
    save_as: Option<String>,
    /// A downloaded file whose bytes the library already holds, or a link to an address the library already has a link to: `keep` it (unless set), `skip` it, or keep it and `tell`; the answer's `duplicates` lists `{ id, of }`, or `{ of, skipped: true }`.
    #[serde(default, rename = "onDuplicate")]
    on_duplicate: Option<String>,
    /// Wait for the downloads, up to a minute, and answer what they added (unless set to false); with false the answer is `queued` at once, and `items.added` tells of each item with this call's `requestId`.
    #[serde(default)]
    wait: Option<bool>,
    /// A `Cookie` header for each download, as the browser sends it to that site: sent to the address's own host only, never to one a redirect leads to, and never stored. Not for plugins.
    #[serde(default)]
    cookies: Option<String>,
    /// The `User-Agent` of each download, such as the browser's own.
    #[serde(default, rename = "userAgent")]
    user_agent: Option<String>,
    /// With one address in `urls`: other addresses of the same file, such as larger versions, tried in order before it; the first that answers with a file is kept.
    #[serde(default)]
    alternates: Vec<String>,
    #[serde(flatten)]
    place: Placing,
}

/// Where new items came from, for `items.added`.
pub(super) fn how(c: &Ctx) -> &'static str {
    if c.caller.kind == CallerKind::Window {
        "window"
    } else {
        "api"
    }
}

/// What an import made: items with a name for the window's message, errors, and duplicates found (the item made, none when skipped, and the item it duplicates).
#[derive(Default)]
struct Made {
    items: Vec<(AssetId, String)>,
    errors: Vec<String>,
    dups: Vec<(Option<AssetId>, AssetId)>,
}

impl Made {
    fn one(r: DbResult<(Option<AssetId>, Option<AssetId>)>, name: String) -> Self {
        match r {
            Ok((id, of)) => Made { items: id.map(|i| (i, name)).into_iter().collect(), errors: vec![], dups: of.map(|o| (id, o)).into_iter().collect() },
            Err(e) => Made { errors: vec![e], ..Default::default() },
        }
    }
}

/// `onDuplicate`, for adds that bring a file.
fn on_duplicate(v: Option<&str>) -> Result<edits::OnDuplicate, ApiError> {
    match v.unwrap_or("keep") {
        "keep" => Ok(edits::OnDuplicate::Keep),
        "skip" => Ok(edits::OnDuplicate::Skip),
        "tell" => Ok(edits::OnDuplicate::Tell),
        other => Err(bad(format!("`onDuplicate` is keep, skip or tell, not {other}"))),
    }
}

/// An add named by `requestId` runs once: a repeat gets the first one's answer.
fn once(c: &Ctx, request_id: Option<String>, f: impl FnOnce(Option<crate::core::api::requests::Request>) -> ApiResult) -> ApiResult {
    let req = match crate::core::api::requests::begin(c, request_id.as_deref())? {
        crate::core::api::requests::Begun::Repeat(v) => return Ok(v),
        crate::core::api::requests::Begun::Fresh(r) => r,
    };
    let out = f(req.clone());
    if let Some(r) = &req {
        r.finish(&out);
    }
    out
}

/// Imports on the scanner thread; the answer is kept for `req` once the job ends, however long the call waited.
fn import_job(c: &Ctx, wait: Duration, place: Placing, req: Option<crate::core::api::requests::Request>, f: impl FnOnce(&mut Db, &LibCtx, &crate::core::library::paths::LibraryPaths, crate::core::store::SuffixStyle, Option<i64>) -> Made + Send + 'static) -> ApiResult {
    folders_exist(c, &place.folders)?;
    crate::core::api::operations::item_edits::check_plugin_fields(c, place.fields.as_ref())?;
    let (tx, rx) = crossbeam_channel::bounded(1);
    let how = how(c);
    // a caller outside Curio has no window to say what failed
    let outside = c.caller.kind == CallerKind::Web;
    with_library(c.host, |lib| {
        let style = lib.settings().name_suffix;
        let lp = lib.paths().clone();
        lib.job(move |db, ctx| {
            let before = db.list_sizes()?;
            let made = f(db, ctx, &lp, style, place.first());
            let ids: Vec<AssetId> = made.items.iter().map(|m| m.0).collect();
            let (answer, mut changed) = settle(db, ctx, &place, &made, how, before)?;
            if outside {
                let piece = piece_of(db, &place, &made, &answer);
                if let Some(n) = crate::core::import::notices::of_rest(ctx.paths.root.clone(), &[(piece, None)]) {
                    crate::core::import::notices::post(n);
                }
            }
            if let Some(r) = &req {
                // once something was added, a repeat must not add it again, even when placing it failed
                match &answer {
                    Ok(v) => r.done(v),
                    Err(e) if !ids.is_empty() => r.done(&json!({"success": false, "ids": uids(db, &ids).unwrap_or_default(), "errors": [e.message]})),
                    Err(_) => {}
                }
            }
            let _ = tx.send(answer);
            changed.request = req.map(|r| r.id);
            Ok(changed)
        });
        lib.kick_thumbs();
        Ok(())
    })?;
    rx.recv_timeout(wait).unwrap_or_else(|_| Ok(json!({"success": true, "queued": true})))
}

/// Places what an import made and tells the window: the call's answer, and the change to publish.
fn settle(db: &mut Db, ctx: &LibCtx, place: &Placing, made: &Made, how: &'static str, before: (i64, i64)) -> DbResult<(ApiResult, Changed)> {
    let ids: Vec<AssetId> = made.items.iter().map(|m| m.0).collect();
    let placed = db.tx(|db| place.apply(db, &ids));
    saved(ctx, &made.items.iter().map(|m| m.1.clone()).collect::<Vec<_>>());
    let one = |id: AssetId| uids(db, &[id]).ok().and_then(|u| u.into_iter().next());
    let dups: Vec<Value> = made.dups.iter().filter_map(|(id, of)| Some(match id { Some(i) => json!({"id": one(*i)?, "of": one(*of)?}), None => json!({"of": one(*of)?, "skipped": true}) })).collect();
    let answer = match (ids.is_empty() && !made.errors.is_empty(), placed) {
        (true, _) => Err(ApiError::new("failed", made.errors.join("; "))),
        (false, Err(e)) => Err(ApiError::new("failed", format!("Added, but couldn't place the new items: {e}"))),
        (false, Ok(())) => {
            let mut v = json!({"success": true, "ids": uids(db, &ids).unwrap_or_default(), "errors": made.errors});
            if !dups.is_empty() {
                v["duplicates"] = json!(dups);
            }
            Ok(v)
        }
    };
    let changed = Changed::added(&ids, how).with(edits::new_lists(db, before)?);
    let changed = if place.favorite.is_some() { changed.with(Parts::SIDEBAR) } else { changed };
    Ok((answer, changed))
}

/// Imports a finished download on the library's job: its piece of the call's answer (`{ ids, errors, duplicates }`), and the change to publish.
pub(crate) fn import_download(db: &mut Db, ctx: &LibCtx, rec: &downloads::Record, outcome: downloads::Outcome) -> (Value, Changed) {
    let mut place: Placing = serde_json::from_value(rec.place.clone()).unwrap_or_default();
    // a folder deleted while the file came is left out; with none left, the item goes to the top level
    if let Ok(have) = db.folders() {
        place.folders.retain(|f| have.iter().any(|h| h.id == *f));
    }
    let dup = on_duplicate(Some(&rec.on_duplicate)).unwrap_or_default();
    let how = if rec.how == "window" { "window" } else { "api" };
    let style = ctx.settings.read().name_suffix;
    let folder = place.first();
    let url = rec.url.clone();
    let failed = |e: String| (json!({"ids": [], "errors": [if e.starts_with(url.as_str()) { e } else { format!("{url}: {e}") }]}), Changed::default());
    let before = match db.list_sizes() {
        Ok(b) => b,
        Err(e) => return failed(e),
    };
    let mut made = match outcome {
        downloads::Outcome::File { path, name, url } => {
            let r = edits::import_checked(db, &ctx.paths, crate::core::import::Incoming::Moved(&path), &name, Some(&url), folder, style, dup);
            // taken into the store, or left here when skipped as a duplicate
            let _ = std::fs::remove_file(&path);
            Made::one(r, name)
        }
        downloads::Outcome::Link => Made::one(link_checked(db, &ctx.paths, &rec.url, rec.name.as_deref(), folder, style, dup), rec.name.clone().unwrap_or_else(|| rec.url.clone())),
    };
    for e in made.errors.iter_mut().filter(|e| !e.starts_with(url.as_str())) {
        *e = format!("{url}: {e}");
    }
    let (answer, mut changed) = match settle(db, ctx, &place, &made, how, before) {
        Ok(x) => x,
        Err(e) => return failed(e),
    };
    changed.request = rec.request_id.clone();
    (piece_of(db, &place, &made, &answer), changed)
}

/// An add's part of its call's answer, with what its notice shows: `{ ids, names, errors, duplicates, folder }`.
fn piece_of(db: &Db, place: &Placing, made: &Made, answer: &ApiResult) -> Value {
    let ids: Vec<AssetId> = made.items.iter().map(|m| m.0).collect();
    let folder = place.first().and_then(|f| db.folders().ok()?.into_iter().find(|x| x.id == f).map(|x| x.name));
    let names: Vec<&String> = made.items.iter().map(|m| &m.1).collect();
    match answer {
        Ok(v) => json!({"ids": v["ids"], "names": names, "errors": v["errors"], "duplicates": v.get("duplicates").cloned().unwrap_or_else(|| json!([])), "folder": folder}),
        // added, but not placed: what was added still counts
        Err(e) => json!({"ids": uids(db, &ids).unwrap_or_default(), "names": names, "errors": [e.message], "folder": folder}),
    }
}

/// A link, compared with the library's links by address unless `dup` is Keep: the item made (none when skipped) and the link it repeats.
#[allow(clippy::too_many_arguments)]
fn link_checked(db: &mut Db, paths: &crate::core::library::paths::LibraryPaths, url: &str, title: Option<&str>, folder: Option<i64>, style: crate::core::store::SuffixStyle, dup: edits::OnDuplicate) -> DbResult<(Option<AssetId>, Option<AssetId>)> {
    let of = if dup == edits::OnDuplicate::Keep { None } else { db.link_with_address(url)? };
    if let (Some(o), edits::OnDuplicate::Skip) = (of, dup) {
        return Ok((None, Some(o)));
    }
    Ok((Some(crate::core::import::import_link(db, paths, url, title, folder, style)?), of))
}

/// Addresses plugins take over go to them: a program runs with them, or the plugin's page opens in the window with them in `plugin.run`. Answers the addresses left, what was handed over and what couldn't be.
fn hand_over(c: &Ctx, urls: Vec<String>, place: &Placing) -> Result<(Vec<String>, Vec<Value>, Vec<String>), ApiError> {
    let contrib = with_library(c.host, |lib| Ok(lib.contributions()))?;
    let mut groups: Vec<(String, crate::core::plugins::manifest::UrlDecl, Vec<String>)> = Vec::new();
    let mut rest = Vec::new();
    for u in urls {
        match contrib.url_handler(&u, c.caller.plugin.as_deref()) {
            Some((plugin, h)) => match groups.iter_mut().find(|g| g.0 == *plugin && g.1.id == h.id) {
                Some(g) => g.2.push(u),
                None => groups.push((plugin.clone(), h.clone(), vec![u])),
            },
            None => rest.push(u),
        }
    }
    let (mut handed, mut errors) = (Vec::new(), Vec::new());
    for (plugin, h, list) in groups {
        let ok = match &h.opens {
            Some(opens) => match c.host.window("plugin.open", json!({ "plugin": plugin, "opens": opens, "run": { "handler": h.id, "urls": list, "place": place.json() } }), false) {
                Some(Ok(_)) => true,
                Some(Err(e)) => {
                    errors.extend(list.iter().map(|u| format!("{u}: {}", e.message)));
                    false
                }
                None => {
                    errors.extend(list.iter().map(|u| format!("{u}: {} opens it in a window, and this library has none open", h.title)));
                    false
                }
            },
            None => {
                let (pl, h2, l2, pj) = (plugin.clone(), h.clone(), list.clone(), place.json());
                // the program reports through the window's messages; the call doesn't wait for it
                with_library(c.host, move |lib| {
                    lib.run_url_program(pl, h2, l2, pj);
                    Ok(())
                })?;
                true
            }
        };
        if ok {
            handed.extend(list.iter().map(|u| json!({ "url": u, "plugin": plugin, "handler": h.id })));
        }
    }
    Ok((rest, handed, errors))
}

pub(super) fn item_add_from_urls(c: &Ctx, p: UrlsParams) -> ApiResult {
    if p.urls.is_empty() {
        return Err(bad("No addresses given"));
    }
    on_duplicate(p.on_duplicate.as_deref())?;
    once(c, p.place.request_id.clone(), |req| add_urls_once(c, p, req))
}

fn add_urls_once(c: &Ctx, p: UrlsParams, req: Option<crate::core::api::requests::Request>) -> ApiResult {
    let name = if p.urls.len() == 1 { p.name } else { None };
    let save_as = match p.save_as.as_deref().unwrap_or("auto") {
        "auto" => "auto",
        "file" => "file",
        "link" => "link",
        other => return Err(bad(format!("`as` is auto, file or link, not {other}"))),
    };
    if let Some(r) = &p.referer {
        let web = url::Url::parse(r).is_ok_and(|u| matches!(u.scheme(), "http" | "https"));
        if !web || r.contains(['\r', '\n']) || r.len() > 2048 {
            return Err(bad("`referer` is a web address"));
        }
    }
    if let Some(k) = &p.cookies {
        if c.caller.plugin.is_some() {
            return Err(ApiError::new("not_allowed", "`cookies` aren't for plugins"));
        }
        if k.len() > 16 << 10 || k.contains(['\r', '\n']) {
            return Err(bad("`cookies` is one `Cookie` header, up to 16 KB"));
        }
    }
    if p.user_agent.as_deref().is_some_and(|u| u.trim().is_empty() || u.len() > 512 || u.contains(['\r', '\n'])) {
        return Err(bad("`userAgent` is one line, up to 512 characters"));
    }
    if !p.alternates.is_empty() {
        if p.urls.len() != 1 || p.alternates.len() > 8 {
            return Err(bad("`alternates` are up to 8 addresses, with one address in `urls`"));
        }
        for a in &p.alternates {
            crate::core::import::links::check_web_url(a).map_err(bad)?;
        }
    }
    let queue = with_library(c.host, |lib| Ok(lib.downloads(&c.host.data_dir(), c.host.events())))?;
    if let Some(v) = req.as_ref().and_then(|r| queue.repeat(&r.key())) {
        return Ok(v);
    }
    let (urls, handed, refused) = if p.handlers.unwrap_or(true) { hand_over(c, p.urls, &p.place)? } else { (p.urls, vec![], vec![]) };
    if urls.is_empty() {
        if handed.is_empty() {
            return Err(ApiError::new("failed", refused.join("; ")));
        }
        return Ok(json!({ "success": true, "ids": [], "errors": refused, "handed": handed }));
    }
    folders_exist(c, &p.place.folders)?;
    crate::core::api::operations::item_edits::check_plugin_fields(c, p.place.fields.as_ref())?;
    let (place, how) = (p.place.json(), how(c));
    let on_duplicate = p.on_duplicate.unwrap_or_else(|| "keep".into());
    let news = urls
        .into_iter()
        .map(|url| {
            let link = save_as == "link" || (save_as == "auto" && crate::core::thumbs::links::youtube_id(&url).is_some());
            downloads::New {
                ask: downloads::Ask { url, referer: p.referer.clone(), cookies: p.cookies.clone(), user_agent: p.user_agent.clone() },
                alternates: p.alternates.clone(),
                place: place.clone(),
                name: name.clone(),
                save_as: if link { "link" } else { save_as },
                on_duplicate: on_duplicate.clone(),
                how,
                request_id: req.as_ref().map(|r| r.id.clone()),
                request_key: req.as_ref().map(|r| r.key()),
                notify: c.caller.kind == CallerKind::Web,
            }
        })
        .collect();
    let (tx, rx) = crossbeam_channel::bounded(1);
    let r = req.clone();
    queue.add(
        news,
        Some(Box::new(move |out: ApiResult| {
            if let Some(r) = &r {
                match &out {
                    Ok(v) => r.done(v),
                    Err(_) => r.forget(),
                }
            }
            let _ = tx.send(out);
        })),
    );
    let queued = json!({"success": true, "queued": true});
    let mut out = if p.wait.unwrap_or(true) { rx.recv_timeout(URL_WAIT).unwrap_or(Ok(queued))? } else { queued };
    if !handed.is_empty() || !refused.is_empty() {
        if let Some(e) = out["errors"].as_array_mut() {
            e.extend(refused.into_iter().map(Value::String));
        }
        out["handed"] = json!(handed);
    }
    Ok(out)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct BytesParams {
    /// The file as base64, or a data: address; up to 64 MB of request.
    #[serde(default)]
    data: Option<String>,
    /// In place of `data`: the id `POST /api/v1/upload` answered, for files up to 2 GB. It works once, for the token that sent it, within ten minutes.
    #[serde(default)]
    upload: Option<String>,
    /// The file's name with its extension, e.g. `sunset.jpg`.
    name: String,
    /// Where the file came from.
    #[serde(default)]
    url: Option<String>,
    /// A file whose bytes the library already holds: `keep` it (unless set), `skip` it, or keep it and `tell`; the answer's `duplicates` lists `{ id, of }`, or `{ of, skipped: true }`.
    #[serde(default, rename = "onDuplicate")]
    on_duplicate: Option<String>,
    #[serde(flatten)]
    place: Placing,
}

pub(super) fn item_add_from_bytes(c: &Ctx, p: BytesParams) -> ApiResult {
    if !p.name.contains('.') {
        return Err(bad("`name` needs the file's extension, e.g. sunset.jpg"));
    }
    let dup = on_duplicate(p.on_duplicate.as_deref())?;
    once(c, p.place.request_id.clone(), |req| add_bytes_once(c, p, dup, req))
}

fn add_bytes_once(c: &Ctx, p: BytesParams, dup: edits::OnDuplicate, req: Option<crate::core::api::requests::Request>) -> ApiResult {
    let (name, url) = (p.name, p.url);
    let data = match (p.data, p.upload) {
        (Some(d), None) => d,
        (None, Some(id)) => {
            crate::core::api::upload::check(&id, c.caller.token.as_deref())?;
            let (dir, token) = (crate::core::api::upload::spool(&c.host.data_dir()), c.caller.token.clone());
            // claimed in the job, after the folders were found, so a call refused before it never spends the upload
            return import_job(c, FILE_WAIT, p.place, req, move |db, _, lp, style, folder| {
                let file = match crate::core::api::upload::claim(&dir, &id, token.as_deref()) {
                    Ok(f) => f,
                    Err(e) => return Made { errors: vec![e], ..Default::default() },
                };
                let made = edits::import_checked(db, lp, crate::core::import::Incoming::Moved(&file), &name, url.as_deref(), folder, style, dup);
                // taken into the store, or left here when skipped as a duplicate
                let _ = std::fs::remove_file(&file);
                Made::one(made, name)
            });
        }
        _ => return Err(bad("Give `data` or `upload`, one of them")),
    };
    let bytes = base64_bytes(&data)?;
    if bytes.is_empty() {
        return Err(bad("`data` is empty"));
    }
    import_job(c, FILE_WAIT, p.place, req, move |db, _, lp, style, folder| {
        let made = edits::import_checked(db, lp, crate::core::import::Incoming::Bytes(&bytes), &name, url.as_deref(), folder, style, dup);
        Made::one(made, name)
    })
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct BookmarkParams {
    url: String,
    /// The link's name; the address without one.
    #[serde(default)]
    name: Option<String>,
    #[serde(flatten)]
    place: Placing,
}

pub(super) fn item_add_bookmark(c: &Ctx, p: BookmarkParams) -> ApiResult {
    once(c, p.place.request_id.clone(), |req| {
        let (url, name) = (p.url, p.name);
        import_job(c, WRITE_WAIT, p.place, req, move |db, _, lp, style, folder| {
            let made = crate::core::import::import_link(db, lp, &url, name.as_deref(), folder, style).map(|id| (Some(id), None));
            Made::one(made, name.unwrap_or(url))
        })
    })
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct BesideParams {
    /// The item the new one goes beside.
    id: ItemRef,
    /// A file on this computer.
    #[serde(default)]
    path: Option<String>,
    /// The file as base64, or a data: address.
    #[serde(default)]
    data: Option<String>,
    /// The new file's name with its extension; the item's own name with the new file's extension without it.
    #[serde(default)]
    name: Option<String>,
}

pub(super) fn item_add_beside(c: &Ctx, p: BesideParams) -> ApiResult {
    let id = one_id(c, &p.id)?;
    let (orig, meta, folders) = read_checked(c, |db| {
        let Some(a) = db.asset(id)? else { return Ok(Err(not_found("No such item"))) };
        let fields: HashMap<i64, FieldDef> = db.fields()?.into_iter().map(|f| (f.id, f)).collect();
        let folders: Vec<i64> = db.asset_folders(id)?.into_iter().map(|f| f.id).collect();
        Ok(Ok((a.name.clone(), crate::core::export::asset_json(db, &a, &fields), folders)))
    })?;
    let tmp = with_library(c.host, |lib| Ok(lib.paths().tmp.clone()))?;
    let (src, temporary) = match (p.path, p.data) {
        (Some(path), None) => {
            may_name(c, std::slice::from_ref(&path))?;
            (PathBuf::from(path), false)
        }
        (None, Some(data)) => {
            std::fs::create_dir_all(&tmp).map_err(|e| ApiError::new("failed", e.to_string()))?;
            let f = tmp.join(format!("beside-{}", fastrand::u64(..)));
            std::fs::write(&f, base64_bytes(&data)?).map_err(|e| ApiError::new("failed", e.to_string()))?;
            (f, true)
        }
        _ => return Err(bad("Give either `path` or `data`")),
    };
    if !src.is_file() {
        return Err(no_file(&src.display()));
    }
    let name = match p.name.filter(|n| !n.trim().is_empty()) {
        Some(n) if n.contains('.') => n,
        Some(n) => return Err(bad(format!("`name` needs the file's extension, e.g. {n}.webp"))),
        None if temporary => return Err(bad("`name` needs the file's name with its extension when the file comes as `data`")),
        None => {
            let ext = src.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
            let stem = Path::new(&orig).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or(orig.clone());
            if ext.is_empty() { orig.clone() } else { format!("{stem}.{ext}") }
        }
    };
    let (style, lp) = with_library(c.host, |lib| Ok((lib.settings().name_suffix, lib.paths().clone())))?;
    let how = how(c);
    let from = src.clone();
    let made = job_for(c, FILE_WAIT, move |db, ctx| {
        let (id, _) = crate::core::import::import_one(db, &lp, crate::core::import::Incoming::File(&from), &name, folders.first().copied(), OnClash::Keep, style, None)?;
        db.tx(|db| {
            crate::core::import::apply_incoming(db, id, &meta)?;
            for f in folders.iter().skip(1) {
                db.add_to_folder(&[id], *f)?;
            }
            Ok(())
        })?;
        saved(ctx, std::slice::from_ref(&name));
        Ok((uids(db, &[id])?, Changed::added(&[id], how)))
    });
    if temporary {
        let _ = std::fs::remove_file(&src);
    }
    with_library(c.host, |lib| {
        lib.kick_thumbs();
        Ok(())
    })?;
    Ok(json!({ "success": true, "ids": made?, "errors": [] }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct WriteUrlParams {
    id: ItemRef,
    /// `replace` (the default) or `beside`.
    #[serde(default)]
    mode: Option<String>,
    /// For `beside`: the new file's name with its extension.
    #[serde(default)]
    name: Option<String>,
}

pub(super) fn item_write_url(c: &Ctx, p: WriteUrlParams) -> ApiResult {
    let id = one_id(c, &p.id)?;
    let beside = match p.mode.as_deref() {
        None | Some("replace") => false,
        Some("beside") => true,
        Some(m) => return Err(bad(format!("Unknown mode {m}: replace or beside"))),
    };
    if beside {
        read_checked(c, |db| Ok(db.asset(id)?.map(|_| ()).ok_or_else(|| not_found("No such item"))))?;
    } else {
        writable(c, id)?;
    }
    if let Some(n) = p.name.as_deref().filter(|n| !n.contains('.')) {
        return Err(bad(format!("`name` needs the file's extension, e.g. {n}.webp")));
    }
    let url = c.host.page_write(crate::core::api::PageWrite { item: id, beside, name: p.name }).ok_or_else(|| ApiError::new("not_allowed", "Only a plugin's page writes files by address"))?;
    Ok(json!({ "url": url }))
}
