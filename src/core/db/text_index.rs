//! The full-text index kept beside the items.

use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use rusqlite::{params, OptionalExtension};
use crate::core::model::*;

impl Db {
    /// Rebuild the FTS row of each asset from its current data. Cheap per asset.
    pub fn refresh_fts(&self, ids: &[AssetId]) -> DbResult<()> {
        let mut del = self.conn.prepare_cached("DELETE FROM asset_fts WHERE rowid = ?").map_err(e)?;
        let mut get = self
            .conn
            .prepare_cached(
                "SELECT a.name || ' ' || a.display_name, a.annotation, a.source_url,
                   (SELECT group_concat(t.name, ' ') FROM asset_tags x JOIN tags t ON t.id = x.tag_id WHERE x.asset_id = a.id),
                   (SELECT group_concat(fv.text_value, ' ') FROM field_values fv JOIN fields f ON f.id = fv.field_id
                      WHERE fv.asset_id = a.id AND fv.text_value IS NOT NULL AND f.type IN ('text', 'single_select', 'multi_select', 'url') AND json_extract(f.def, '$.indexed') IS NOT 0)
                 FROM assets a WHERE a.id = ?",
            )
            .map_err(e)?;
        let mut ins = self
            .conn
            .prepare_cached("INSERT INTO asset_fts(rowid, name, annotation, url, tags, fields) VALUES(?, ?, ?, ?, ?, ?)")
            .map_err(e)?;
        for id in ids {
            del.execute([id]).map_err(e)?;
            let row: Option<(String, String, String, Option<String>, Option<String>)> = get
                .query_row([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
                .optional()
                .map_err(e)?;
            if let Some((name, ann, url, tags, fields)) = row {
                // split name into words so "IMG_0042.png" matches "img" and "0042"
                let name_words: String = name.chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect();
                ins.execute(params![id, format!("{name} {name_words}"), ann, url, tags.unwrap_or_default(), fields.unwrap_or_default()])
                    .map_err(e)?;
            }
        }
        Ok(())
    }
}
