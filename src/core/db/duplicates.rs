//! Duplicate groups as found and as settled.

use crate::core::db::LOOK_PART_RADIUS;
use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use std::collections::HashSet;
use rusqlite::OptionalExtension;
use crate::core::duplicates::fingerprints::{self, Look};
use crate::core::model::*;

impl Db {
    /// The oldest link, not in the trash, to `url`.
    pub fn link_with_address(&self, url: &str) -> DbResult<Option<AssetId>> {
        self.conn.query_row("SELECT id FROM assets WHERE deleted = 0 AND part_of IS NULL AND ext = 'url' AND source_url = ? ORDER BY id LIMIT 1", [url], |r| r.get(0)).optional().map_err(e)
    }

    /// Items not in the trash with a file of this size.
    pub fn ids_of_size(&self, size: i64) -> DbResult<Vec<AssetId>> {
        let mut st = self.conn.prepare("SELECT id FROM assets WHERE deleted = 0 AND part_of IS NULL AND size = ?").map_err(e)?;
        let rows = st.query_map([size], |r| r.get(0)).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// The looks of items not in the trash, made from their current thumbnails, with their shapes.
    pub fn looks(&self) -> DbResult<Vec<(AssetId, f32, Look)>> {
        self.looks_where("", &[])
    }

    fn looks_where(&self, more: &str, args: &[&dyn rusqlite::ToSql]) -> DbResult<Vec<(AssetId, f32, Look)>> {
        let sql = format!("SELECT f.asset_id, a.aspect, f.key, f.data FROM fingerprints f JOIN assets a ON a.id = f.asset_id WHERE f.kind = {} AND f.ver = {} AND f.src = a.thumb_ver AND a.thumb_state = 1 AND a.deleted = 0 AND a.part_of IS NULL{more}", fingerprints::LOOK, fingerprints::version(fingerprints::LOOK));
        let mut st = self.conn.prepare_cached(&sql).map_err(e)?;
        let rows = st.query_map(args, |r| Ok((r.get::<_, AssetId>(0)?, r.get::<_, f64>(1)? as f32, r.get::<_, i64>(2)?, r.get::<_, Vec<u8>>(3)?))).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).filter_map(|(id, aspect, key, data)| Look::from_row(key, &data).map(|l| (id, aspect, l))).collect())
    }

    /// One item's current look and shape.
    pub fn look_of(&self, id: AssetId) -> DbResult<Option<(f32, Look)>> {
        Ok(self.looks_where(" AND f.asset_id = ?", &[&id])?.into_iter().next().map(|(_, a, l)| (a, l)))
    }

    /// Looks whose DCT hash is within `max_bits` of this one, found through the four parts' indexes; None when that many bits would ask for too many parts, and reading every look is quicker.
    pub fn looks_near(&self, dct: u64, max_bits: u32) -> DbResult<Option<Vec<(AssetId, f32, Look)>>> {
        let within = max_bits / 4;
        if within > LOOK_PART_RADIUS {
            return Ok(None);
        }
        let masks: Vec<u64> = (0u64..1 << 16).filter(|m| m.count_ones() <= within).collect();
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for part in 0..4u32 {
            let mine = (dct >> (16 * part)) & 0xFFFF;
            let values: Vec<i64> = masks.iter().map(|m| (mine ^ m) as i64).collect();
            let expr = if part == 0 { "f.key & 65535".to_string() } else { format!("(f.key >> {}) & 65535", 16 * part) };
            let marks = vec!["?"; values.len()].join(",");
            let args: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
            for row in self.looks_where(&format!(" AND {expr} IN ({marks})"), &args)? {
                if fingerprints::bits_apart(row.2.dct, dct) <= max_bits && seen.insert(row.0) {
                    out.push(row);
                }
            }
        }
        Ok(Some(out))
    }
}
