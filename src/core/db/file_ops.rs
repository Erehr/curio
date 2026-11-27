//! The journal of the store's file operations, written before they run and replayed after a crash.

use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use std::collections::HashSet;
use rusqlite::params;
use crate::core::model::*;

impl Db {
    pub fn add_file_op(&self, op: &FileOp) -> DbResult<i64> {
        self.conn
            .execute(
                "INSERT INTO file_ops(kind, root_id, rel_path, tmp, asset_uid, size, mtime, state, created_at, dest, detail) VALUES(?, ?, ?, ?, uid_bytes(?), ?, ?, ?, ?, ?, ?)",
                params![op.kind, op.root_id, op.rel_path, op.tmp, op.asset_uid, op.size, op.mtime, op.state, now_ms(), op.dest, op.detail],
            )
            .map_err(e)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Up to `limit` ops of `kind` in `state` (all states when `None`), oldest first.
    pub fn file_ops(&self, kind: Option<&str>, state: Option<&str>, limit: i64) -> DbResult<Vec<FileOp>> {
        let mut st = self
            .conn
            .prepare_cached(
                "SELECT id, kind, root_id, rel_path, tmp, uid_text(asset_uid), size, mtime, state, error, created_at, dest, detail FROM file_ops WHERE (?1 IS NULL OR kind = ?1) AND (?2 IS NULL OR state = ?2) ORDER BY id LIMIT ?3",
            )
            .map_err(e)?;
        let rows = st
            .query_map(params![kind, state, limit], |r| {
                Ok(FileOp {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    root_id: r.get(2)?,
                    rel_path: r.get(3)?,
                    tmp: r.get(4)?,
                    asset_uid: r.get(5)?,
                    size: r.get(6)?,
                    mtime: r.get(7)?,
                    state: r.get(8)?,
                    error: r.get(9)?,
                    created_at: r.get(10)?,
                    dest: r.get(11)?,
                    detail: r.get(12)?,
                })
            })
            .map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Paths of a root with a recycle op not yet finished: purged copies that must not be taken for new files.
    pub fn recycle_paths(&self, root_id: i64) -> DbResult<HashSet<String>> {
        let mut st = self.conn.prepare_cached("SELECT rel_path FROM file_ops WHERE kind = 'recycle' AND root_id = ?").map_err(e)?;
        let rows = st.query_map([root_id], |r| r.get::<_, String>(0)).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn file_op_counts(&self) -> DbResult<(i64, i64)> {
        self.conn
            .query_row("SELECT COUNT(*) FILTER (WHERE state <> 'failed'), COUNT(*) FILTER (WHERE state = 'failed') FROM file_ops", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(e)
    }

    /// Marks every unfinished op failed with `why`, so nothing acts on it until the person retries it. Returns how many.
    pub fn set_aside_file_ops(&self, why: &str) -> DbResult<usize> {
        self.conn.execute("UPDATE file_ops SET state = 'failed', error = ? WHERE state <> 'failed'", [why]).map_err(e)
    }

    pub fn set_file_op_state(&self, id: i64, state: &str, error: Option<&str>) -> DbResult<()> {
        self.conn.execute("UPDATE file_ops SET state = ?, error = ? WHERE id = ?", params![state, error, id]).map_err(e)?;
        Ok(())
    }

    pub fn remove_file_op(&self, id: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM file_ops WHERE id = ?", [id]).map_err(e)?;
        Ok(())
    }
}
