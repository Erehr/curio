//! The database's schema: its migrations in order, each declaring whether an older build may still open the library and what a newer build catches up after an older one writes.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(super) enum Floor {
    Keep,
    Raise,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(super) enum CatchUp {
    Sql(&'static str),
    RefreshFts,
    RequeueMeta,
    RequeueThumbs,
}

pub(super) struct Migration {
    pub(super) sql: &'static str,
    pub(super) floor: Floor,
    pub(super) catch_up: &'static [CatchUp],
}

/// Ordered list of schema migrations. Never edit a shipped entry: append a new one.
pub(super) const MIGRATIONS: &[Migration] = &[
    // v1 — the schema; ids are the 16 bytes of a UUIDv7
    Migration {
        floor: Floor::Keep,
        catch_up: &[],
        sql: r#"
    CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
    CREATE TABLE roots (
        id INTEGER PRIMARY KEY,
        uid BLOB NOT NULL UNIQUE,
        path TEXT NOT NULL UNIQUE,
        kind TEXT NOT NULL DEFAULT 'linked',
        label TEXT NOT NULL DEFAULT '',
        volume TEXT NOT NULL DEFAULT '',
        enabled INTEGER NOT NULL DEFAULT 1,
        watch INTEGER NOT NULL DEFAULT 1,
        explicit_only INTEGER NOT NULL DEFAULT 0,
        online INTEGER NOT NULL DEFAULT 1,
        last_scan INTEGER,
        access TEXT NOT NULL DEFAULT 'full',
        sort_order INTEGER NOT NULL DEFAULT 0,
        icon TEXT,
        color TEXT
    );
    CREATE TABLE assets (
        id INTEGER PRIMARY KEY,
        uid BLOB NOT NULL UNIQUE,
        root_id INTEGER NOT NULL REFERENCES roots(id),
        rel_path TEXT NOT NULL,
        name TEXT NOT NULL,
        display_name TEXT NOT NULL DEFAULT '',
        ext TEXT NOT NULL DEFAULT '',
        mime TEXT NOT NULL DEFAULT '',
        kind TEXT NOT NULL DEFAULT 'other',
        size INTEGER NOT NULL DEFAULT 0,
        width INTEGER,
        height INTEGER,
        aspect REAL NOT NULL DEFAULT 1.0,
        duration REAL,
        ctime INTEGER,
        mtime INTEGER,
        added_at INTEGER NOT NULL,
        modified_at INTEGER NOT NULL,
        last_used INTEGER,
        rating INTEGER NOT NULL DEFAULT 0,
        favorite INTEGER NOT NULL DEFAULT 0,
        explicit INTEGER NOT NULL DEFAULT 0,
        annotation TEXT NOT NULL DEFAULT '',
        source_url TEXT NOT NULL DEFAULT '',
        palette TEXT,
        quick_hash TEXT,
        content_hash TEXT,
        file_key TEXT,
        status INTEGER NOT NULL DEFAULT 0,
        deleted INTEGER NOT NULL DEFAULT 0,
        thumb_state INTEGER NOT NULL DEFAULT 0,
        thumb_ver INTEGER NOT NULL DEFAULT 0,
        meta_state INTEGER NOT NULL DEFAULT 0,
        animated INTEGER,
        rotation INTEGER NOT NULL DEFAULT 0,
        part_of INTEGER REFERENCES assets(id) ON DELETE SET NULL,
        part_role TEXT,
        part_rule TEXT,
        part_suffix TEXT,
        part_saved TEXT,
        solo INTEGER NOT NULL DEFAULT 0,
        part_cover INTEGER NOT NULL DEFAULT 0,
        UNIQUE (root_id, rel_path)
    );
    CREATE TABLE folders (
        id INTEGER PRIMARY KEY,
        uid BLOB NOT NULL UNIQUE,
        parent_id INTEGER REFERENCES folders(id),
        name TEXT NOT NULL,
        color TEXT,
        description TEXT NOT NULL DEFAULT '',
        sort_order INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL,
        icon TEXT,
        auto_tags TEXT,
        dir TEXT
    );
    CREATE TABLE asset_folders ( folder_id INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE, asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE, position REAL NOT NULL DEFAULT 0, PRIMARY KEY (folder_id, asset_id) ) WITHOUT ROWID;
    CREATE TABLE tag_groups (
        id INTEGER PRIMARY KEY,
        uid BLOB NOT NULL UNIQUE,
        name TEXT NOT NULL,
        color TEXT,
        sort_order INTEGER NOT NULL DEFAULT 0,
        icon TEXT,
        description TEXT
    );
    CREATE TABLE tags (
        id INTEGER PRIMARY KEY,
        uid BLOB NOT NULL UNIQUE,
        name TEXT NOT NULL UNIQUE COLLATE NOCASE,
        group_id INTEGER REFERENCES tag_groups(id) ON DELETE SET NULL,
        color TEXT,
        starred INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE asset_tags ( tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE, asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE, PRIMARY KEY (tag_id, asset_id) ) WITHOUT ROWID;
    CREATE TABLE tag_group_members (
        tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
        group_id INTEGER NOT NULL REFERENCES tag_groups(id) ON DELETE CASCADE,
        PRIMARY KEY (tag_id, group_id)
    );
    CREATE TABLE fields (
        id INTEGER PRIMARY KEY,
        uid BLOB NOT NULL UNIQUE,
        key TEXT NOT NULL UNIQUE COLLATE NOCASE,
        def TEXT NOT NULL,
        type TEXT NOT NULL,
        owner TEXT NOT NULL DEFAULT 'user',
        schema_version INTEGER NOT NULL DEFAULT 1,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    );
    CREATE TABLE field_values ( asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE, field_id INTEGER NOT NULL REFERENCES fields(id) ON DELETE CASCADE, idx INTEGER NOT NULL DEFAULT 0, text_value TEXT, num_value REAL, PRIMARY KEY (asset_id, field_id, idx) ) WITHOUT ROWID;
    CREATE TABLE smart_folders (
        id INTEGER PRIMARY KEY,
        uid BLOB NOT NULL UNIQUE,
        parent_id INTEGER,
        name TEXT NOT NULL,
        query TEXT NOT NULL,
        sort TEXT,
        created_at INTEGER NOT NULL,
        icon TEXT,
        group_by TEXT,
        sort_order INTEGER NOT NULL DEFAULT 0
    );
    CREATE VIRTUAL TABLE asset_fts USING fts5( name, annotation, url, tags, fields, content='', contentless_delete=1, tokenize='unicode61 remove_diacritics 2' );
    CREATE TABLE plugin_state (
        plugin_id TEXT PRIMARY KEY,
        enabled INTEGER NOT NULL DEFAULT 1,
        version TEXT NOT NULL DEFAULT '',
        granted TEXT NOT NULL DEFAULT '[]',
        failures INTEGER NOT NULL DEFAULT 0,
        installed_at INTEGER NOT NULL
    );
    CREATE TABLE search_history ( query TEXT PRIMARY KEY, used_at INTEGER NOT NULL, uses INTEGER NOT NULL DEFAULT 1 );
    CREATE TABLE auto_import_state (
        path TEXT PRIMARY KEY,
        size INTEGER NOT NULL,
        mtime INTEGER NOT NULL,
        stable_since INTEGER NOT NULL,
        imported INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE file_ops (
        id INTEGER PRIMARY KEY,
        kind TEXT NOT NULL,
        root_id INTEGER NOT NULL,
        rel_path TEXT NOT NULL,
        tmp TEXT,
        asset_uid BLOB,
        size INTEGER,
        mtime INTEGER,
        state TEXT NOT NULL DEFAULT 'pending',
        error TEXT,
        created_at INTEGER NOT NULL,
        dest TEXT,
        detail TEXT
    );
    CREATE TABLE forgotten_files ( root_id INTEGER NOT NULL REFERENCES roots(id) ON DELETE CASCADE, rel_path TEXT NOT NULL, size INTEGER NOT NULL, mtime INTEGER, PRIMARY KEY (root_id, rel_path) ) WITHOUT ROWID;
    CREATE TABLE placements (
        id INTEGER PRIMARY KEY,
        asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
        folder_id INTEGER REFERENCES folders(id),
        rel_path TEXT NOT NULL,
        rel_key TEXT NOT NULL UNIQUE,
        linked INTEGER NOT NULL DEFAULT 1
    );
    CREATE TABLE trashed_entries ( asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE, folder_id INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE, position REAL NOT NULL DEFAULT 0, trashed_at INTEGER NOT NULL, PRIMARY KEY (asset_id, folder_id) ) WITHOUT ROWID;
    CREATE TABLE store_dirty (asset_id INTEGER PRIMARY KEY);
    CREATE TABLE trashed_folders ( asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE, path TEXT NOT NULL, PRIMARY KEY (asset_id, path) ) WITHOUT ROWID;
    CREATE TABLE store_gone_dirs (dir TEXT PRIMARY KEY) WITHOUT ROWID;
    CREATE TABLE plugin_data ( plugin_id TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY (plugin_id, key) ) WITHOUT ROWID;
    CREATE TABLE plugin_item_data ( plugin_id TEXT NOT NULL, asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE, value TEXT NOT NULL, PRIMARY KEY (plugin_id, asset_id) ) WITHOUT ROWID;
    CREATE TABLE meta_queue (asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE, reader TEXT NOT NULL, mode INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (asset_id, reader)) WITHOUT ROWID;
    CREATE TABLE meta_errors (asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE, reader TEXT NOT NULL, error TEXT NOT NULL, at INTEGER NOT NULL, PRIMARY KEY (asset_id, reader)) WITHOUT ROWID;
    CREATE TABLE field_overrides (asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE, field_id INTEGER NOT NULL REFERENCES fields(id) ON DELETE CASCADE, PRIMARY KEY (asset_id, field_id)) WITHOUT ROWID;
    CREATE TABLE fingerprints ( asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE, kind INTEGER NOT NULL, ver INTEGER NOT NULL, src INTEGER NOT NULL, size INTEGER NOT NULL DEFAULT 0, key INTEGER NOT NULL, data BLOB NOT NULL, PRIMARY KEY (asset_id, kind) ) WITHOUT ROWID;
    CREATE TABLE dup_ignored ( a INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE, b INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE, PRIMARY KEY (a, b) ) WITHOUT ROWID;
    CREATE INDEX idx_assets_name ON assets(name COLLATE NOCASE);
    CREATE INDEX idx_assets_ext ON assets(ext);
    CREATE INDEX idx_assets_kind ON assets(kind);
    CREATE INDEX idx_assets_size ON assets(size);
    CREATE INDEX idx_assets_mtime ON assets(mtime);
    CREATE INDEX idx_assets_ctime ON assets(ctime);
    CREATE INDEX idx_assets_rating ON assets(rating);
    CREATE INDEX idx_assets_favorite ON assets(favorite) WHERE favorite = 1;
    CREATE INDEX idx_assets_explicit ON assets(explicit) WHERE explicit = 1;
    CREATE INDEX idx_assets_dims ON assets(width, height);
    CREATE INDEX idx_assets_qhash ON assets(quick_hash);
    CREATE INDEX idx_assets_noted ON assets(id) WHERE annotation <> '';
    CREATE INDEX idx_assets_chash ON assets(content_hash);
    CREATE INDEX idx_assets_fkey ON assets(file_key);
    CREATE INDEX idx_assets_status ON assets(status);
    CREATE INDEX idx_assets_thumb ON assets(thumb_state);
    CREATE INDEX idx_assets_used ON assets(last_used);
    CREATE INDEX idx_folders_parent ON folders(parent_id);
    CREATE INDEX idx_af_asset ON asset_folders(asset_id);
    CREATE INDEX idx_at_asset ON asset_tags(asset_id);
    CREATE INDEX idx_fv_num ON field_values(field_id, num_value);
    CREATE INDEX idx_fv_text ON field_values(field_id, text_value COLLATE NOCASE);
    CREATE INDEX idx_sh_used ON search_history(used_at);
    CREATE INDEX idx_pl_asset ON placements(asset_id);
    CREATE INDEX idx_pl_folder ON placements(folder_id);
    CREATE UNIQUE INDEX idx_pl_asset_folder ON placements(asset_id, folder_id) WHERE folder_id IS NOT NULL;
    CREATE INDEX idx_te_folder ON trashed_entries(folder_id);
    CREATE INDEX idx_plugin_item_asset ON plugin_item_data(asset_id);
    CREATE INDEX idx_meta_queue_reader ON meta_queue(reader);
    CREATE INDEX idx_field_overrides_field ON field_overrides(field_id);
    CREATE INDEX idx_assets_part ON assets(part_of) WHERE part_of IS NOT NULL;
    CREATE INDEX idx_assets_added ON assets(deleted, added_at) WHERE part_of IS NULL;
    CREATE INDEX idx_fp_key ON fingerprints(kind, key) WHERE kind <> 1;
    CREATE INDEX idx_fp_b0 ON fingerprints(key & 65535) WHERE kind = 1;
    CREATE INDEX idx_fp_b1 ON fingerprints((key >> 16) & 65535) WHERE kind = 1;
    CREATE INDEX idx_fp_b2 ON fingerprints((key >> 32) & 65535) WHERE kind = 1;
    CREATE INDEX idx_fp_b3 ON fingerprints((key >> 48) & 65535) WHERE kind = 1;
    CREATE INDEX idx_dup_ignored_b ON dup_ignored(b);
    CREATE INDEX idx_assets_pending_meta ON assets(meta_state, id) WHERE meta_state = 0 AND status = 0 AND part_of IS NULL;
    CREATE TRIGGER store_folder_upd AFTER UPDATE OF name, parent_id ON folders
        WHEN OLD.name IS NOT NEW.name OR OLD.parent_id IS NOT NEW.parent_id
        BEGIN INSERT OR IGNORE INTO store_dirty VALUES (0); END;
    CREATE TRIGGER store_af_ins AFTER INSERT ON asset_folders BEGIN INSERT OR IGNORE INTO store_dirty VALUES (NEW.asset_id); END;
    CREATE TRIGGER store_af_del AFTER DELETE ON asset_folders BEGIN INSERT OR IGNORE INTO store_dirty VALUES (OLD.asset_id); END;
    CREATE TRIGGER store_trash AFTER UPDATE OF deleted ON assets WHEN OLD.deleted <> NEW.deleted BEGIN INSERT OR IGNORE INTO store_dirty VALUES (NEW.id); END;
    CREATE TRIGGER store_rename AFTER UPDATE OF name ON assets WHEN OLD.name IS NOT NEW.name BEGIN INSERT OR IGNORE INTO store_dirty VALUES (NEW.id); END;
    CREATE TRIGGER store_te_ins AFTER INSERT ON trashed_entries BEGIN INSERT OR IGNORE INTO store_dirty VALUES (NEW.asset_id); END;
    CREATE TRIGGER store_te_del AFTER DELETE ON trashed_entries BEGIN INSERT OR IGNORE INTO store_dirty VALUES (OLD.asset_id); END;
    CREATE TRIGGER store_source AFTER UPDATE OF file_key, size, mtime, status ON assets
        WHEN (OLD.file_key IS NOT NEW.file_key OR OLD.size IS NOT NEW.size OR OLD.mtime IS NOT NEW.mtime OR OLD.status IS NOT NEW.status)
        AND NOT EXISTS (SELECT 1 FROM roots r WHERE r.id = NEW.root_id AND r.kind = 'managed')
        AND (EXISTS (SELECT 1 FROM placements p WHERE p.asset_id = NEW.id) OR EXISTS (SELECT 1 FROM asset_folders x WHERE x.asset_id = NEW.id))
        BEGIN INSERT OR IGNORE INTO store_dirty VALUES (NEW.id); END;
    CREATE TRIGGER store_folder_ins AFTER INSERT ON folders BEGIN INSERT OR IGNORE INTO store_dirty VALUES (0); END;
    CREATE TRIGGER store_folder_del BEFORE DELETE ON folders
        BEGIN
        INSERT OR IGNORE INTO store_dirty SELECT asset_id FROM placements WHERE folder_id = OLD.id;
        UPDATE placements SET folder_id = NULL WHERE folder_id = OLD.id;
        INSERT OR IGNORE INTO store_gone_dirs SELECT OLD.dir WHERE OLD.dir IS NOT NULL;
        INSERT OR IGNORE INTO store_dirty VALUES (0);
        END;
    CREATE TRIGGER part_af_ins AFTER INSERT ON asset_folders BEGIN INSERT OR IGNORE INTO store_dirty SELECT id FROM assets WHERE part_of = NEW.asset_id; END;
    CREATE TRIGGER part_af_del AFTER DELETE ON asset_folders BEGIN INSERT OR IGNORE INTO store_dirty SELECT id FROM assets WHERE part_of = OLD.asset_id; END;
    CREATE TRIGGER part_te_ins AFTER INSERT ON trashed_entries BEGIN INSERT OR IGNORE INTO store_dirty SELECT id FROM assets WHERE part_of = NEW.asset_id; END;
    CREATE TRIGGER part_te_del AFTER DELETE ON trashed_entries BEGIN INSERT OR IGNORE INTO store_dirty SELECT id FROM assets WHERE part_of = OLD.asset_id; END;
    CREATE TRIGGER part_rename AFTER UPDATE OF name ON assets WHEN OLD.name IS NOT NEW.name AND NEW.part_of IS NULL BEGIN INSERT OR IGNORE INTO store_dirty SELECT id FROM assets WHERE part_of = NEW.id; END;
    CREATE TRIGGER part_trash AFTER UPDATE OF deleted ON assets WHEN OLD.deleted <> NEW.deleted AND NEW.part_of IS NULL BEGIN UPDATE assets SET deleted = NEW.deleted WHERE part_of = NEW.id AND deleted <> NEW.deleted; END;
    CREATE TRIGGER part_changed AFTER UPDATE OF size, mtime, status ON assets
        WHEN NEW.part_of IS NOT NULL AND (OLD.size IS NOT NEW.size OR OLD.mtime IS NOT NEW.mtime OR (OLD.status = 2) IS NOT (NEW.status = 2))
        BEGIN
        UPDATE assets SET meta_state = 0 WHERE id = NEW.part_of;
        UPDATE assets SET thumb_state = CASE WHEN thumb_state = 0 THEN 0 ELSE 3 END, thumb_ver = thumb_ver + 1 WHERE id = NEW.part_of AND NEW.part_cover = 1;
        END;
    CREATE TRIGGER part_source AFTER UPDATE OF size, mtime, status ON assets
        WHEN NEW.part_of IS NOT NULL AND (OLD.size IS NOT NEW.size OR OLD.mtime IS NOT NEW.mtime OR OLD.status IS NOT NEW.status)
        BEGIN INSERT OR IGNORE INTO store_dirty VALUES (NEW.id); END;
    CREATE TRIGGER loose_root_del AFTER DELETE ON assets WHEN EXISTS (SELECT 1 FROM roots r WHERE r.id = OLD.root_id AND r.kind = 'linked' AND r.explicit_only = 1 AND NOT (r.path LIKE '%.library/images' OR r.path LIKE '%.library\images'))
        BEGIN DELETE FROM roots WHERE id = OLD.root_id AND NOT EXISTS (SELECT 1 FROM assets WHERE root_id = OLD.root_id) AND NOT EXISTS (SELECT 1 FROM file_ops WHERE root_id = OLD.root_id); END;
    CREATE TRIGGER loose_root_move AFTER UPDATE OF root_id ON assets WHEN OLD.root_id <> NEW.root_id AND EXISTS (SELECT 1 FROM roots r WHERE r.id = OLD.root_id AND r.kind = 'linked' AND r.explicit_only = 1 AND NOT (r.path LIKE '%.library/images' OR r.path LIKE '%.library\images'))
        BEGIN DELETE FROM roots WHERE id = OLD.root_id AND NOT EXISTS (SELECT 1 FROM assets WHERE root_id = OLD.root_id) AND NOT EXISTS (SELECT 1 FROM file_ops WHERE root_id = OLD.root_id); END;
    "#,
    },
    Migration {
        floor: Floor::Keep,
        catch_up: &[],
        sql: "CREATE INDEX IF NOT EXISTS idx_assets_pending_thumb ON assets(added_at DESC) WHERE thumb_state IN (0, 3) AND status = 0 AND deleted = 0 AND part_of IS NULL;",
    },
];

pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;
