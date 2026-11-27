//! Still and animated pictures, camera raw and layered image files.
use super::Family;
use crate::core::model::AssetKind;

pub const FAMILY: Family = Family {
    id: "image",
    name: "Images",
    description: "Photos, drawings, camera raw and layered files, with their EXIF data.",
    icon: "photo",
    kind: AssetKind::Image,
    exts: &[
        "png", "jpg", "jpeg", "jfif", "jpe", "insp", "webp", "avif", "gif", "apng", "svg", "tif", "tiff",
        "bmp", "ico", "icns", "heic", "heif", "hif", "jxl", "tga", "psd", "psdt", "psb", "exr", "hdr", "dds",
        "qoi", "pnm", "ppm", "pgm", "arw", "cr2", "cr3", "crw", "dng", "nef", "nrw", "orf", "raf", "rw2",
        "pef", "srw", "sr2", "raw", "3fr", "erf", "x3f", "mrw", "kra", "xcf", "ai", "ait", "eps",
    ],
};

/// Camera RAW files, which the camera often writes beside a JPEG or HEIC of the same name.
pub const RAW: &[&str] = &["arw", "cr2", "cr3", "crw", "dng", "nef", "nrw", "orf", "raf", "rw2", "pef", "srw", "sr2", "raw", "3fr", "erf", "x3f", "mrw"];

/// JPEG can be decoded at 1/2, 1/4 or 1/8 size for a fraction of the work.
pub fn scalable_decode(ext: &str) -> bool {
    matches!(ext, "jpg" | "jpeg" | "jfif" | "jpe")
}
