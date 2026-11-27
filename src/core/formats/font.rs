//! Font files, read by `metadata/fonts/` and shown by the window's font viewer (`web/src/formats/FontView.vue`).
use super::Family;
use crate::core::model::AssetKind;

pub const FAMILY: Family = Family {
    id: "font",
    name: "Fonts",
    description: "Previews fonts and their glyphs, and reads their weight, category and scripts for filters.",
    icon: "typography",
    kind: AssetKind::Font,
    exts: &[
        "ttf", "otf", "ttc", "otc", "woff", "woff2", "dfont",
    ],
};
