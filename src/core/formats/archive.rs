//! Compressed archives. ZIP and 7z show their first picture as a thumbnail.
use super::Family;
use crate::core::model::AssetKind;

pub const FAMILY: Family = Family {
    id: "archive",
    name: "Archives",
    description: "Picture thumbnails for ZIP and 7z archives.",
    icon: "file-zip",
    kind: AssetKind::Archive,
    exts: &["zip", "rar", "7z", "tar", "gz", "tgz", "xz"],
};
