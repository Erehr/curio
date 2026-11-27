//! InDesign documents and templates keep a JPEG thumbnail as base64 in their XMP metadata (`xmpGImg:image`); each save may add another, so the largest one found is used.

use std::path::Path;

use image::DynamicImage;

pub fn preview(path: &Path) -> Option<DynamicImage> {
    let mut best: Option<DynamicImage> = None;
    super::each_between(path, b"<xmpGImg:image>", b"</xmpGImg:image>", (super::MAX_PICTURE as usize) * 4 / 3 + 4096, 32, |text| {
        let img = super::xml_base64(text).and_then(|b| super::picture(&b));
        if let Some(img) = img {
            let area = |i: &DynamicImage| u64::from(i.width()) * u64::from(i.height());
            if best.as_ref().is_none_or(|b| area(&img) > area(b)) {
                best = Some(img);
            }
        }
        false
    });
    best
}
