//! Custom fields and the readers that fill them.

use crate::core::api::operations::write;
use std::collections::HashMap;
use std::path::PathBuf;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::core::api::{read, with_library, ApiError, ApiResult, Ctx};
use crate::core::library::{changes::Changed, changes::Parts};
use crate::core::model::*;
use crate::core::api::operations::item_reads::scope_of;
use crate::core::api::operations::item_reads::found;
use crate::core::api::operations::items_of;
use crate::core::api::operations::ids_of;
use crate::core::api::operations::ItemRef;
use crate::core::api::operations::done;
use crate::core::api::operations::write_for;
use crate::core::api::operations::bad;
use crate::core::api::operations::not_found;
use crate::core::api::operations::libraries::Nothing;
use crate::core::api::operations::to_json;

/// A field as `field.list` shows it, with the readers that fill it.
fn field_json(f: &FieldDef, readers: &[crate::core::metadata::readers::ReaderInfo]) -> ApiResult {
    let mut j = to_json(f)?;
    j["readers"] = json!(readers.iter().filter(|r| r.fills(&f.key)).map(|r| r.id.clone()).collect::<Vec<_>>());
    j["needsScan"] = json!(f.needs_scan);
    j["pair"] = json!(f.pair);
    j["filterGroup"] = json!(f.filter_group);
    Ok(j)
}

pub(super) fn field_list(c: &Ctx, _: Nothing) -> ApiResult {
    let (fields, readers) = read(c.host, |db| Ok((db.fields()?, db.known_readers())))?;
    Ok(Value::Array(fields.iter().map(|f| field_json(f, &readers)).collect::<Result<Vec<_>, _>>()?))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct FieldKey {
    key: String,
}

fn field_of(c: &Ctx, key: &str) -> Result<FieldDef, ApiError> {
    let all = read(c.host, |db| db.fields())?;
    all.into_iter().find(|f| f.key == key).ok_or_else(|| not_found(format!("There is no field {key}")))
}

pub(super) fn field_get(c: &Ctx, p: FieldKey) -> ApiResult {
    let f = field_of(c, &p.key)?;
    field_json(&f, &read(c.host, |db| Ok(db.known_readers()))?)
}

/// The properties `field.list` shows; on a field that exists, those left out keep their values.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct FieldDefine {
    /// Lower case, used in queries: `steps >= 20`.
    key: String,
    #[serde(default)]
    label: Option<String>,
    /// text, integer, float, boolean, date, datetime, single_select, multi_select, json, url or file_ref; needed for a new field.
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    description: Option<String>,
    /// The choices of a select or multiselect field.
    #[serde(default)]
    options: Option<Vec<String>>,
    /// `{ min, max, required }`.
    #[serde(default)]
    validation: Option<Value>,
    #[serde(default)]
    default_value: Option<String>,
    #[serde(default)]
    format: Option<String>,
    /// row (default), block or code.
    #[serde(default)]
    widget: Option<String>,
    #[serde(default)]
    visible: Option<bool>,
    #[serde(default)]
    sort_order: Option<i64>,
    /// A filter of its own in the filter bar and smart folder rules.
    #[serde(default)]
    own_filter: Option<bool>,
    /// How that filter asks: list, range, text or has; empty picks by type.
    #[serde(default)]
    filter_kind: Option<String>,
    /// Curio keeps what readers find for it, and it takes part in filters, sorts, columns and search; its pair follows.
    #[serde(default)]
    indexed: Option<bool>,
    /// The key of the field this one only makes sense with, as `lora_strength` with `lora`.
    #[serde(default)]
    pair: Option<String>,
    /// Number fields of one owner with the same name here are one filter of that name, with a range for each; empty takes the field out.
    #[serde(default)]
    filter_group: Option<String>,
    /// Switched on without reading the files yet; `field.rescan` reads them.
    #[serde(default)]
    needs_scan: Option<bool>,
}

pub(super) fn field_define(c: &Ctx, p: FieldDefine) -> ApiResult {
    let kind = p.kind.as_deref().map(|k| FieldType::parse(k).ok_or_else(|| bad(format!("Unknown field type {k}")))).transpose()?;
    let current = read(c.host, |db| db.fields())?.into_iter().find(|f| f.key == p.key);
    let mut f = match (current, &kind) {
        (Some(f), _) => f,
        (None, Some(t)) => FieldDef::new(&p.key, p.label.as_deref().unwrap_or(&p.key), t.clone(), "user"),
        (None, None) => return Err(bad("A new field needs a `type`")),
    };
    if let Some(t) = kind {
        if f.owner != "user" && t != f.ftype {
            return Err(bad(format!("{} belongs to plugin '{}'; its type can't change", f.key, f.owner)));
        }
        f.ftype = t;
    }
    if let Some(v) = p.validation {
        f.validation = serde_json::from_value(v).map_err(|e| bad(format!("validation: {e}")))?;
    }
    macro_rules! set {
        ($($name:ident),*) => { $(if let Some(v) = p.$name { f.$name = v; })* };
    }
    set!(label, description, options, format, widget, visible, sort_order, own_filter, filter_kind, needs_scan);
    if p.default_value.is_some() {
        f.default_value = p.default_value.filter(|s| !s.is_empty());
    }
    if let Some(pair) = p.pair {
        f.pair = Some(pair).filter(|s| !s.trim().is_empty());
    }
    if let Some(g) = p.filter_group {
        f.filter_group = Some(g.trim().to_string()).filter(|s| !s.is_empty());
    }
    let switch = p.indexed.filter(|on| *on != f.indexed);
    if let Some(on) = switch {
        f.indexed = on;
        if !on {
            f.needs_scan = false;
        }
    }
    let switched = write_for(c, move |db| {
        db.define_field(&f)?;
        let mut switched = vec![f.clone()];
        if let Some(on) = switch {
            // a field that only makes sense with another is switched with it
            for mut g in db.fields()?.into_iter().filter(|g| g.key != f.key && (g.pair.as_deref() == Some(f.key.as_str()) || f.pair.as_deref() == Some(g.key.as_str()))) {
                if g.indexed != on {
                    g.indexed = on;
                    g.needs_scan = on && f.needs_scan;
                    db.define_field(&g)?;
                    switched.push(g);
                }
            }
        }
        Ok((if switch.is_some() { switched } else { vec![] }, Changed::parts(Parts::FIELDS)))
    })?;
    // the full-text index holds the indexed fields' text: switching one rebuilds the rows of the items holding it
    let text: Vec<i64> = switched.iter().filter(|g| matches!(g.ftype, FieldType::Text | FieldType::SingleSelect | FieldType::MultiSelect | FieldType::Url)).map(|g| g.id).collect();
    if !text.is_empty() {
        with_library(c.host, |lib| {
            lib.reindex_text(text);
            Ok(())
        })
        .map_err(bad)?;
    }
    Ok(done())
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct ReadFields {
    id: ItemRef,
    /// Only these fields; without them, the fields read from files that are switched off.
    #[serde(default)]
    keys: Option<Vec<String>>,
}

/// A value read from a file as the field would store it.
fn typed_json(f: &FieldDef, v: &RawValue) -> Vec<Value> {
    crate::core::db::fields::typed_values(f, v).unwrap_or_default().into_iter().map(|(t, n)| match (n, &f.ftype) {
        (Some(n), &FieldType::Boolean) => json!(n != 0.0),
        (Some(n), _) => json!(n),
        (None, _) => json!(t),
    }).collect()
}

pub(super) fn item_read_fields(c: &Ctx, p: ReadFields) -> ApiResult {
    use crate::core::metadata::readers::{self, Outcome, Source};
    let id = ids_of(c, std::slice::from_ref(&p.id))?[0];
    let all = with_library(c.host, |lib| Ok(lib.contributions().readers)).map_err(bad)?;
    let (a, defs, parts) = read(c.host, |db| Ok((db.asset(id)?, db.fields()?, crate::core::metadata::readers::parts_of(db, id))))?;
    let a = a.ok_or_else(|| not_found("There is no such item"))?;
    let keys: Vec<String> = p.keys.unwrap_or_else(|| defs.iter().filter(|f| !f.indexed).map(|f| f.key.clone()).collect());
    let (path, info) = (PathBuf::from(&a.abs_path), readers::FileInfo { ext: &a.ext, size: a.size, duration: a.duration, parts: &parts });
    let mut cache = readers::Cache::default();
    // the first reader to give a key wins it, as when the file is read for the library
    let mut got: Vec<(String, String, Vec<Value>)> = Vec::new();
    for r in all.iter().filter(|r| r.applies(&a.ext) && keys.iter().any(|k| r.fills(k)) && !matches!(&r.source, Source::Extractor(x) if !x.command.is_empty())) {
        let Outcome::Read(values) = readers::run(r, &path, &info, &mut cache) else { continue };
        for (k, v) in values.into_iter().filter(|(k, _)| keys.contains(k)) {
            let Some(f) = defs.iter().find(|f| f.key == k) else { continue };
            match got.iter_mut().find(|g| g.0 == k) {
                Some(g) if g.1 == r.id => g.2.extend(typed_json(f, &v)),
                Some(_) => {}
                None => got.push((k, r.id.clone(), typed_json(f, &v))),
            }
        }
    }
    got.retain(|g| !g.2.is_empty());
    let fields: serde_json::Map<String, Value> = got.into_iter().map(|(k, _, mut v)| (k, if v.len() == 1 { v.remove(0) } else { Value::Array(v) })).collect();
    Ok(json!({ "fields": fields }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct FieldKeys {
    /// Only these fields; all of them without it.
    #[serde(default)]
    keys: Option<Vec<String>>,
}

pub(super) fn field_coverage(c: &Ctx, p: FieldKeys) -> ApiResult {
    let readers = with_library(c.host, |lib| Ok(lib.contributions().readers)).map_err(bad)?;
    read(c.host, |db| {
        let (values, exts, unread, queued, failed) = (db.value_counts()?, db.ext_counts(false)?, db.ext_counts(true)?, db.reader_counts(false)?, db.reader_counts(true)?);
        let known = db.known_readers();
        let mut out = Vec::new();
        for f in db.fields()?.into_iter().filter(|f| known.iter().any(|r| r.fills(&f.key)) && p.keys.as_ref().is_none_or(|k| k.contains(&f.key))) {
            let own: Vec<&crate::core::metadata::readers::Reader> = readers.iter().filter(|r| r.fills(&f.key)).collect();
            let every = own.iter().any(|r| r.exts.is_empty());
            let mut of: std::collections::BTreeSet<&str> = Default::default();
            for r in &own {
                of.extend(r.exts.iter().map(|e| e.as_str()));
            }
            let sum = |m: &HashMap<String, i64>| if every { m.values().sum() } else { of.iter().map(|e| m.get(*e).copied().unwrap_or(0)).sum::<i64>() };
            let applies = sum(&exts);
            let by = |m: &HashMap<String, i64>| own.iter().map(|r| m.get(&r.id).copied().unwrap_or(0)).sum::<i64>();
            // items in the trash keep their values, and a value can be set by hand on a file no reader applies to: never more than there are files
            let held = values.get(&f.id).copied().unwrap_or(0);
            out.push(json!({ "key": f.key, "withValue": if f.indexed { held.min(applies) } else { held }, "applies": applies, "waiting": (by(&queued) + sum(&unread)).min(applies), "failed": by(&failed).min(applies) }));
        }
        Ok(json!({ "fields": out }))
    })
    .map_err(Into::into)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct FieldFailures {
    #[serde(default)]
    keys: Option<Vec<String>>,
    /// That plugin's readers.
    #[serde(default)]
    plugin: Option<String>,
    /// 200 unless given; at most 2000.
    #[serde(default)]
    limit: Option<usize>,
}

pub(super) fn field_failures(c: &Ctx, p: FieldFailures) -> ApiResult {
    read(c.host, |db| {
        let readers: Vec<String> = db.known_readers().into_iter().filter(|r| p.plugin.as_ref().is_none_or(|pid| &r.owner == pid) && p.keys.as_ref().is_none_or(|k| k.iter().any(|k| r.fills(k)))).map(|r| r.id).collect();
        let rows = db.meta_errors(&readers, p.limit.unwrap_or(200).min(2000))?;
        let ids: Vec<AssetId> = rows.iter().map(|r| r.0).collect();
        let assets: HashMap<AssetId, Asset> = db.assets(&ids)?.into_iter().map(|a| (a.id, a)).collect();
        let list: Vec<Value> = rows.into_iter().filter_map(|(id, reader, error, at)| assets.get(&id).map(|a| json!({ "id": a.uid, "name": a.name, "reader": reader, "error": error, "at": at }))).collect();
        Ok(json!({ "failures": list }))
    })
    .map_err(Into::into)
}

pub(super) fn field_used_by(c: &Ctx, p: FieldKey) -> ApiResult {
    let contrib = with_library(c.host, |lib| Ok(lib.contributions())).map_err(bad)?;
    read(c.host, |db| {
        let cat = db.field_catalog();
        let names = |q: &str| crate::core::query::compile::fields_in(q, &cat).contains(&p.key);
        let smart: Vec<Value> = db.smart_folders()?.into_iter().filter(|s| names(&s.query)).map(|s| json!({ "id": s.id, "name": s.name })).collect();
        let mut filters: Vec<Value> = contrib.filters.iter().filter(|(_, f)| names(&f.query)).map(|(pid, f)| json!({ "plugin": pid, "label": f.label })).collect();
        filters.extend(contrib.panels.iter().flat_map(|(pid, pn)| pn.filters.iter().filter(|f| names(&f.query)).map(move |f| json!({ "plugin": pid, "label": format!("{} › {}", pn.title, f.label) }))));
        let sorts: Vec<Value> = contrib.sorts.iter().filter(|(_, s)| s.field == p.key).map(|(pid, s)| json!({ "plugin": pid, "label": s.label })).collect();
        Ok(json!({ "smartFolders": smart, "filters": filters, "sorts": sorts }))
    })
    .map_err(Into::into)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct FieldCleanup {
    /// Only these fields. A field whose plugin is gone, which can't be read again, is cleaned only when named here.
    #[serde(default)]
    keys: Option<Vec<String>>,
    /// Count only.
    #[serde(default)]
    dry_run: bool,
}

pub(super) fn field_cleanup(c: &Ctx, p: FieldCleanup) -> ApiResult {
    let (fields, known) = read(c.host, |db| Ok((db.fields()?, db.known_readers())))?;
    // switched off: only a field read from files can be, so one with no reader left lost its plugin
    let off: Vec<(FieldDef, bool)> = fields.into_iter().filter(|f| !f.indexed && f.owner != "user").map(|f| {
        let gone = !known.iter().any(|r| r.fills(&f.key));
        (f, gone)
    }).filter(|(f, gone)| match &p.keys {
        Some(k) => k.contains(&f.key),
        None => !gone,
    }).collect();
    let counts: Vec<(String, i64, bool)> = read(c.host, |db| Ok(off.iter().map(|(f, gone)| (f.key.clone(), db.removable_values(f.id), *gone)).collect()))?;
    let mut removed = 0usize;
    if !p.dry_run {
        for (f, _) in &off {
            loop {
                let id = f.id;
                let n = write_for(c, move |db| Ok((db.remove_values(id, 5000)?, Changed::default())))?;
                removed += n;
                if n == 0 {
                    break;
                }
            }
        }
    }
    Ok(json!({ "fields": counts.iter().map(|(k, n, gone)| json!({ "key": k, "values": n, "pluginGone": gone })).collect::<Vec<_>>(), "removed": removed }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct FieldRescan {
    /// The fields to read: every reader that fills one of them reads its files.
    #[serde(default)]
    keys: Option<Vec<String>>,
    /// Read with this plugin's readers.
    #[serde(default)]
    plugin: Option<String>,
    /// Only these items; without them, every file the readers apply to.
    #[serde(default)]
    items: Option<Vec<ItemRef>>,
    /// missing: only files with none of the readers' fields, filling only what has no value; all (the default): what was read before is replaced, and what a file no longer has is cleared. Values set by hand stay either way.
    #[serde(default)]
    mode: Option<String>,
}

pub(super) fn field_rescan(c: &Ctx, p: FieldRescan) -> ApiResult {
    use crate::core::metadata::readers::Mode;
    let mode = match p.mode.as_deref() {
        None | Some("all") => Mode::All,
        Some("missing") => Mode::Missing,
        Some(m) => return Err(bad(format!("Unknown mode {m}: missing or all"))),
    };
    // a file grouped into an item is read with its item
    let scope = p.items.as_deref().map(|i| items_of(c, i)).transpose()?;
    let rx = with_library(c.host, |lib| {
        let all = lib.contributions().readers;
        // missing those fields, not every field the reader fills: a field just switched on is read on files that hold the others
        let narrow = |mut i: crate::core::metadata::readers::ReaderInfo| {
            if let (Mode::Missing, Some(keys)) = (mode, &p.keys) {
                i.keys.retain(|k| keys.contains(k));
            }
            i
        };
        let readers: Vec<_> = all.iter().filter(|r| p.plugin.as_ref().is_none_or(|pid| &r.owner == pid) && p.keys.as_ref().is_none_or(|keys| keys.iter().any(|k| r.fills(k)))).map(|r| narrow(r.info(true))).collect();
        if readers.is_empty() {
            return Err("No reader that is on reads those fields".into());
        }
        Ok(lib.rescan(readers, scope, mode, c.host.events(), c.caller.plugin.clone()))
    })
    .map_err(bad)?;
    let (job, files) = rx.recv().map_err(|e| bad(e.to_string()))?.map_err(bad)?;
    Ok(json!({ "job": job, "files": files }))
}

pub(super) fn field_delete(c: &Ctx, p: FieldKey) -> ApiResult {
    let f = field_of(c, &p.key)?;
    if f.owner != "user" {
        return Err(bad(format!("{} belongs to plugin '{}'; hide it instead", f.key, f.owner)));
    }
    write(c, move |db| {
        db.delete_field(f.id)?;
        Ok(Changed::parts(Parts::FIELDS))
    })
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct FieldFacets {
    key: String,
    /// Among the items this query finds; every item without one.
    #[serde(default)]
    query: String,
    /// all (default), uncategorized, untagged, recent, trash or missing.
    #[serde(default)]
    scope: Option<String>,
}

pub(super) fn field_facets(c: &Ctx, p: FieldFacets) -> ApiResult {
    field_of(c, &p.key)?;
    let ids = found(c, scope_of(p.scope.as_deref())?, &p.query, Sort::default(), false)?;
    Ok(read(c.host, |db| db.field_facets(&ids, &p.key))?)
}
