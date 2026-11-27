//! Project files of design programs, shown by the preview they carry (see `embedded`), else through system thumbnails until a plugin reads them.
use super::Family;
use crate::core::model::AssetKind;

pub const FAMILY: Family = Family {
    id: "design",
    name: "Design projects",
    description: "Files from Affinity, InDesign, XD, CorelDRAW, SketchUp, Clip Studio and others.",
    icon: "vector-bezier-2",
    kind: AssetKind::Document,
    exts: &[
        "xd", "cdr", "skp", "dwg", "clip", "af", "afpub", "afdesign", "afphoto", "indd", "indt", "idml",
        "prd", "fig", "sketch", "pxd", "skt", "graffle", "mindnode", "xmind", "eddx", "emmx", "ps",
    ],
};
