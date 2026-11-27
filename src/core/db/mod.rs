//! SQLite persistence. One row per asset, incremental writes, WAL journaling, versioned
//! migrations. Every write touches only the rows it changes — renaming, tagging or rating one
//! asset never rewrites the library.

pub mod assets;
mod duplicates;
mod edits;
pub mod fields;
mod file_ops;
mod folders;
mod placements;
mod plugins;
mod reader_queue;
mod roots;
pub mod schema;
mod search;
mod smart_folders;
mod tags;
mod text_index;

use crate::core::db::schema::SCHEMA_VERSION;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::functions::FunctionFlags;
use rusqlite::types::{Value, ValueRef};
use rusqlite::{params, Connection, OptionalExtension};

use crate::core::images::color;
use crate::core::system::filesystem;
use crate::core::model::*;
use crate::core::db::schema::MIGRATIONS;

pub type DbResult<T> = Result<T, String>;

pub fn read_only_message(label: &str) -> String {
    format!("“{label}” is read-only in this library, so Curio doesn't write files there. Change it in Settings › Storage & backup.")
}

/// Queues every file entry of the trashed managed items for the Recycle Bin; `?1` is the time. Linked originals aren't ours to remove.
/// `sub`: folder `?1` and every folder below it.
const SUBTREE: &str = "WITH RECURSIVE sub(id) AS (SELECT ?1 UNION ALL SELECT f.id FROM folders f JOIN sub ON f.parent_id = sub.id)";

/// An item wholly in the trash, or with a folder entry there.
const IN_TRASH: &str = "a.deleted = 1 OR EXISTS (SELECT 1 FROM trashed_entries t WHERE t.asset_id = a.id)";

/// A row that is an item of its own, not a file grouped into another item; every query listing or counting items needs it.
pub const ITEMS: &str = "a.part_of IS NULL";

/// An `assets` row whose file is in the library's store: only those take a placement's path as their own.
const IN_STORE: &str = "root_id IN (SELECT id FROM roots WHERE kind = 'managed')";

/// Items in no folder, excluding full locations whose files already live in their own folders on disk.
const UNCATEGORIZED: &str = "NOT EXISTS (SELECT 1 FROM asset_folders f WHERE f.asset_id = a.id) AND a.root_id IN (SELECT id FROM roots WHERE kind = 'managed' OR explicit_only <> 0)";

const PURGE_OPS: &str = "INSERT INTO file_ops(kind, root_id, rel_path, asset_uid, size, mtime, created_at) SELECT 'recycle', a.root_id, COALESCE(p.rel_path, a.rel_path), a.uid, a.size, a.mtime, ?1 FROM assets a JOIN roots r ON r.id = a.root_id LEFT JOIN placements p ON p.asset_id = a.id WHERE a.deleted = 1 AND r.kind = 'managed'";

/// Remembers a purged linked file (`?1`, the item) so the scanner doesn't add it back. Roots of single added files never pick up new files, so they need no rows.
/// The item's filter is part of the WHERE here: anything appended would land after the upsert clause.
const PURGE_FORGET: &str = "INSERT INTO forgotten_files(root_id, rel_path, size, mtime) SELECT a.root_id, a.rel_path, a.size, a.mtime FROM assets a JOIN roots r ON r.id = a.root_id WHERE a.deleted = 1 AND r.kind = 'linked' AND r.explicit_only = 0 AND a.id = ?1 ON CONFLICT(root_id, rel_path) DO UPDATE SET size = excluded.size, mtime = excluded.mtime";

/// Queue for the Recycle Bin the store links of indexed items matching `filter` (on `a`, the item), which are about to be forgotten; their location files stay. Each op records the link's size and date as they are now.
fn queue_link_recycles(conn: &Connection, filter: &str) -> DbResult<()> {
    let store: Option<(i64, String)> = conn.query_row("SELECT id, path FROM roots WHERE kind = 'managed' ORDER BY id LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?))).optional().map_err(e)?;
    let Some((root, base)) = store else { return Ok(()) };
    let rows: Vec<(String, String)> = {
        let mut st = conn.prepare(&format!("SELECT p.rel_path, uid_text(a.uid) FROM placements p JOIN assets a ON a.id = p.asset_id JOIN roots r ON r.id = a.root_id WHERE r.kind <> 'managed' AND {filter}")).map_err(e)?;
        let q = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
        q.filter_map(|r| r.ok()).collect()
    };
    let now = now_ms();
    for (rel, uid) in rows {
        let st = filesystem::stat(&filesystem::db_to_abs(&base, &rel)).ok();
        conn.execute(
            "INSERT INTO file_ops(kind, root_id, rel_path, asset_uid, size, mtime, created_at) VALUES('recycle', ?, ?, uid_bytes(?), ?, ?, ?)",
            params![root, rel, uid, st.as_ref().map(|s| s.size as i64), st.as_ref().and_then(|s| s.mtime), now],
        )
        .map_err(e)?;
    }
    Ok(())
}

fn e<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

/// Why a field can't be filtered, sorted, searched or written.
pub fn not_indexed(key: &str) -> String {
    format!("Field '{key}' isn't indexed (Settings › Fields)")
}

/// Bits a look's 16-bit part may differ by when its neighbours are looked up through the indexes: 697 values a part at 3.
const LOOK_PART_RADIUS: u32 = 3;

pub struct Db {
    pub conn: Connection,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Counts {
    pub all: i64,
    pub favorites: i64,
    pub uncategorized: i64,
    pub untagged: i64,
    pub trash: i64,
    pub missing: i64,
    pub recent: i64,
    pub tags: i64,
    /// Bytes of every file not in the trash.
    pub size: i64,
}

impl Db {
    pub fn open(path: &Path) -> DbResult<Db> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(e)?;
        }
        let conn = Connection::open(path).map_err(e)?;
        Self::setup(conn, path)
    }

    /// Open without running migrations, for checking a file that may not be a live library
    /// (a backup about to be restored). Refuses a database whose floor is above this build.
    pub fn open_unmigrated(path: &Path) -> DbResult<Db> {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(e)?;
        conn.busy_timeout(std::time::Duration::from_millis(8000)).map_err(e)?;
        register_functions(&conn).map_err(e)?;
        let db = Db { conn, path: path.to_path_buf() };
        db.check_floor().map_err(|s| s.text())?;
        Ok(db)
    }

    /// Check whether a library file on disk requires a newer version of Curio, without modifying it.
    pub fn floor_refusal(path: &Path) -> Option<crate::core::said::Said> {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
        let db = Db { conn, path: path.to_path_buf() };
        db.check_floor().err()
    }

    pub fn schema_floor(&self) -> i64 {
        self.meta_get("schema_floor").and_then(|s| s.parse().ok()).unwrap_or(1)
    }

    pub fn check_floor(&self) -> Result<(), crate::core::said::Said> {
        let current = self.schema_version();
        if current <= SCHEMA_VERSION || self.schema_floor() <= SCHEMA_VERSION {
            return Ok(());
        }
        Err(match self.meta_get("schema_floor_app").filter(|s| !s.trim().is_empty()) {
            Some(ver) => crate::core::said::Said::new("errors.library.needsVersion", serde_json::json!({"version": ver}), format!("This library needs Curio {ver} or newer.")),
            None => crate::core::said::Said::new("errors.library.tooNew", serde_json::Value::Null, "This library was saved by a newer version of Curio."),
        })
    }

    fn setup(conn: Connection, path: &Path) -> DbResult<Db> {
        conn.busy_timeout(std::time::Duration::from_millis(8000)).map_err(e)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;
             PRAGMA cache_size = -65536;
             PRAGMA mmap_size = 268435456;",
        )
        .map_err(e)?;
        register_functions(&conn).map_err(e)?;
        let mut db = Db { conn, path: path.to_path_buf() };
        db.migrate()?;
        Ok(db)
    }

    /// Apply pending migrations inside one transaction each; the user_version is the source of truth.
    pub fn migrate(&mut self) -> DbResult<()> {
        let current: i64 = self.conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(e)?;
        if current > SCHEMA_VERSION {
            self.check_floor().map_err(|s| s.text())?;
            return Ok(());
        }
        // only the first connection of an open sees pending migrations; later ones skip this
        if current > 0 && current < SCHEMA_VERSION && self.path != Path::new(":memory:") {
            let dir = self.pre_migration_backup(current)?;
            crate::core::system::logging::info(&format!("backed up the library before migrating from schema {current}: {}", dir.display()));
        }
        for (i, m) in MIGRATIONS.iter().enumerate().skip(current as usize) {
            let ver = (i + 1) as i64;
            let tx = self.conn.transaction().map_err(e)?;
            tx.execute_batch(m.sql).map_err(|err| format!("migration v{ver} failed: {err}"))?;
            if m.floor == schema::Floor::Raise {
                tx.execute("INSERT INTO meta(key, value) VALUES('schema_floor', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value", [ver.to_string()]).map_err(e)?;
                tx.execute("INSERT INTO meta(key, value) VALUES('schema_floor_app', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value", [env!("CURIO_VERSION")]).map_err(e)?;
            }
            tx.execute_batch(&format!("PRAGMA user_version = {ver}")).map_err(e)?;
            tx.commit().map_err(e)?;
        }
        Ok(())
    }

    /// Run idempotent catch-up steps for migrations newer than the build that last wrote to this library.
    pub fn catch_up_if_needed(&mut self) -> DbResult<()> {
        let prev: i64 = self.meta_get("written_by").and_then(|s| s.parse().ok()).unwrap_or(0);
        if prev <= 0 || prev >= SCHEMA_VERSION {
            return Ok(());
        }
        let applied = (self.schema_version().min(SCHEMA_VERSION) as usize).min(MIGRATIONS.len());
        let mut refresh_fts = false;
        let mut requeue_meta = false;
        let mut requeue_thumbs = false;
        for m in &MIGRATIONS[prev as usize..applied] {
            for step in m.catch_up {
                match step {
                    schema::CatchUp::Sql(sql) => self.tx(|db| db.conn.execute_batch(sql).map_err(e))?,
                    schema::CatchUp::RefreshFts => refresh_fts = true,
                    schema::CatchUp::RequeueMeta => requeue_meta = true,
                    schema::CatchUp::RequeueThumbs => requeue_thumbs = true,
                }
            }
        }
        if refresh_fts {
            let ids: Vec<AssetId> = {
                let mut st = self.conn.prepare("SELECT id FROM assets").map_err(e)?;
                let rows = st.query_map([], |r| r.get(0)).map_err(e)?;
                rows.filter_map(|r| r.ok()).collect()
            };
            for chunk in ids.chunks(500) {
                self.tx(|db| db.refresh_fts(chunk))?;
            }
        }
        if requeue_meta {
            self.conn.execute("UPDATE assets SET meta_state = 0 WHERE status = 0 AND part_of IS NULL", []).map_err(e)?;
        }
        if requeue_thumbs {
            self.conn.execute("UPDATE assets SET thumb_state = CASE WHEN thumb_state = 0 THEN 0 ELSE 3 END, thumb_ver = thumb_ver + 1 WHERE status = 0 AND part_of IS NULL", []).map_err(e)?;
        }
        Ok(())
    }

    /// Copy the database to `backups/pre-migration-v<from>-<stamp>/library.db` next to it and
    /// check the copy. Any failure stops the migration, so the library stays on its old schema.
    fn pre_migration_backup(&self, from: i64) -> DbResult<PathBuf> {
        let fail = |what: String| format!("Couldn't back up the library before upgrading it, so it was left unchanged: {what}");
        let base = self.path.parent().unwrap_or(Path::new(".")).join("backups");
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let mut dir = base.join(format!("pre-migration-v{from}-{stamp}"));
        let mut n = 1;
        while dir.exists() {
            n += 1;
            dir = base.join(format!("pre-migration-v{from}-{stamp}-{n}"));
        }
        let dest = dir.join("library.db");
        self.backup_to(&dest).map_err(fail)?;
        let copy = Db::open_unmigrated(&dest).map_err(fail)?;
        let ok = copy.integrity_check(true).map_err(fail)?;
        if ok != vec!["ok".to_string()] {
            return Err(fail(format!("the copy failed its check ({})", ok.join("; "))));
        }
        if copy.schema_version() != from {
            return Err(fail(format!("the copy has schema {} instead of {from}", copy.schema_version())));
        }
        Ok(dir)
    }

    pub fn schema_version(&self) -> i64 {
        self.conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap_or(0)
    }

    // ------------------------------------------------------------ maintenance

    pub fn integrity_check(&self, quick: bool) -> DbResult<Vec<String>> {
        let sql = if quick { "PRAGMA quick_check" } else { "PRAGMA integrity_check" };
        let mut st = self.conn.prepare(sql).map_err(e)?;
        let rows = st.query_map([], |r| r.get::<_, String>(0)).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Background optimisation: refresh planner stats, merge FTS segments, checkpoint WAL.
    pub fn optimize(&self) -> DbResult<()> {
        self.conn
            .execute_batch(
                "PRAGMA optimize;
                 INSERT INTO asset_fts(asset_fts) VALUES('optimize');
                 PRAGMA wal_checkpoint(TRUNCATE);",
            )
            .map_err(e)?;
        Ok(())
    }

    /// Consistent online backup into a single file.
    pub fn backup_to(&self, dest: &Path) -> DbResult<()> {
        if dest.exists() {
            return Err(format!("{} already exists", dest.display()));
        }
        if let Some(p) = dest.parent() {
            std::fs::create_dir_all(p).map_err(e)?;
        }
        self.conn.execute("VACUUM INTO ?", [dest.to_string_lossy().to_string()]).map_err(e)?;
        Ok(())
    }

    pub fn meta_get(&self, key: &str) -> Option<String> {
        self.conn.query_row("SELECT value FROM meta WHERE key = ?", [key], |r| r.get(0)).optional().ok().flatten()
    }

    pub fn meta_set(&self, key: &str, value: &str) -> DbResult<()> {
        self.conn
            .execute("INSERT INTO meta(key, value) VALUES(?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value", [key, value])
            .map_err(e)?;
        Ok(())
    }

    // ------------------------------------------------------------ transactions helper

    /// After a panic inside a transaction, so the connection can be used again.
    pub fn rollback_open(&self) {
        if !self.conn.is_autocommit() {
            let _ = self.conn.execute_batch("ROLLBACK");
        }
    }

    pub fn tx<T>(&mut self, f: impl FnOnce(&Db) -> DbResult<T>) -> DbResult<T> {
        self.in_tx(|db| f(db))
    }

    /// All or nothing. Inside another transaction this becomes a savepoint, so it nests (the writer runs every edit in one).
    pub fn in_tx<T>(&mut self, f: impl FnOnce(&mut Db) -> DbResult<T>) -> DbResult<T> {
        let outer = self.conn.is_autocommit();
        self.conn.execute_batch(if outer { "BEGIN IMMEDIATE" } else { "SAVEPOINT tx" }).map_err(e)?;
        let res = f(self).and_then(|v| self.conn.execute_batch(if outer { "COMMIT" } else { "RELEASE tx" }).map(|_| v).map_err(e));
        if res.is_err() {
            let _ = self.conn.execute_batch(if outer { "ROLLBACK" } else { "ROLLBACK TO tx; RELEASE tx" });
        }
        res
    }
}

/// Key that two store paths share when they differ only in case: such names can't coexist on Windows, nor in a library moved between systems.
/// Case-folded key of a path, one char for one char: a char whose lower case has another length (Turkish İ) stays as it is, so prefixes and lengths of a path and its key line up.
pub fn fold_path(rel: &str) -> String {
    rel.chars()
        .map(|c| {
            let mut low = c.to_lowercase();
            match (low.next(), low.next()) {
                (Some(l), None) if l.len_utf8() == c.len_utf8() => l,
                _ => c,
            }
        })
        .collect()
}

fn register_functions(conn: &Connection) -> rusqlite::Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC;
    conn.create_scalar_function("fold_path", 1, flags, |ctx| Ok(fold_path(&ctx.get::<String>(0)?)))?;
    conn.create_scalar_function("name_fits", 2, flags, |ctx| {
        let (rel, name) = (ctx.get::<String>(0)?, ctx.get::<String>(1)?);
        let base = rel.rsplit_once('/').map(|(_, b)| b).unwrap_or(&rel);
        Ok(crate::core::store::is_variant(base, &crate::core::store::clean_name(&name, false)))
    })?;
    conn.create_scalar_function("regexp", 2, flags, |ctx| {
        let pat: String = ctx.get(0)?;
        let text: Option<String> = ctx.get(1)?;
        let Some(text) = text else { return Ok(false) };
        thread_local! {
            static CACHE: std::cell::RefCell<HashMap<String, Option<regex::Regex>>> = std::cell::RefCell::new(HashMap::new());
        }
        let m = CACHE.with(|c| {
            let mut c = c.borrow_mut();
            if c.len() > 64 {
                c.clear();
            }
            let re = c.entry(pat.clone()).or_insert_with(|| regex::RegexBuilder::new(&pat).case_insensitive(true).build().ok());
            re.as_ref().map(|r| r.is_match(&text)).unwrap_or(false)
        });
        Ok(m)
    })?;
    conn.create_scalar_function("color_similar", 3, flags, |ctx| {
        let pal: Option<String> = ctx.get(0)?;
        let packed: i64 = ctx.get(1)?;
        let limit: f64 = ctx.get(2)?;
        let Some(pal) = pal else { return Ok(false) };
        let p = color::palette_from_json(&pal);
        let target = [((packed >> 16) & 255) as u8, ((packed >> 8) & 255) as u8, (packed & 255) as u8];
        Ok(color::palette_matches(&p, target, limit))
    })?;
    conn.create_scalar_function("color_grayscale", 1, flags, |ctx| {
        let pal: Option<String> = ctx.get(0)?;
        Ok(pal.map(|p| color::palette_grayscale(&color::palette_from_json(&p))).unwrap_or(false))
    })?;
    // ids are kept as the 16 bytes of a UUID and read as its text form
    conn.create_scalar_function("uid_text", 1, flags, |ctx| {
        Ok(match ctx.get_raw(0) {
            ValueRef::Blob(b) => Value::Text(crate::core::model::uid_text(b).unwrap_or_else(|| b.iter().map(|x| format!("{x:02x}")).collect())),
            ValueRef::Text(t) => Value::Text(String::from_utf8_lossy(t).into_owned()),
            _ => Value::Null,
        })
    })?;
    conn.create_scalar_function("uid_bytes", 1, flags, |ctx| {
        Ok(match ctx.get_raw(0) {
            ValueRef::Text(t) => {
                let s = String::from_utf8_lossy(t);
                crate::core::model::uid_bytes(&s).map(|b| Value::Blob(b.to_vec())).unwrap_or_else(|| Value::Text(s.into_owned()))
            }
            ValueRef::Blob(b) => Value::Blob(b.to_vec()),
            _ => Value::Null,
        })
    })?;
    Ok(())
}
