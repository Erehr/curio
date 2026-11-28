//! Plugins as the library runs them: loaded, their programs' replies applied, their failures counted.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use serde_json::json;
use crate::core::db::{Db, DbResult};
use crate::core::system::logging;
use crate::core::model::*;
use crate::core::plugins::{self, manifest::Plugin};
use crate::core::said::Said;
use crate::core::thumbs;
use crate::core::library::changes::Changed;
use crate::core::library::LibEvent;
use crate::core::library::WriteFn;
use crate::core::library::WriterJob;
use crate::core::library::Ctx;

pub(super) fn load_plugins(db: &Db, ctx: &Ctx, dirs: &[PathBuf]) {
    let mut list = plugins::loading::builtin_plugins();
    // built-in plugins can be switched off too
    for p in list.iter_mut() {
        if let Some((enabled, _, _)) = db.plugin_state(&p.manifest.id) {
            p.enabled = enabled || plugins::always_on(&p.manifest.id);
        }
    }
    let approved = crate::core::import::auto_import::load_approvals();
    for mut p in plugins::loading::discover(dirs) {
        if let Some((enabled, granted, failures)) = db.plugin_state(&p.manifest.id) {
            p.enabled = enabled;
            p.configured = true;
            p.granted = granted;
            if failures >= plugins::MAX_FAILURES && p.problem.is_none() {
                p.problem = Some(format!("disabled after {failures} failures (re-enable to retry)"));
            }
        } else if p.problem.is_none() && !p.dir.starts_with(&ctx.paths.plugins) {
            // one added for every library (an added folder, the app's plugins folder) is on, with what it asks for, in a library that hasn't switched it off; one inside the library folder waits for approval instead
            p.enabled = true;
            p.configured = true;
            p.granted = p.manifest.permissions.clone();
            if let Err(e) = db.set_plugin_state(&p.manifest.id, true, &p.manifest.version, &p.granted) {
                logging::warn(&format!("switching on {}: {e}", p.manifest.id));
            }
        }
        plugins::loading::hold_until_approved(&mut p, &ctx.paths.plugins, &approved);
        list.push(p);
    }
    plugins::loading::resolve_dependencies(&mut list);
    // thumbnails that waited for FFmpeg are made now it is here
    if crate::core::media::engine() == crate::core::media::Engine::Ffmpeg {
        if let Err(e) = db.requeue_waiting_thumbs() {
            logging::warn(&format!("thumbnails waiting for FFmpeg: {e}"));
        }
    }
    let mut contrib = plugins::contributions::contributions(&list);
    crate::core::formats::set_plugin_kinds(contrib.ext_kinds.clone());
    crate::core::formats::set_plugin_types(contrib.format_types.clone());
    for h in contrib.thumb_handlers.iter_mut() {
        h.report = handler_report(ctx, &h.plugin_id);
    }
    // in one transaction: each on its own would write the pages it touches to the WAL again, at every open
    let batch = db.conn.is_autocommit() && db.conn.execute_batch("BEGIN IMMEDIATE").is_ok();
    let present: HashSet<&str> = list.iter().map(|p| p.manifest.id.as_str()).collect();
    if let Err(e) = db.prune_plugin_states(&present) {
        logging::warn(&format!("prune plugin states: {e}"));
    }
    for f in &contrib.fields {
        if let Err(e) = db.register_field(f) {
            logging::warn(&format!("plugin field {}: {e}", f.key));
        }
    }
    for (ext, kind) in &contrib.ext_kinds {
        if let Err(e) = db.conn.execute("UPDATE assets SET kind = ? WHERE ext = ? AND kind = 'other'", rusqlite::params![kind.as_str(), ext]) {
            logging::warn(&format!("plugin format kind {ext}: {e}"));
        }
    }
    if batch {
        if let Err(e) = db.conn.execute_batch("COMMIT") {
            logging::warn(&format!("plugin fields: {e}"));
        }
    }
    if let Err(e) = db.set_known_readers(&contrib.known_readers) {
        logging::warn(&format!("readers: {e}"));
    }
    // thumbnails left to a page that no longer makes them, and failed ones a page now makes, are tried again
    let paged: Vec<String> = contrib.page_thumbs.iter().flat_map(|t| t.exts.clone()).collect();
    if let Err(e) = db.requeue_page_thumbs(&paged) {
        logging::warn(&format!("plugin thumbnails: {e}"));
    }
    *ctx.plugins.write() = list;
    *ctx.contrib.write() = contrib;
}

pub(super) fn record_failure(ctx: &Ctx, plugin_id: &str, err: &str) {
    logging::error(&format!("plugin {plugin_id}: {err}"));
    let _ = ctx.writer.send(WriterJob::Run(count_failure(plugin_id.to_string())));
}

fn count_failure(plugin_id: String) -> WriteFn {
    Box::new(move |db, ctx| {
        let n = db.plugin_failure(&plugin_id, false);
        if n >= plugins::MAX_FAILURES {
            let mut newly_disabled = false;
            {
                let mut list = ctx.plugins.write();
                if let Some(p) = list.iter_mut().find(|p| p.manifest.id == plugin_id) {
                    newly_disabled = p.problem.is_none();
                    p.problem = Some(format!("disabled after {n} failures"));
                }
            }
            {
                let mut contrib = ctx.contrib.write();
                contrib.thumb_handlers.retain(|h| h.plugin_id != plugin_id);
                contrib.readers.retain(|r| r.owner != plugin_id);
            }
            if newly_disabled {
                ctx.send(LibEvent::Error(Said::new("errors.plugins.disabled", json!({"plugin": plugin_id, "count": n}), format!("Plugin {plugin_id} was disabled after {n} failures"))));
            }
        }
        Ok(Changed::default())
    })
}

fn clear_failures(plugin_id: String) -> WriteFn {
    Box::new(move |db, _| {
        db.plugin_failure(&plugin_id, true);
        Ok(Changed::default())
    })
}

pub(super) fn plugin_succeeded(ctx: &Ctx, plugin_id: &str) {
    let _ = ctx.writer.send(WriterJob::Run(clear_failures(plugin_id.to_string())));
}

/// Thumbnail handlers run for every file, so a success is written only when it ends a run of failures.
fn handler_report(ctx: &Ctx, plugin_id: &str) -> thumbs::HandlerReport {
    let writer = ctx.writer.clone();
    let id = plugin_id.to_string();
    let failing = Arc::new(AtomicBool::new(false));
    thumbs::HandlerReport(Some(Arc::new(move |err: Option<&str>| {
        let job = if err.is_some() {
            failing.store(true, Ordering::Relaxed);
            count_failure(id.clone())
        } else if failing.swap(false, Ordering::Relaxed) {
            clear_failures(id.clone())
        } else {
            return;
        };
        let _ = writer.send(WriterJob::Run(job));
    })))
}

pub(super) fn plugin_payload(db: &Db, plugin: &Plugin, event: &str, ids: &[AssetId]) -> serde_json::Value {
    let fields: HashMap<i64, FieldDef> = db.fields().unwrap_or_default().into_iter().map(|f| (f.id, f)).collect();
    let assets: Vec<serde_json::Value> = db
        .assets(ids)
        .unwrap_or_default()
        .iter()
        .map(|a| {
            if plugin.has("metadata.read") {
                let mut j = crate::core::export::asset_json(db, a, &fields);
                if !plugin.has("files.read") {
                    j["path"] = serde_json::Value::Null;
                }
                j
            } else {
                serde_json::json!({ "uid": a.uid, "path": if plugin.has("files.read") { serde_json::json!(a.abs_path) } else { serde_json::Value::Null } })
            }
        })
        .collect();
    serde_json::json!({ "api": plugins::PLUGIN_API_VERSION, "event": event, "assets": assets })
}

/// How long an action or URL program may run.
pub(super) const PROGRAM_TIMEOUT: Duration = Duration::from_secs(600);

/// `scope`: the items the plugin was called for; a reply may only change those (a run for addresses, which has none, may name any).
pub(super) fn apply_reply(db: &Db, plugin: &Plugin, reply: plugins::run::PluginReply, ctx: &Ctx, scope: &[AssetId]) -> DbResult<Changed> {
    if let Some(m) = &reply.message {
        ctx.send(LibEvent::Done(m.clone().into()));
    }
    if reply.set.is_empty() {
        return Ok(Changed::default());
    }
    if !plugin.has("metadata.write") {
        return Err(format!("{} tried to change metadata without the 'metadata.write' permission", plugin.manifest.name));
    }
    let cat = db.field_catalog();
    let mut ids = Vec::new();
    let mut tagged = false;
    for ch in reply.set {
        let Ok(id) = db.conn.query_row("SELECT id FROM assets WHERE uid = uid_bytes(?)", [&ch.id], |r| r.get::<_, i64>(0)) else { continue };
        if !scope.is_empty() && !scope.contains(&id) {
            logging::warn(&format!("{} replied for an item it wasn't given ({})", plugin.manifest.id, ch.id));
            continue;
        }
        if !ch.tags.is_empty() {
            db.add_tags(&[id], &ch.tags)?;
            tagged = true;
        }
        if let Some(r) = ch.rating {
            db.set_rating(&[id], r)?;
        }
        if let Some(a) = ch.annotation {
            db.set_annotation(&[id], &a)?;
        }
        for (k, v) in ch.fields {
            if let Some(f) = cat.custom(&k) {
                if !plugins::may_write_field(&plugin.manifest.id, f) {
                    logging::warn(&format!("{} may not write {k}, which belongs to {}", plugin.manifest.id, f.owner));
                    continue;
                }
                if !f.indexed {
                    logging::warn(&format!("{}: {}", plugin.manifest.id, crate::core::db::not_indexed(&k)));
                    continue;
                }
                let raw = (!v.is_null()).then(|| plugins::run::json_to_raw(&v));
                db.set_field_by_hand(&[id], f, raw.as_ref())?;
                db.touch(&[id])?;
            }
        }
        ids.push(id);
    }
    let ch = Changed::items(&ids);
    Ok(if tagged { ch.with(crate::core::library::changes::Parts::TAGS) } else { ch })
}
