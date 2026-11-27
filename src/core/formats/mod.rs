//! Every file type the library knows, one file per family. To support a new format, add its
//! extension to the family it belongs to; decoding lives in `thumbs.rs` and `media.rs`, and each
//! family is also listed under Settings > Plugins > Format, where it can be switched off.
//!

pub mod archive;
pub mod audio;
pub mod code;
pub mod design;
pub mod document;
pub mod font;
pub mod image;
pub mod link;
pub mod model3d;
pub mod video;

use std::collections::HashMap;
use std::sync::LazyLock;
use parking_lot::RwLock;
use crate::core::model::AssetKind;

/// A family of formats handled together.
pub struct Family {
    /// `builtin.format.<id>` in the plugin list
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub kind: AssetKind,
    pub exts: &'static [&'static str],
}

pub const FAMILIES: &[&Family] = &[&image::FAMILY, &video::FAMILY, &audio::FAMILY, &document::FAMILY, &code::FAMILY, &link::FAMILY, &design::FAMILY, &archive::FAMILY, &font::FAMILY, &model3d::FAMILY, &model3d::SCENES];

/// Extension kinds declared by active Format plugins, consulted when no built-in family claims the extension.
static PLUGIN_KINDS: LazyLock<RwLock<HashMap<String, AssetKind>>> = LazyLock::new(Default::default);
/// Custom type groups declared by active Format plugins (`formats[].type`): `(label, exts)`.
static PLUGIN_TYPES: LazyLock<RwLock<Vec<(String, Vec<String>)>>> = LazyLock::new(Default::default);

pub fn set_plugin_kinds(kinds: HashMap<String, AssetKind>) {
    *PLUGIN_KINDS.write() = kinds;
}

pub fn set_plugin_types(types: Vec<(String, Vec<String>)>) {
    *PLUGIN_TYPES.write() = types;
}

pub fn plugin_types() -> Vec<(String, Vec<String>)> {
    PLUGIN_TYPES.read().clone()
}

pub fn family_of(ext: &str) -> Option<&'static Family> {
    FAMILIES.iter().find(|f| f.exts.contains(&ext)).copied()
}

pub fn kind_for_ext(ext: &str) -> AssetKind {
    family_of(ext).map(|f| f.kind).or_else(|| PLUGIN_KINDS.read().get(ext).copied()).unwrap_or(AssetKind::Other)
}

/// The id a family has in the plugin list.
pub fn plugin_id(f: &Family) -> String {
    format!("builtin.format.{}", f.id)
}
