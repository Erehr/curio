//! Items as the scanner writes them and as the window reads them back.

use crate::core::db::ITEMS;
use crate::core::db::UNCATEGORIZED;
use crate::core::db::Counts;
use crate::core::db::e;
use crate::core::db::DbResult;
use crate::core::db::Db;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use rusqlite::{params, params_from_iter, OptionalExtension};
use crate::core::images::color;
use crate::core::system::filesystem;
use crate::core::duplicates::fingerprints::{self, Look};
use crate::core::model::*;
use crate::core::db::search::not_in_roots;

const ASSET_COLS: &str = "a.id, uid_text(a.uid), a.root_id, a.rel_path, a.name, a.display_name, a.ext, a.mime, a.kind, a.size, \
    a.width, a.height, a.aspect, a.duration, a.ctime, a.mtime, a.added_at, a.modified_at, a.rating, a.annotation, \
    a.source_url, a.palette, a.quick_hash, a.content_hash, a.status, a.deleted, a.thumb_state, a.thumb_ver, r.path, \
    a.part_of, a.part_role, (SELECT count(*) FROM assets m WHERE m.part_of = a.id), a.favorite, a.explicit";

/// Rows inserted or updated by the scanner.
#[derive(Clone, Debug)]
pub struct ScannedFile {
    pub rel_path: String,
    pub name: String,
    pub ext: String,
    pub size: i64,
    pub mtime: Option<i64>,
    pub ctime: Option<i64>,
    pub file_key: Option<String>,
    pub quick_hash: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration: Option<f64>,
    pub animated: Option<bool>,
}

fn aspect_of(w: Option<i64>, h: Option<i64>) -> f64 {
    match (w, h) {
        (Some(w), Some(h)) if w > 0 && h > 0 => w as f64 / h as f64,
        _ => 1.0,
    }
}

impl Db {
    /// Known files of a root: rel_path → (id, size, mtime, file_key, status).
    pub fn root_index(&self, root_id: i64) -> DbResult<HashMap<String, (AssetId, i64, Option<i64>, Option<String>, i64)>> {
        let mut st = self
            .conn
            .prepare("SELECT rel_path, id, size, mtime, file_key, status FROM assets WHERE root_id = ?")
            .map_err(e)?;
        let rows = st
            .query_map([root_id], |r| Ok((r.get::<_, String>(0)?, (r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))))
            .map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn insert_scanned(&self, root_id: i64, f: &ScannedFile) -> DbResult<AssetId> {
        let now = now_ms();
        // added back, by the user or because it changed: no longer forgotten
        self.conn.prepare_cached("DELETE FROM forgotten_files WHERE root_id = ? AND rel_path = ?").and_then(|mut st| st.execute(params![root_id, f.rel_path])).map_err(e)?;
        let kind = kind_for_ext(&f.ext);
        let aspect = aspect_of(f.width, f.height);
        self.conn
            .execute(
                "INSERT INTO assets(uid, root_id, rel_path, name, ext, mime, kind, size, width, height, aspect, duration,
                    ctime, mtime, added_at, modified_at, quick_hash, file_key, animated, status)
                 VALUES(uid_bytes(?),?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,0)",
                params![
                    new_uid(),
                    root_id,
                    f.rel_path,
                    f.name,
                    f.ext,
                    mime_for_ext(&f.ext),
                    kind.as_str(),
                    f.size,
                    f.width,
                    f.height,
                    aspect,
                    f.duration,
                    f.ctime,
                    f.mtime,
                    now,
                    now,
                    f.quick_hash,
                    f.file_key,
                    f.animated.map(|b| b as i64)
                ],
            )
            .map_err(e)?;
        let id = self.conn.last_insert_rowid();
        self.refresh_fts(&[id])?;
        Ok(id)
    }

    /// File content changed in place: keep identity and user metadata, invalidate derived data.
    pub fn update_scanned(&self, id: AssetId, f: &ScannedFile) -> DbResult<()> {
        let aspect = match (f.width, f.height) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some(w as f64 / h as f64),
            _ => None,
        };
        self.conn
            .execute(
                "UPDATE assets SET size = ?, mtime = ?, ctime = COALESCE(?, ctime), width = COALESCE(?, width),
                    height = COALESCE(?, height), aspect = COALESCE(?, aspect), duration = COALESCE(?, duration), quick_hash = ?,
                    file_key = COALESCE(?, file_key), animated = ?, content_hash = NULL, status = 0,
                    thumb_state = CASE WHEN thumb_state = 0 THEN 0 ELSE 3 END, thumb_ver = thumb_ver + 1, meta_state = 0
                 WHERE id = ?",
                params![f.size, f.mtime, f.ctime, f.width, f.height, aspect, f.duration, f.quick_hash, f.file_key, f.animated.map(|b| b as i64), id],
            )
            .map_err(e)?;
        Ok(())
    }

    /// A format plugin's thumbnails made again: every one (`all`), or those that couldn't be made. Answers how many files.
    pub fn remake_thumbs(&self, exts: &[String], all: bool) -> DbResult<usize> {
        if exts.is_empty() {
            return Ok(0);
        }
        let list = vec!["?"; exts.len()].join(", ");
        let set = if all { "thumb_state = CASE WHEN thumb_state = 0 THEN 0 ELSE 3 END, thumb_ver = thumb_ver + 1" } else { "thumb_state = 0" };
        let only = if all { "" } else { " AND thumb_state = 2" };
        self.conn.execute(&format!("UPDATE assets SET {set} WHERE deleted = 0 AND part_of IS NULL{only} AND ext IN ({list})"), params_from_iter(exts.iter())).map_err(e)
    }

    /// Asset found at a new place (moved/renamed). Identity, tags and fields are preserved.
    pub fn move_asset(&self, id: AssetId, root_id: i64, rel_path: &str, name: &str) -> DbResult<()> {
        let ext = filesystem::ext_of(Path::new(name));
        self.conn
            .execute(
                "UPDATE assets SET root_id = ?, rel_path = ?, name = ?, ext = ?, status = 0 WHERE id = ?",
                params![root_id, rel_path, name, ext, id],
            )
            .map_err(e)?;
        self.refresh_fts(&[id])?;
        Ok(())
    }

    pub fn set_status(&self, ids: &[AssetId], status: Availability) -> DbResult<()> {
        let mut st = self.conn.prepare_cached("UPDATE assets SET status = ? WHERE id = ?").map_err(e)?;
        for id in ids {
            st.execute(params![status as i64, id]).map_err(e)?;
        }
        Ok(())
    }

    /// A missing item a new file may be, moved: by its file key, else by its quick hash and size.
    pub fn find_move_candidate(&self, file_key: Option<&str>, quick_hash: Option<&str>, size: i64) -> Option<AssetId> {
        self.find_move_candidates(file_key, quick_hash, size).into_iter().next()
    }

    /// Every missing item a new file may be, best first, so one already claimed by another file doesn't hide the rest.
    pub fn find_move_candidates(&self, file_key: Option<&str>, quick_hash: Option<&str>, size: i64) -> Vec<AssetId> {
        let mut out: Vec<AssetId> = vec![];
        let mut take = |sql: &str, p: &[&dyn rusqlite::ToSql]| {
            if let Ok(mut st) = self.conn.prepare_cached(sql) {
                if let Ok(rows) = st.query_map(p, |r| r.get::<_, AssetId>(0)) {
                    out.extend(rows.filter_map(|r| r.ok()));
                }
            }
        };
        if let Some(k) = file_key {
            // file systems reuse identities, so a new file can carry a deleted file's key
            take("SELECT id FROM assets WHERE file_key = ?1 AND status = 2 AND deleted = 0 AND size = ?2 AND (?3 IS NULL OR quick_hash IS NULL OR quick_hash = ?3) LIMIT 64", &[&k, &size, &quick_hash]);
        }
        if let Some(h) = quick_hash {
            take("SELECT id FROM assets WHERE quick_hash = ?1 AND size = ?2 AND status = 2 AND deleted = 0 LIMIT 64", &[&h, &size]);
        }
        let mut seen = HashSet::new();
        out.retain(|id| seen.insert(*id));
        out
    }

    pub fn set_thumb_done(
        &self,
        id: AssetId,
        ok: bool,
        dims: Option<(i64, i64)>,
        palette: Option<&[PaletteColor]>,
        look: Option<&Look>,
    ) -> DbResult<()> {
        let pal = palette.map(color::palette_to_json);
        let (w, h) = dims.map(|(w, h)| (Some(w), Some(h))).unwrap_or((None, None));
        self.conn
            .execute(
                "UPDATE assets SET thumb_state = ?, width = COALESCE(width, ?), height = COALESCE(height, ?),
                   aspect = CASE WHEN width IS NULL AND ? IS NOT NULL THEN ? * 1.0 / ? ELSE aspect END,
                   palette = COALESCE(?, palette) WHERE id = ?",
                params![if ok { 1 } else { 2 }, w, h, w, w, h.unwrap_or(1).max(1), pal, id],
            )
            .map_err(e)?;
        if let Some(l) = look {
            self.set_look(id, l)?;
        }
        Ok(())
    }

    /// The look of the item's current thumbnail.
    pub fn set_look(&self, id: AssetId, l: &Look) -> DbResult<()> {
        self.conn
            .prepare_cached("INSERT INTO fingerprints(asset_id, kind, ver, src, size, key, data) SELECT id, ?, ?, thumb_ver, 0, ?, ? FROM assets WHERE id = ? ON CONFLICT(asset_id, kind) DO UPDATE SET ver = excluded.ver, src = excluded.src, size = excluded.size, key = excluded.key, data = excluded.data")
            .map_err(e)?
            .execute(params![fingerprints::LOOK, fingerprints::version(fingerprints::LOOK), l.key(), l.data(), id])
            .map_err(e)?;
        Ok(())
    }

    /// A look made from the thumbnail of version `ver`, kept only while that is still the item's thumbnail.
    pub fn set_look_at(&self, id: AssetId, l: &Look, ver: i64) -> DbResult<()> {
        self.conn
            .prepare_cached("INSERT INTO fingerprints(asset_id, kind, ver, src, size, key, data) SELECT id, ?, ?, thumb_ver, 0, ?, ? FROM assets WHERE id = ? AND thumb_ver = ? ON CONFLICT(asset_id, kind) DO UPDATE SET ver = excluded.ver, src = excluded.src, size = excluded.size, key = excluded.key, data = excluded.data")
            .map_err(e)?
            .execute(params![fingerprints::LOOK, fingerprints::version(fingerprints::LOOK), l.key(), l.data(), id, ver])
            .map_err(e)?;
        Ok(())
    }

    /// A print of a file, made from the file as it was at `mtime` and `size`.
    pub fn set_print(&self, id: AssetId, kind: i64, mtime: i64, size: i64, key: i64, data: &[u8]) -> DbResult<()> {
        self.conn
            .prepare_cached("INSERT INTO fingerprints(asset_id, kind, ver, src, size, key, data) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(asset_id, kind) DO UPDATE SET ver = excluded.ver, src = excluded.src, size = excluded.size, key = excluded.key, data = excluded.data")
            .map_err(e)?
            .execute(params![id, kind, fingerprints::version(kind), mtime, size, key, data])
            .map_err(e)?;
        Ok(())
    }

    pub fn set_meta_done(&self, id: AssetId) -> DbResult<()> {
        self.conn.execute("UPDATE assets SET meta_state = 1 WHERE id = ?", [id]).map_err(e)?;
        Ok(())
    }

    /// The thumbnail is left to a plugin's page (thumb_state 5), which the window runs for `page_jobs`.
    pub fn set_thumb_page(&self, id: AssetId) -> DbResult<()> {
        self.conn.execute("UPDATE assets SET thumb_state = 5 WHERE id = ?", params![id]).map_err(e)?;
        Ok(())
    }

    /// Items waiting for a plugin page's thumbnail: id, uid and extension, excluding `skip`.
    /// How many items wait for a plugin page's thumbnail, as `page_jobs` hands them out.
    pub fn page_jobs_left(&self) -> DbResult<i64> {
        self.conn.query_row("SELECT count(*) FROM assets WHERE thumb_state = 5 AND status = 0 AND deleted = 0 AND part_of IS NULL", [], |r| r.get(0)).map_err(e)
    }

    pub fn page_jobs(&self, limit: i64, skip: &[AssetId]) -> DbResult<Vec<(AssetId, String, String)>> {
        if skip.is_empty() {
            let mut st = self.conn.prepare_cached("SELECT id, uid_text(uid), ext FROM assets WHERE thumb_state = 5 AND status = 0 AND deleted = 0 AND part_of IS NULL ORDER BY added_at DESC LIMIT ?").map_err(e)?;
            let rows = st.query_map(params![limit], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(e)?;
            return rows.collect::<Result<Vec<_>, _>>().map_err(e);
        }
        let skip = &skip[..skip.len().min(500)];
        let sql = format!("SELECT id, uid_text(uid), ext FROM assets WHERE thumb_state = 5 AND status = 0 AND deleted = 0 AND part_of IS NULL AND id NOT IN ({}) ORDER BY added_at DESC LIMIT ?", vec!["?"; skip.len()].join(", "));
        let mut p: Vec<rusqlite::types::Value> = skip.iter().map(|i| rusqlite::types::Value::Integer(*i)).collect();
        p.push(rusqlite::types::Value::Integer(limit));
        let mut st = self.conn.prepare(&sql).map_err(e)?;
        let rows = st.query_map(rusqlite::params_from_iter(p.iter()), |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(e)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(e)
    }

    /// After plugins load: items waiting for a page that makes no thumbnails for them now, and failed ones of the extensions `paged` lists, go back in the queue.
    /// So do all of an extension a page makes thumbnails for since the last load, whose thumbnails came from elsewhere: the system's, before the plugin was there or had loaded.
    pub fn requeue_page_thumbs(&self, paged: &[String]) -> DbResult<usize> {
        let mut now: Vec<String> = paged.iter().map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect();
        now.sort();
        now.dedup();
        let prev_raw = self.meta_get("page_thumb_exts").unwrap_or_default();
        let before: Vec<String> = prev_raw.split(',').map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect();
        let new: Vec<String> = now.iter().filter(|x| !before.contains(x)).cloned().collect();
        let mut clauses = Vec::new();
        let mut values: Vec<rusqlite::types::Value> = Vec::new();
        if now.is_empty() {
            clauses.push("thumb_state = 5".to_string());
        } else {
            let ph = vec!["?"; now.len()].join(", ");
            clauses.push(format!("(thumb_state = 5 AND ext NOT IN ({ph}))"));
            values.extend(now.iter().cloned().map(rusqlite::types::Value::Text));
            clauses.push(format!("(thumb_state = 2 AND ext IN ({ph}))"));
            values.extend(now.iter().cloned().map(rusqlite::types::Value::Text));
        }
        if !new.is_empty() {
            let ph = vec!["?"; new.len()].join(", ");
            clauses.push(format!("(thumb_state = 1 AND ext IN ({ph}))"));
            values.extend(new.iter().cloned().map(rusqlite::types::Value::Text));
        }
        let sql = format!("UPDATE assets SET thumb_ver = thumb_ver + (thumb_state = 1), thumb_state = 3 WHERE part_of IS NULL AND ({})", clauses.join(" OR "));
        let n = self.conn.execute(&sql, params_from_iter(values)).map_err(e)?;
        let joined = now.join(",");
        if joined != prev_raw {
            self.meta_set("page_thumb_exts", &joined)?;
        }
        Ok(n)
    }

    /// The thumbnail waits for FFmpeg (thumb_state 4).
    pub fn set_thumb_waiting(&self, id: AssetId) -> DbResult<()> {
        self.conn.execute("UPDATE assets SET thumb_state = 4 WHERE id = ?", params![id]).map_err(e)?;
        Ok(())
    }

    /// Thumbnails that waited for FFmpeg are made again, and the lengths and encodings of those files read.
    pub fn requeue_waiting_thumbs(&self) -> DbResult<usize> {
        self.conn.execute("UPDATE assets SET meta_state = 0 WHERE thumb_state = 4 AND kind IN ('video', 'audio') AND part_of IS NULL", []).map_err(e)?;
        self.conn.execute("UPDATE assets SET thumb_state = 0 WHERE thumb_state = 4 AND part_of IS NULL", []).map_err(e)
    }

    pub fn set_duration(&self, id: AssetId, duration: f64) -> DbResult<()> {
        self.conn.execute("UPDATE assets SET duration = ? WHERE id = ?", params![duration, id]).map_err(e)?;
        Ok(())
    }

    pub fn set_dims(&self, id: AssetId, w: i64, h: i64) -> DbResult<()> {
        self.conn
            .execute("UPDATE assets SET width = ?, height = ?, aspect = ? WHERE id = ?", params![w, h, aspect_of(Some(w), Some(h)), id])
            .map_err(e)?;
        Ok(())
    }

    pub fn fill_quick_hash(&self, id: AssetId, hash: &str) -> DbResult<()> {
        self.conn.prepare_cached("UPDATE assets SET quick_hash = ? WHERE id = ? AND quick_hash IS NULL").map_err(e)?.execute(params![hash, id]).map_err(e)?;
        Ok(())
    }

    pub fn set_content_hash(&self, id: AssetId, hash: &str) -> DbResult<()> {
        self.conn.execute("UPDATE assets SET content_hash = ? WHERE id = ?", params![hash, id]).map_err(e)?;
        Ok(())
    }

    /// A hash worked out away from the writer, kept only while the item's file is still the one that was read.
    pub fn set_content_hash_if(&self, id: AssetId, hash: &str, size: i64, mtime: Option<i64>) -> DbResult<()> {
        self.conn.prepare_cached("UPDATE assets SET content_hash = ? WHERE id = ? AND size = ? AND mtime IS ?").map_err(e)?.execute(params![hash, id, size, mtime]).map_err(e)?;
        Ok(())
    }

    /// Next batch of assets that need thumbnails (new or invalidated).
    /// Items waiting for a thumbnail, newest first, but those in `skip_roots`.
    pub fn pending_thumbs(&self, limit: usize, exclude: &HashSet<AssetId>, skip_roots: &[i64]) -> DbResult<Vec<AssetId>> {
        let sql = format!("SELECT id FROM assets WHERE thumb_state IN (0, 3) AND status = 0 AND deleted = 0 AND part_of IS NULL{} ORDER BY added_at DESC LIMIT ?", not_in_roots(skip_roots));
        let mut st = self.conn.prepare_cached(&sql).map_err(e)?;
        let rows = st.query_map([(limit + exclude.len()) as i64], |r| r.get::<_, i64>(0)).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).filter(|id| !exclude.contains(id)).take(limit).collect())
    }

    fn row_to_asset(r: &rusqlite::Row) -> rusqlite::Result<Asset> {
        let root_path: String = r.get(28)?;
        let rel: String = r.get(3)?;
        let pal: Option<String> = r.get(21)?;
        Ok(Asset {
            id: r.get(0)?,
            uid: r.get(1)?,
            root_id: r.get(2)?,
            abs_path: filesystem::db_to_abs(&root_path, &rel).to_string_lossy().to_string(),
            rel_path: rel,
            name: r.get(4)?,
            display_name: r.get(5)?,
            ext: r.get(6)?,
            mime: r.get(7)?,
            kind: AssetKind::parse(&r.get::<_, String>(8)?),
            size: r.get(9)?,
            width: r.get(10)?,
            height: r.get(11)?,
            aspect: r.get::<_, f64>(12)? as f32,
            duration: r.get(13)?,
            ctime: r.get(14)?,
            mtime: r.get(15)?,
            added_at: r.get(16)?,
            modified_at: r.get(17)?,
            rating: r.get(18)?,
            favorite: r.get::<_, i64>(32)? != 0,
            explicit: r.get::<_, i64>(33)? != 0,
            annotation: r.get(19)?,
            source_url: r.get(20)?,
            palette: pal.map(|p| color::palette_from_json(&p)).unwrap_or_default(),
            quick_hash: r.get(22)?,
            content_hash: r.get(23)?,
            status: Availability::from_i64(r.get(24)?),
            deleted: r.get::<_, i64>(25)? != 0,
            thumb_state: r.get(26)?,
            thumb_ver: r.get(27)?,
            part_of: r.get(29)?,
            part_role: r.get(30)?,
            parts: r.get(31)?,
        })
    }

    pub fn asset(&self, id: AssetId) -> DbResult<Option<Asset>> {
        self.conn
            .query_row(
                &format!("SELECT {ASSET_COLS} FROM assets a JOIN roots r ON r.id = a.root_id WHERE a.id = ?"),
                [id],
                Self::row_to_asset,
            )
            .optional()
            .map_err(e)
    }

    pub fn assets(&self, ids: &[AssetId]) -> DbResult<Vec<Asset>> {
        let mut out = Vec::with_capacity(ids.len());
        for chunk in ids.chunks(500) {
            let ph = vec!["?"; chunk.len()].join(",");
            let mut st = self
                .conn
                .prepare(&format!("SELECT {ASSET_COLS} FROM assets a JOIN roots r ON r.id = a.root_id WHERE a.id IN ({ph})"))
                .map_err(e)?;
            let rows = st.query_map(params_from_iter(chunk.iter()), Self::row_to_asset).map_err(e)?;
            out.extend(rows.filter_map(|r| r.ok()));
        }
        Ok(out)
    }

    pub fn abs_path(&self, id: AssetId) -> Option<PathBuf> {
        let mut st = self.conn.prepare_cached("SELECT r.path, a.rel_path FROM assets a JOIN roots r ON r.id = a.root_id WHERE a.id = ?").ok()?;
        st.query_row([id], |r| Ok(filesystem::db_to_abs(&r.get::<_, String>(0)?, &r.get::<_, String>(1)?))).optional().ok().flatten()
    }

    /// Batched absolute paths for `ids` in chunks of 500.
    pub fn abs_paths(&self, ids: &[AssetId]) -> HashMap<AssetId, PathBuf> {
        let mut out = HashMap::with_capacity(ids.len());
        for chunk in ids.chunks(500) {
            let ph = vec!["?"; chunk.len()].join(",");
            let Ok(mut st) = self.conn.prepare(&format!("SELECT a.id, r.path, a.rel_path FROM assets a JOIN roots r ON r.id = a.root_id WHERE a.id IN ({ph})")) else { continue };
            let Ok(rows) = st.query_map(params_from_iter(chunk.iter()), |r| Ok((r.get::<_, AssetId>(0)?, filesystem::db_to_abs(&r.get::<_, String>(1)?, &r.get::<_, String>(2)?)))) else { continue };
            for (id, p) in rows.filter_map(|r| r.ok()) {
                out.insert(id, p);
            }
        }
        out
    }

    /// Light rows for visible grid cells only.
    pub fn grid_items(&self, ids: &[AssetId]) -> DbResult<Vec<GridItem>> {
        let mut out = Vec::with_capacity(ids.len());
        for chunk in ids.chunks(500) {
            let ph = vec!["?"; chunk.len()].join(",");
            let mut st = self
                .conn
                .prepare(&format!(
                    "SELECT a.id, uid_text(a.uid), COALESCE(NULLIF(a.display_name,''), a.name), a.ext, a.kind, a.width, a.height, a.aspect, a.size,
                            a.rating, a.status, a.thumb_state, a.thumb_ver, a.mtime, COALESCE(a.animated, 0),
                            (SELECT group_concat(t.name, ', ') FROM (SELECT t.name FROM asset_tags x JOIN tags t ON t.id = x.tag_id
                               WHERE x.asset_id = a.id LIMIT 4) t),
                            COALESCE(a.rotation, 0), r.access = 'read_only', r.kind = 'managed', r.access = 'organize', substr(COALESCE(a.annotation, ''), 1, 200),
                            (SELECT count(*) FROM assets m WHERE m.part_of = a.id), (SELECT m.part_role FROM assets m WHERE m.part_of = a.id ORDER BY CASE m.part_role WHEN 'motion' THEN 0 WHEN 'raw' THEN 1 WHEN 'jpeg' THEN 1 ELSE 2 END, m.id LIMIT 1), a.solo,
                            (SELECT m.id FROM assets m WHERE m.part_of = a.id AND m.part_role = 'motion' AND m.status = 0 ORDER BY m.id LIMIT 1),
                            (SELECT m.id FROM assets m WHERE m.part_of = a.id AND m.part_role IN ('raw', 'jpeg') AND m.status = 0 ORDER BY m.id LIMIT 1),
                            a.favorite, a.explicit
                     FROM assets a LEFT JOIN roots r ON r.id = a.root_id WHERE a.id IN ({ph})"
                ))
                .map_err(e)?;
            let rows = st
                .query_map(params_from_iter(chunk.iter()), |r| {
                    Ok(GridItem {
                        id: r.get(0)?,
                        uid: r.get(1)?,
                        title: r.get(2)?,
                        ext: r.get(3)?,
                        kind: AssetKind::parse(&r.get::<_, String>(4)?),
                        width: r.get(5)?,
                        height: r.get(6)?,
                        aspect: r.get::<_, f64>(7)? as f32,
                        size: r.get(8)?,
                        rating: r.get(9)?,
                        favorite: r.get::<_, i64>(26)? != 0,
                        explicit: r.get::<_, i64>(27)? != 0,
                        status: Availability::from_i64(r.get(10)?),
                        thumb_state: r.get(11)?,
                        thumb_ver: r.get(12)?,
                        mtime: r.get(13)?,
                        animated: r.get::<_, i64>(14)? != 0,
                        tags_preview: r.get::<_, Option<String>>(15)?.unwrap_or_default(),
                        rotation: r.get(16)?,
                        read_only: r.get::<_, Option<bool>>(17)?.unwrap_or(false),
                        managed: r.get::<_, Option<bool>>(18)?.unwrap_or(false),
                        organize: r.get::<_, Option<bool>>(19)?.unwrap_or(false),
                        note: r.get(20)?,
                        parts: r.get(21)?,
                        part_role: r.get(22)?,
                        solo: r.get::<_, i64>(23)? != 0,
                        motion: r.get(24)?,
                        pair: r.get(25)?,
                    })
                })
                .map_err(e)?;
            out.extend(rows.filter_map(|r| r.ok()));
        }
        Ok(out)
    }

    /// The palettes of those of `ids` that have one.
    pub fn palettes_of(&self, ids: &[AssetId]) -> DbResult<HashMap<AssetId, Vec<PaletteColor>>> {
        let mut out = HashMap::new();
        for chunk in ids.chunks(500) {
            let ph = vec!["?"; chunk.len()].join(",");
            let mut st = self.conn.prepare(&format!("SELECT id, palette FROM assets WHERE id IN ({ph}) AND palette IS NOT NULL AND palette <> '[]'")).map_err(e)?;
            let rows = st.query_map(params_from_iter(chunk.iter()), |r| Ok((r.get::<_, AssetId>(0)?, r.get::<_, String>(1)?))).map_err(e)?;
            for (id, p) in rows.filter_map(|r| r.ok()) {
                out.insert(id, color::palette_from_json(&p));
            }
        }
        Ok(out)
    }

    /// The items that have a note; few have one, so a set of them is cheaper than asking per item.
    pub fn noted_ids(&self) -> DbResult<HashSet<AssetId>> {
        let mut st = self.conn.prepare_cached("SELECT id FROM assets WHERE annotation <> ''").map_err(e)?;
        let rows = st.query_map([], |r| r.get::<_, AssetId>(0)).map_err(e)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn counts(&self) -> DbResult<Counts> {
        let q = |sql: &str| -> i64 { self.conn.query_row(sql, [], |r| r.get(0)).unwrap_or(0) };
        Ok(Counts {
            all: q(&format!("SELECT count(*) FROM assets a WHERE a.deleted = 0 AND {ITEMS}")),
            favorites: q(&format!("SELECT count(*) FROM assets a WHERE a.deleted = 0 AND {ITEMS} AND a.favorite = 1")),
            uncategorized: q(&format!("SELECT count(*) FROM assets a WHERE a.deleted = 0 AND {ITEMS} AND {UNCATEGORIZED}")),
            untagged: q(&format!("SELECT count(*) FROM assets a WHERE a.deleted = 0 AND {ITEMS} AND NOT EXISTS (SELECT 1 FROM asset_tags t WHERE t.asset_id = a.id)")),
            trash: q(&format!("SELECT count(*) FROM (SELECT id FROM assets a WHERE {ITEMS} AND a.deleted = 1 UNION SELECT a.id FROM trashed_entries te JOIN assets a ON a.id = te.asset_id WHERE {ITEMS})")),
            missing: q(&format!("SELECT count(*) FROM assets a WHERE a.deleted = 0 AND {ITEMS} AND a.status > 0")),
            recent: q(&format!("SELECT count(*) FROM assets a WHERE a.deleted = 0 AND {ITEMS} AND a.last_used IS NOT NULL")),
            tags: q("SELECT count(*) FROM tags"),
            size: q("SELECT COALESCE(sum(size), 0) FROM assets WHERE deleted = 0"),
        })
    }
}
