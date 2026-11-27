//! A library's own settings, with their defaults.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum HashPolicy {
    /// Full content hashes only when looking for duplicates (default).
    OnDemand,
    /// Hash files below the size limit as they are indexed.
    SmallFiles,
    /// Hash everything in the background.
    Always,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LibrarySettings {
    #[serde(default)]
    pub index_all_files: bool,
    #[serde(default = "default_hash_policy")]
    pub hash_policy: HashPolicy,
    #[serde(default = "default_hash_small_limit_mb")]
    pub hash_small_limit_mb: u64,
    #[serde(default = "default_cache_limit_mb")]
    pub cache_limit_mb: u64,
    #[serde(default = "default_thumb_workers")]
    pub thumb_workers: usize,
    #[serde(default = "default_watch_folders")]
    pub watch_folders: bool,
    #[serde(default = "default_maintenance_minutes")]
    pub maintenance_minutes: u64,
    /// Watched folders and their rules.
    #[serde(default)]
    pub auto_import: crate::core::import::auto_import::AutoImportConfig,
    /// Checked backups of this library kept in the destination; 0 keeps all.
    #[serde(default = "default_backup_keep")]
    pub backup_keep: usize,
    /// Back up when the library changed and the last backup is this old; 0 = only by hand.
    #[serde(default = "default_backup_every_hours")]
    pub backup_every_hours: u64,
    /// Where backups go; empty = the library's own `backups` folder.
    #[serde(default)]
    pub backup_dir: String,
    /// How a second file of the same name in one folder of the managed store is numbered.
    #[serde(default)]
    pub name_suffix: crate::core::store::SuffixStyle,
    /// A camera RAW file and the JPEG or HEIC of the same name beside it: `off` keeps both as items, `jpeg` or `raw` makes that one the item.
    #[serde(default = "default_raw_pairs")]
    pub raw_pairs: String,
}

fn default_hash_policy() -> HashPolicy {
    HashPolicy::OnDemand
}

fn default_hash_small_limit_mb() -> u64 {
    64
}

fn default_cache_limit_mb() -> u64 {
    4096
}

fn default_thumb_workers() -> usize {
    std::thread::available_parallelism().map(|n| n.get().saturating_sub(1).clamp(1, 6)).unwrap_or(2)
}

fn default_watch_folders() -> bool {
    true
}

fn default_maintenance_minutes() -> u64 {
    30
}

fn default_raw_pairs() -> String {
    "off".into()
}

fn default_backup_keep() -> usize {
    10
}

fn default_backup_every_hours() -> u64 {
    24
}

impl Default for LibrarySettings {
    fn default() -> Self {
        LibrarySettings {
            index_all_files: false,
            hash_policy: default_hash_policy(),
            hash_small_limit_mb: default_hash_small_limit_mb(),
            cache_limit_mb: default_cache_limit_mb(),
            thumb_workers: default_thumb_workers(),
            watch_folders: default_watch_folders(),
            maintenance_minutes: default_maintenance_minutes(),
            auto_import: Default::default(),
            backup_keep: default_backup_keep(),
            backup_every_hours: default_backup_every_hours(),
            backup_dir: String::new(),
            name_suffix: Default::default(),
            raw_pairs: default_raw_pairs(),
        }
    }
}

impl LibrarySettings {
    /// Parses a stored settings JSON object field by field, keeping defaults only for keys this build can't parse.
    pub fn from_map(map: &Map<String, Value>) -> Self {
        let mut s = Self::default();
        fn read<T: serde::de::DeserializeOwned>(map: &Map<String, Value>, key: &str, out: &mut T) {
            if let Some(v) = map.get(key) {
                if let Ok(parsed) = serde_json::from_value(v.clone()) {
                    *out = parsed;
                }
            }
        }
        read(map, "index_all_files", &mut s.index_all_files);
        read(map, "hash_policy", &mut s.hash_policy);
        read(map, "hash_small_limit_mb", &mut s.hash_small_limit_mb);
        read(map, "cache_limit_mb", &mut s.cache_limit_mb);
        read(map, "thumb_workers", &mut s.thumb_workers);
        read(map, "watch_folders", &mut s.watch_folders);
        read(map, "maintenance_minutes", &mut s.maintenance_minutes);
        read(map, "auto_import", &mut s.auto_import);
        read(map, "backup_keep", &mut s.backup_keep);
        read(map, "backup_every_hours", &mut s.backup_every_hours);
        read(map, "backup_dir", &mut s.backup_dir);
        read(map, "name_suffix", &mut s.name_suffix);
        read(map, "raw_pairs", &mut s.raw_pairs);
        s
    }

    pub fn parse_stored(text: Option<&str>) -> (Map<String, Value>, Self) {
        let raw = text.and_then(|s| serde_json::from_str::<Map<String, Value>>(s).ok()).unwrap_or_default();
        let parsed = Self::from_map(&raw);
        (raw, parsed)
    }
}
