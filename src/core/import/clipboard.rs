//! What the clipboard holds, as a picture, text or files.


pub fn clipboard_image_png() -> Result<Vec<u8>, String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let img = cb.get_image().map_err(|_| "The clipboard has no image".to_string())?;
    let rgba = image::RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.into_owned()).ok_or("bad clipboard image")?;
    let mut out = Vec::new();
    image::DynamicImage::ImageRgba8(rgba)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

pub fn clipboard_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Files copied in a file manager (the clipboard's file list).
pub fn clipboard_files() -> Vec<String> {
    arboard::Clipboard::new()
        .and_then(|mut c| c.get().file_list())
        .map(|l| l.into_iter().map(|p| p.to_string_lossy().to_string()).collect())
        .unwrap_or_default()
}
