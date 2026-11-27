//! Saved web links and internet shortcuts.
use super::Family;
use crate::core::model::AssetKind;

pub const FAMILY: Family = Family {
    id: "link",
    name: "Links",
    description: "Saved web links and internet shortcuts.",
    icon: "link",
    kind: AssetKind::Link,
    exts: &["url", "webloc"],
};
