//! Where a library keeps what it has: its folder, database, files, thumbnails, backups and logs.

use std::path::{Path, PathBuf};
use crate::core::system::filesystem;

/// Library folder layout. Originals, caches and the database are kept apart.
#[derive(Clone, Debug)]
pub struct LibraryPaths {
    pub root: PathBuf,
    pub db: PathBuf,
    pub managed: PathBuf,
    pub thumbs: PathBuf,
    pub previews: PathBuf,
    pub tmp: PathBuf,
    pub backups: PathBuf,
    pub exports: PathBuf,
    pub plugins: PathBuf,
}

impl LibraryPaths {
    pub fn new(root: &Path, cache_override: Option<&Path>) -> Self {
        // a shared custom location gets one folder per library, so their limits and cleanups don't touch each other
        let cache = match cache_override {
            Some(p) => {
                let name: String = filesystem::name_of(root).chars().map(|c| if c.is_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect();
                let id = blake3::hash(filesystem::canonical(root).to_string_lossy().to_lowercase().as_bytes()).to_hex();
                p.join(format!("{name}-{}", &id[..8]))
            }
            None => root.join("cache"),
        };
        LibraryPaths {
            root: root.to_path_buf(),
            db: root.join("library.db"),
            managed: root.join("files"),
            thumbs: cache.join("thumbs"),
            previews: cache.join("previews"),
            tmp: cache.join("tmp"),
            backups: root.join("backups"),
            exports: root.join("exports"),
            plugins: root.join("plugins"),
        }
    }
    pub fn ensure(&self) -> std::io::Result<()> {
        for d in [&self.root, &self.managed, &self.thumbs, &self.previews, &self.tmp, &self.backups, &self.exports, &self.plugins] {
            std::fs::create_dir_all(d)?;
        }
        Ok(())
    }
}
