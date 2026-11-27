//! Plugin runtime.
//!
//! A plugin is a folder with `plugin.json`. Plugins extend the app declaratively (fields, extractors, widgets, actions, URL handlers, filters, sorts, panels, formats, theme variables), show pages of their own (windows, tabs, page widgets, viewers, a settings page), and run programs (actions, URL handlers, extractors, thumbnailers).
//!
//! Isolation: a plugin never gets the database. Pages call the API through the window, programs with a token made for their run (`api::programs`), both with what was granted to the plugin. Failing plugins are logged and auto-disabled after 5 consecutive failures.
//!
//! Plugins made for other programs are not loaded: a separate, optional compatibility plugin can be
//! written later on top of this API (see docs/plugins/index.md).

pub mod contributions;
pub mod examples;
pub mod loading;
pub mod manifest;
pub mod pages;
pub mod run;
pub mod tools;

use crate::core::model::FieldDef;

pub const PLUGIN_API_VERSION: u32 = 1;
pub const MAX_FAILURES: i64 = 5;

/// Ids served as `<origin>/curio/`, which no plugin may take.
pub const RESERVED_ID: &str = "curio";

/// The kinds of files are Curio itself: their entries can't be switched off, though a plugin can take over their formats.
pub fn always_on(id: &str) -> bool {
    id.starts_with("builtin.format.")
}

/// A plugin writes the fields it declares (alone or shared with others) and the user's, never another plugin's or the built-in readers'.
pub fn may_write_field(plugin_id: &str, field: &FieldDef) -> bool {
    field.owner == "user" || field.declared_by(plugin_id)
}

