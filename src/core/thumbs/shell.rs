//! Operating-system thumbnails: the Windows shell thumbnail cache, and Quick Look on macOS.
//!
//! Asks the Windows thumbnail cache:
//! reject an empty size, create an IShellItem from the path (SHCreateItemFromParsingName),
//! get the local thumbnail cache (CoCreateInstance(CLSID_LocalThumbnailCache)), call
//! with class context 3 (in-process server + handler), call
//! IThumbnailCache::GetThumbnail(item, requested = size.width, flags = 0 / WTS_EXTRACT),
//! take the shared HBITMAP and convert it to pixels. This gives thumbnails for every format
//! that has a Windows thumbnail provider installed (Cinema 4D, Blender, CorelDRAW, Affinity,
//! SketchUp, DWG, PSD, Office, PDF readers, video codecs …).

/// Requested edge for formats that go to the native thumbnailer first.
pub const NATIVE_THUMB_SIZE: u32 = 1024;
/// Size when the caller has no better idea.
pub const NATIVE_THUMB_DEFAULT: u32 = 512;

/// Extensions that go to the native thumbnailer first: their own programs draw them best.
pub const NATIVE_FIRST: &[&str] = &["dds", "c4d", "blend", "cdr", "clip", "af", "afdesign", "afpub", "afphoto", "skp", "dwg"];

/// Runs on its own short-lived thread so the COM apartment never leaks into the caller
/// (the window's thread needs a single-threaded apartment for drag and drop).
#[cfg(windows)]
pub fn thumbnail(path: &std::path::Path, size: u32) -> Result<image::DynamicImage, String> {
    if size == 0 {
        return Err("size must not be empty".into());
    }
    let p = path.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("shell-thumb".into())
        .spawn(move || {
            let r = std::panic::catch_unwind(|| thumbnail_on_this_thread(&p, size)).unwrap_or_else(|_| Err("thumbnail provider crashed".into()));
            unsafe { windows::Win32::System::Com::CoUninitialize() };
            let _ = tx.send(r);
        })
        .map_err(|e| e.to_string())?;
    // a provider that hangs keeps its own thread, not a thumbnail worker
    rx.recv_timeout(std::time::Duration::from_secs(20)).map_err(|_| "the thumbnail provider didn't answer".to_string())?
}

#[cfg(windows)]
fn thumbnail_on_this_thread(path: &std::path::Path, size: u32) -> Result<image::DynamicImage, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_HANDLER, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
    use windows::Win32::UI::Shell::{IShellItem, ISharedBitmap, IThumbnailCache, LocalThumbnailCache, SHCreateItemFromParsingName, WTS_EXTRACT};

    if size == 0 {
        return Err("size must not be empty".into());
    }
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    unsafe {
        // each worker thread joins the multithreaded apartment once; repeated calls are harmless
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let item: IShellItem = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None).map_err(|_| "failed to create IShellItem from the given path".to_string())?;
        let cache: IThumbnailCache =
            CoCreateInstance(&LocalThumbnailCache, None, CLSCTX_INPROC_SERVER | CLSCTX_INPROC_HANDLER).map_err(|_| "failed to acquire local thumbnail cache reference".to_string())?;
        let mut shared: Option<ISharedBitmap> = None;
        cache
            .GetThumbnail(&item, size, WTS_EXTRACT, Some(&mut shared), None, None)
            .map_err(|_| "failed to get thumbnail from local thumbnail cache reference".to_string())?;
        let shared = shared.ok_or("failed to get thumbnail from local thumbnail cache reference")?;
        let hbm = shared.GetSharedBitmap().map_err(|_| "failed to extract bitmap from thumbnail".to_string())?;
        let mut bm = BITMAP::default();
        if GetObjectW(HGDIOBJ(hbm.0), std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _)) == 0 {
            return Err("could not convert HBITMAP to BITMAP".into());
        }
        let (w, h) = (bm.bmWidth.max(1) as u32, bm.bmHeight.unsigned_abs().max(1));
        let mut info = BITMAPINFO::default();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w as i32,
            biHeight: -(h as i32), // top-down rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let len = (w as usize).checked_mul(h as usize).and_then(|n| n.checked_mul(4)).filter(|&n| n <= 256 << 20).ok_or("thumbnail too large")?;
        let mut buf = vec![0u8; len];
        let dc = CreateCompatibleDC(None);
        let lines = GetDIBits(dc, hbm, 0, h, Some(buf.as_mut_ptr() as *mut _), &mut info, DIB_RGB_COLORS);
        let _ = DeleteDC(dc);
        if lines == 0 {
            return Err("could not read thumbnail pixels".into());
        }
        // BGRA → RGBA; providers without alpha leave it at 0, treat that as opaque
        let any_alpha = buf.chunks(4).any(|p| p[3] != 0);
        for p in buf.chunks_mut(4) {
            p.swap(0, 2);
            if !any_alpha {
                p[3] = 255;
            }
        }
        image::RgbaImage::from_raw(w, h, buf).map(image::DynamicImage::ImageRgba8).ok_or_else(|| "bad thumbnail buffer".to_string())
    }
}

/// Quick Look's thumbnail, through its command-line tool: PDFs, office, Pages and Keynote files, and whatever else has a Quick Look extension installed.
#[cfg(target_os = "macos")]
pub fn thumbnail(path: &std::path::Path, size: u32) -> Result<image::DynamicImage, String> {
    if size == 0 {
        return Err("no thumbnail size".into());
    }
    let dir = std::env::temp_dir().join(format!("curio_ql_{}", fastrand::u64(..)));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let args = vec!["-t".to_string(), "-s".into(), size.to_string(), "-o".into(), dir.to_string_lossy().into_owned(), path.to_string_lossy().into_owned()];
    let ran = crate::core::system::process::run("/usr/bin/qlmanage", &args, None, std::time::Duration::from_millis(crate::core::consts::THUMBNAIL_TIMEOUT_MS));
    // it writes <file name>.png into the folder, and nothing when it has no thumbnail for the file
    let out = dir.join(format!("{}.png", path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
    let img = image::open(&out).map_err(|e| e.to_string());
    let _ = std::fs::remove_dir_all(&dir);
    img.map_err(|e| format!("Quick Look made no thumbnail: {}", ran.err().unwrap_or(e)))
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn thumbnail(_path: &std::path::Path, _size: u32) -> Result<image::DynamicImage, String> {
    // Linux: thumbnails come from the built-in decoders, the window, or plugin handlers.
    Err("operating-system thumbnails are not used on Linux".into())
}
