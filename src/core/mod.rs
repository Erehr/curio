//! Core: everything that is not presentation. The UI only talks to `library::Library`
//! (and read-only `db::Db` lookups); nothing here depends on the window.

pub mod api;
pub mod archive;
pub mod consts;
pub mod db;
pub mod duplicates;
pub mod export;
pub mod formats;
pub mod library;
pub mod grouping;
pub mod images;
pub mod import;
pub mod media;
pub mod metadata;
pub mod model;
pub mod plugins;
pub mod query;
pub mod said;
pub mod scan;
pub mod store;
pub mod system;
pub mod thumbs;
