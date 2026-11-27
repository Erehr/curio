//! The items a search covers.

use crate::core::db::{Db, DbResult};
use crate::core::duplicates::Search;

/// The items a search reads: `mine` are those in its scope.
pub(super) struct Universe {
    pub(super) scoped: bool,
    pub(super) with_library: bool,
}

const BASE: &str = "a.deleted = 0 AND a.part_of IS NULL";

impl Universe {
    pub(super) fn new(db: &Db, s: &Search) -> DbResult<Universe> {
        db.conn.execute_batch("CREATE TEMP TABLE IF NOT EXISTS dup_scope(id INTEGER PRIMARY KEY); DELETE FROM temp.dup_scope;").map_err(|e| e.to_string())?;
        if let Some(ids) = &s.scope {
            db.conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
            let r = (|| {
                let mut st = db.conn.prepare_cached("INSERT OR IGNORE INTO temp.dup_scope(id) VALUES (?)").map_err(|e| e.to_string())?;
                for id in ids {
                    st.execute([id]).map_err(|e| e.to_string())?;
                }
                Ok::<_, String>(())
            })();
            db.conn.execute_batch(if r.is_ok() { "COMMIT" } else { "ROLLBACK" }).map_err(|e| e.to_string())?;
            r?;
        }
        Ok(Universe { scoped: s.scope.is_some(), with_library: s.with_library })
    }

    /// `1` for an item in scope.
    pub(super) fn mine(&self) -> &'static str {
        if self.scoped {
            "(a.id IN (SELECT id FROM temp.dup_scope))"
        } else {
            "1"
        }
    }

    /// The condition for the items read: not trashed, not grouped into another item, and in scope unless compared with the library.
    pub(super) fn filter(&self) -> String {
        if self.scoped && !self.with_library {
            format!("{BASE} AND a.id IN (SELECT id FROM temp.dup_scope)")
        } else {
            BASE.to_string()
        }
    }

    pub(super) fn count(&self, db: &Db, more: &str) -> DbResult<usize> {
        let sql = format!("SELECT count(*) FROM assets a WHERE {BASE} AND {}{more}", self.mine());
        db.conn.query_row(&sql, [], |r| r.get::<_, i64>(0)).map(|n| n as usize).map_err(|e| e.to_string())
    }
}
