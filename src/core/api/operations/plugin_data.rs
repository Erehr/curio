//! What plugins keep: their settings, their data beside items, their pages' storage.

use crate::core::api::operations::write;
use std::path::PathBuf;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::core::api::{read, ApiError, ApiResult, Ctx};
use crate::core::library::changes::Changed;
use crate::core::api::operations::done;
use crate::core::api::operations::libraries::Nothing;
use crate::core::api::operations::ItemRef;
use crate::core::api::operations::uids;
use crate::core::api::operations::ids_of;
use crate::core::api::operations::item_reads::Items;
use crate::core::api::operations::one_id;
use crate::core::api::operations::item_reads::OneItem;
use crate::core::api::operations::bad;

/// The calling plugin's id; `plugin.*` keeps each plugin's own.
pub(super) fn plugin_id(c: &Ctx) -> Result<String, ApiError> {
    c.caller.plugin.clone().ok_or_else(|| ApiError::new("not_allowed", "Only a plugin keeps state"))
}

const STATE_LIMIT: usize = 1 << 20;

const ITEM_STATE_LIMIT: usize = 64 << 10;

#[derive(Deserialize, JsonSchema)]
pub(super) struct StateKey {
    key: String,
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct StateSet {
    key: String,
    /// Any JSON; null removes the key.
    value: Value,
}

pub(super) fn plugin_state_get(c: &Ctx, p: StateKey) -> ApiResult {
    let id = plugin_id(c)?;
    Ok(json!({ "value": read(c.host, |db| db.plugin_data(&id, &p.key))?.unwrap_or(Value::Null) }))
}

pub(super) fn plugin_state_set(c: &Ctx, p: StateSet) -> ApiResult {
    let id = plugin_id(c)?;
    if p.key.is_empty() || p.key.len() > 200 {
        return Err(bad("`key` is 1 to 200 characters"));
    }
    if p.value.to_string().len() > STATE_LIMIT {
        return Err(bad("A value is kept up to 1 MB"));
    }
    write(c, move |db| {
        db.set_plugin_data(&id, &p.key, Some(&p.value).filter(|v| !v.is_null()))?;
        Ok(Changed::default())
    })
}

pub(super) fn plugin_item_state_get(c: &Ctx, p: OneItem) -> ApiResult {
    let (plugin, id) = (plugin_id(c)?, one_id(c, &p.id)?);
    Ok(json!({ "value": read(c.host, |db| db.plugin_item_data(&plugin, &[id]))?.remove(&id).unwrap_or(Value::Null) }))
}

pub(super) fn plugin_item_state_get_many(c: &Ctx, p: Items) -> ApiResult {
    let plugin = plugin_id(c)?;
    let ids = ids_of(c, &p.ids)?;
    let (kept, names) = read(c.host, |db| Ok((db.plugin_item_data(&plugin, &ids)?, uids(db, &ids)?)))?;
    let out: serde_json::Map<String, Value> = ids.iter().zip(names).filter_map(|(id, uid)| kept.get(id).map(|v| (uid, v.clone()))).collect();
    Ok(Value::Object(out))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct ItemStateSet {
    id: ItemRef,
    /// Any JSON; null removes it.
    value: Value,
}

pub(super) fn plugin_item_state_set(c: &Ctx, p: ItemStateSet) -> ApiResult {
    let (plugin, id) = (plugin_id(c)?, one_id(c, &p.id)?);
    if p.value.to_string().len() > ITEM_STATE_LIMIT {
        return Err(bad("A value is kept up to 64 KB per item"));
    }
    write(c, move |db| {
        db.set_plugin_item_data(&plugin, id, Some(&p.value).filter(|v| !v.is_null()))?;
        Ok(Changed::default())
    })
}

/// One file per plugin, named after its id.
fn settings_file(c: &Ctx, plugin: &str) -> PathBuf {
    let name: String = plugin.chars().map(|ch| if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') { ch } else { '_' }).collect();
    c.host.plugin_settings_dir().join(format!("{name}.json"))
}

fn read_settings(c: &Ctx, plugin: &str) -> serde_json::Map<String, Value> {
    std::fs::read_to_string(settings_file(c, plugin)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub(super) fn plugin_settings_get(c: &Ctx, _: Nothing) -> ApiResult {
    Ok(Value::Object(read_settings(c, &plugin_id(c)?)))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct SettingsSet {
    /// The settings to change; null removes one.
    values: serde_json::Map<String, Value>,
}

pub(super) fn plugin_settings_set(c: &Ctx, p: SettingsSet) -> ApiResult {
    let plugin = plugin_id(c)?;
    let file = settings_file(c, &plugin);
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d).map_err(|e| ApiError::new("failed", e.to_string()))?;
    }
    // every process holding a library shares these files, so one save at a time across all of them
    let lock = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(file.with_extension("json.lock")).map_err(|e| ApiError::new("failed", format!("Couldn't save the settings: {e}")))?;
    lock.lock().map_err(|e| ApiError::new("failed", format!("Couldn't save the settings: {e}")))?;
    let mut all = read_settings(c, &plugin);
    for (k, v) in p.values {
        if v.is_null() {
            all.remove(&k);
        } else {
            all.insert(k, v);
        }
    }
    let text = serde_json::to_string_pretty(&all).map_err(|e| ApiError::new("failed", e.to_string()))?;
    if text.len() > STATE_LIMIT {
        return Err(bad("A plugin's settings are kept up to 1 MB"));
    }
    let tmp = file.with_extension(format!("json.{}.partial", std::process::id()));
    std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, &file)).map_err(|e| ApiError::new("failed", format!("Couldn't save the settings: {e}")))?;
    drop(lock);
    Ok(Value::Object(all))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct LogLine {
    /// `info` (the default), `warn` or `error`.
    #[serde(default)]
    level: Option<String>,
    message: String,
}

pub(super) fn plugin_log(c: &Ctx, p: LogLine) -> ApiResult {
    let line = format!("plugin {}: {}", plugin_id(c)?, p.message.chars().take(4000).collect::<String>());
    match p.level.as_deref() {
        Some("error") => crate::core::system::logging::error(&line),
        Some("warn") => crate::core::system::logging::warn(&line),
        _ => crate::core::system::logging::info(&line),
    }
    Ok(done())
}
