//! What plugins keep in a library.

use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use std::collections::{HashMap, HashSet};
use rusqlite::{params, OptionalExtension};
use crate::core::model::*;

impl Db {
    pub fn plugin_state(&self, id: &str) -> Option<(bool, Vec<String>, i64)> {
        self.conn
            .query_row("SELECT enabled, granted, failures FROM plugin_state WHERE plugin_id = ?", [id], |r| {
                let g: String = r.get(1)?;
                Ok((r.get::<_, i64>(0)? != 0, serde_json::from_str(&g).unwrap_or_default(), r.get(2)?))
            })
            .optional()
            .ok()
            .flatten()
    }

    pub fn set_plugin_state(&self, id: &str, enabled: bool, version: &str, granted: &[String]) -> DbResult<()> {
        let g = serde_json::to_string(granted).map_err(e)?;
        self.conn
            .execute(
                "INSERT INTO plugin_state(plugin_id, enabled, version, granted, installed_at) VALUES(?, ?, ?, ?, ?)
                 ON CONFLICT(plugin_id) DO UPDATE SET enabled = excluded.enabled, version = excluded.version, granted = excluded.granted",
                params![id, enabled as i64, version, g, now_ms()],
            )
            .map_err(e)?;
        Ok(())
    }

    pub fn remove_plugin_state(&self, id: &str) -> DbResult<()> {
        self.conn.execute("DELETE FROM plugin_state WHERE plugin_id = ?", [id]).map_err(e)?;
        Ok(())
    }

    /// Drops saved state for plugins no longer present on disk so moved or removed plugins don't reappear with stale state.
    pub fn prune_plugin_states(&self, keep: &HashSet<&str>) -> DbResult<()> {
        let stored: Vec<String> = {
            let mut st = self.conn.prepare("SELECT plugin_id FROM plugin_state").map_err(e)?;
            let rows = st.query_map([], |r| r.get::<_, String>(0)).map_err(e)?;
            rows.filter_map(|r| r.ok()).collect()
        };
        for id in stored {
            if !keep.contains(id.as_str()) {
                self.conn.execute("DELETE FROM plugin_state WHERE plugin_id = ?", [&id]).map_err(e)?;
            }
        }
        Ok(())
    }

    /// A value a plugin keeps for this library (`plugin.state`).
    pub fn plugin_data(&self, plugin: &str, key: &str) -> DbResult<Option<serde_json::Value>> {
        let v: Option<String> = self.conn.query_row("SELECT value FROM plugin_data WHERE plugin_id = ? AND key = ?", [plugin, key], |r| r.get(0)).optional().map_err(e)?;
        Ok(v.and_then(|s| serde_json::from_str(&s).ok()))
    }

    /// `None` removes it.
    pub fn set_plugin_data(&self, plugin: &str, key: &str, value: Option<&serde_json::Value>) -> DbResult<()> {
        match value {
            Some(v) => self.conn.execute("INSERT INTO plugin_data(plugin_id, key, value) VALUES(?, ?, ?) ON CONFLICT(plugin_id, key) DO UPDATE SET value = excluded.value", params![plugin, key, v.to_string()]),
            None => self.conn.execute("DELETE FROM plugin_data WHERE plugin_id = ? AND key = ?", [plugin, key]),
        }
        .map_err(e)?;
        Ok(())
    }

    /// What a plugin keeps for each of these items (`plugin.itemState`); items with nothing kept are left out.
    pub fn plugin_item_data(&self, plugin: &str, ids: &[AssetId]) -> DbResult<HashMap<AssetId, serde_json::Value>> {
        let mut out = HashMap::new();
        let mut st = self.conn.prepare_cached("SELECT value FROM plugin_item_data WHERE plugin_id = ? AND asset_id = ?").map_err(e)?;
        for id in ids {
            let v: Option<String> = st.query_row(params![plugin, id], |r| r.get(0)).optional().map_err(e)?;
            if let Some(v) = v.and_then(|s| serde_json::from_str(&s).ok()) {
                out.insert(*id, v);
            }
        }
        Ok(out)
    }

    /// `None` removes it.
    pub fn set_plugin_item_data(&self, plugin: &str, id: AssetId, value: Option<&serde_json::Value>) -> DbResult<()> {
        match value {
            Some(v) => self.conn.execute("INSERT INTO plugin_item_data(plugin_id, asset_id, value) VALUES(?, ?, ?) ON CONFLICT(plugin_id, asset_id) DO UPDATE SET value = excluded.value", params![plugin, id, v.to_string()]),
            None => self.conn.execute("DELETE FROM plugin_item_data WHERE plugin_id = ? AND asset_id = ?", params![plugin, id]),
        }
        .map_err(e)?;
        Ok(())
    }

    pub fn plugin_failure(&self, id: &str, reset: bool) -> i64 {
        if reset {
            let _ = self.conn.execute("UPDATE plugin_state SET failures = 0 WHERE plugin_id = ?", [id]);
            return 0;
        }
        let _ = self.conn.execute("UPDATE plugin_state SET failures = failures + 1 WHERE plugin_id = ?", [id]);
        self.conn.query_row("SELECT failures FROM plugin_state WHERE plugin_id = ?", [id], |r| r.get(0)).unwrap_or(0)
    }
}
