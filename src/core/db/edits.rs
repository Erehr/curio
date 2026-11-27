//! Edits to what a person sets on items: names, ratings, notes, colors and the like.

use crate::core::db::queue_link_recycles;
use crate::core::db::PURGE_FORGET;
use crate::core::db::PURGE_OPS;
use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use std::path::Path;
use rusqlite::params;
use crate::core::system::filesystem;
use crate::core::model::*;

impl Db {
    /// Record a metadata edit (`modified_at`); an edit isn't a use, so imports and bulk edits leave Recently used alone.
    pub fn touch(&self, ids: &[AssetId]) -> DbResult<()> {
        let now = now_ms();
        let mut st = self.conn.prepare_cached("UPDATE assets SET modified_at = ? WHERE id = ?").map_err(e)?;
        for id in ids {
            st.execute(params![now, id]).map_err(e)?;
        }
        Ok(())
    }

    /// Record an opening (`last_used`), in the viewer or another program, for Recently used.
    pub fn mark_used(&self, id: AssetId) -> DbResult<()> {
        self.conn.execute("UPDATE assets SET last_used = ? WHERE id = ?", params![now_ms(), id]).map_err(e)?;
        Ok(())
    }

    pub fn set_rating(&self, ids: &[AssetId], rating: i64) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("UPDATE assets SET rating = ? WHERE id = ?").map_err(e)?;
        for id in ids {
            st.execute(params![rating.clamp(0, 5), id]).map_err(e)?;
        }
        self.touch(ids)
    }

    pub fn set_favorite(&self, ids: &[AssetId], on: bool) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("UPDATE assets SET favorite = ? WHERE id = ?").map_err(e)?;
        for id in ids {
            st.execute(params![on as i64, id]).map_err(e)?;
        }
        self.touch(ids)
    }

    pub fn set_explicit(&self, ids: &[AssetId], on: bool) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("UPDATE assets SET explicit = ? WHERE id = ?").map_err(e)?;
        for id in ids {
            st.execute(params![on as i64, id]).map_err(e)?;
        }
        self.touch(ids)
    }

    /// Rename an item. A copied item, or one in a location set to organize files, changes its file name (its extension stays; its files are renamed to match); another linked item only shows the new name, unless it equals the file's.
    pub fn rename_item(&self, id: AssetId, value: &str) -> DbResult<()> {
        let (name, ext, managed): (String, String, bool) = self
            .conn
            .query_row("SELECT a.name, a.ext, r.kind = 'managed' OR r.access = 'organize' FROM assets a JOIN roots r ON r.id = a.root_id WHERE a.id = ?", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|_| format!("no item {id}"))?;
        let value = value.trim();
        if !managed {
            return self.set_display_name(id, if value == name { "" } else { value });
        }
        let has_ext = !ext.is_empty() && value.to_lowercase().ends_with(&format!(".{ext}"));
        let file = if has_ext || ext.is_empty() { value.to_string() } else { format!("{value}.{ext}") };
        if file.trim_end_matches(&format!(".{ext}")).trim().is_empty() {
            return Err("A name can't be empty".into());
        }
        self.conn.execute("UPDATE assets SET name = ?, display_name = '' WHERE id = ?", params![file, id]).map_err(e)?;
        self.touch(&[id])?;
        self.refresh_fts(&[id])
    }

    /// The item's file was renamed outside Curio: its name and type follow the file.
    pub fn set_file_name(&self, id: AssetId, file: &str) -> DbResult<()> {
        let ext = filesystem::ext_of(Path::new(file));
        self.conn.execute("UPDATE assets SET name = ?, ext = ?, display_name = '' WHERE id = ?", params![file, ext, id]).map_err(e)?;
        self.refresh_fts(&[id])
    }

    /// Writing a file next to an item (subtitles, sidecars): refused in read-only locations, and for copied items, whose folders in the store hold only the items themselves.
    pub fn writes_beside(&self, root_id: i64) -> DbResult<()> {
        self.require_writable(root_id)?;
        let managed = self.conn.query_row("SELECT kind = 'managed' FROM roots WHERE id = ?", [root_id], |r| r.get::<_, bool>(0)).unwrap_or(false);
        if managed {
            return Err("Sidecars can't be written for items copied into the library.".into());
        }
        Ok(())
    }

    pub fn set_display_name(&self, id: AssetId, name: &str) -> DbResult<()> {
        self.conn.execute("UPDATE assets SET display_name = ? WHERE id = ?", params![name, id]).map_err(e)?;
        self.touch(&[id])?;
        self.refresh_fts(&[id])
    }

    pub fn set_annotation(&self, ids: &[AssetId], text: &str) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("UPDATE assets SET annotation = ? WHERE id = ?").map_err(e)?;
        for id in ids {
            st.execute(params![text, id]).map_err(e)?;
        }
        self.touch(ids)?;
        self.refresh_fts(ids)
    }

    pub fn set_source_url(&self, ids: &[AssetId], url: &str) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("UPDATE assets SET source_url = ? WHERE id = ?").map_err(e)?;
        for id in ids {
            st.execute(params![url, id]).map_err(e)?;
        }
        self.touch(ids)?;
        self.refresh_fts(ids)
    }

    /// Trash is a flag in the index. Files on disk are never deleted by the app.
    pub fn set_deleted(&self, ids: &[AssetId], deleted: bool) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("UPDATE assets SET deleted = ? WHERE id = ?").map_err(e)?;
        for id in ids {
            st.execute(params![deleted as i64, id]).map_err(e)?;
        }
        Ok(())
    }

    /// Forget these items if they are in the trash, and the folder entries of theirs that are. Anything else is left alone. The library's own copies of forgotten items are queued in `file_ops` for the Recycle Bin; a trashed entry's link goes once the store follows.
    pub fn purge_items(&mut self, ids: &[AssetId]) -> DbResult<usize> {
        // the files grouped into an item go with it; listed before any row goes, as a gone item lets go of its members
        let ids = &self.with_members(ids)?;
        let tx = self.conn.savepoint().map_err(e)?;
        let mut n = 0;
        {
            let mut op = tx.prepare_cached(&format!("{PURGE_OPS} AND a.id = ?2")).map_err(e)?;
            let mut forget = tx.prepare_cached(PURGE_FORGET).map_err(e)?;
            let mut fts = tx.prepare_cached("DELETE FROM asset_fts WHERE rowid = ?1 AND EXISTS (SELECT 1 FROM assets WHERE id = ?1 AND deleted = 1)").map_err(e)?;
            let mut row = tx.prepare_cached("DELETE FROM assets WHERE id = ? AND deleted = 1").map_err(e)?;
            let now = now_ms();
            for id in ids {
                queue_link_recycles(&tx, &format!("a.deleted = 1 AND a.id = {id}"))?;
                op.execute(params![now, id]).map_err(e)?;
                forget.execute([id]).map_err(e)?;
                fts.execute([id]).map_err(e)?;
                n += row.execute([id]).map_err(e)?;
            }
            // items still here only had folder entries in the trash
            let mut entries = tx.prepare_cached("DELETE FROM trashed_entries WHERE asset_id = ?").map_err(e)?;
            for id in ids {
                if entries.execute([id]).map_err(e)? > 0 {
                    n += 1;
                }
            }
        }
        tx.commit().map_err(e)?;
        Ok(n)
    }

}
