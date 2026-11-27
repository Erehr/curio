//! Moving pictures. Anything the web view cannot play is converted once into the cache.
use super::Family;
use crate::core::model::AssetKind;

pub const FAMILY: Family = Family {
    id: "video",
    name: "Video",
    description: "Thumbnails, hover preview and playback.",
    icon: "movie",
    kind: AssetKind::Video,
    exts: &[
        "mp4", "mov", "m4v", "webm", "mkv", "avi", "wmv", "mpg", "mpeg", "mts", "m2ts", "ts", "flv", "f4v",
        "3gp", "ogv", "ogm",
    ],
};
