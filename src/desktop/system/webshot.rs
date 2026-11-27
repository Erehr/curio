//! Pictures of web pages for link thumbnails, drawn by Curio's own web view in a window nobody sees: a real browser with the cookies its pages left, where a headless one meets bot checks.
//! WebView2's CapturePreview on Windows, WebKitGTK's snapshot on Linux; where this fails, `core::media` runs a headless browser.

use std::path::Path;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// Longest wait for the page to finish loading; a page that keeps loading is taken as it stands then.
const LOAD_WAIT: Duration = Duration::from_secs(12);
/// What a finished page gets to draw what its scripts add.
const SETTLE: Duration = Duration::from_millis(1500);
const PICTURE_WAIT: Duration = Duration::from_secs(10);
/// Hidden pages drawn at once: a refresh of many links waits its turn instead of opening a browser per worker.
const AT_ONCE: usize = 2;

static RUNNING: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());

struct Turn;

impl Turn {
    fn take() -> Turn {
        let (n, cv) = &RUNNING;
        let mut n = cv.wait_while(n.lock().unwrap_or_else(|e| e.into_inner()), |n| *n >= AT_ONCE).unwrap_or_else(|e| e.into_inner());
        *n += 1;
        Turn
    }
}

impl Drop for Turn {
    fn drop(&mut self) {
        let (n, cv) = &RUNNING;
        *n.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
        cv.notify_one();
    }
}

/// A page may go on to other public pages, never to this computer or the local network by a name or address it names; past the first address (`resolves_public`), a public name that resolves there is not caught, nor is what the page loads into itself.
/// The shared profile is why: its cookies keep banners and bot checks away, and a proxy would need a profile of its own.
fn may_visit(url: &tauri::Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return url.scheme() == "about";
    }
    match url.host() {
        Some(url::Host::Domain(d)) => {
            let d = d.trim_end_matches('.').to_ascii_lowercase();
            d != "localhost" && !d.ends_with(".localhost")
        }
        Some(url::Host::Ipv4(ip)) => crate::core::system::net::is_public(ip.into()),
        Some(url::Host::Ipv6(ip)) => crate::core::system::net::is_public(ip.into()),
        None => false,
    }
}

/// Every address the page's name has is on the internet: a name of a public-looking site may lead to this computer or the local network.
fn resolves_public(url: &tauri::Url) -> bool {
    use std::net::ToSocketAddrs;
    let Some(url::Host::Domain(d)) = url.host() else { return true };
    let Ok(addrs) = (d, url.port_or_known_default().unwrap_or(80)).to_socket_addrs() else { return false };
    let addrs: Vec<_> = addrs.collect();
    !addrs.is_empty() && addrs.iter().all(|a| crate::core::system::net::is_public(a.ip()))
}

// scroll bars would show down the picture's side
const NO_SCROLLBARS: &str = "addEventListener('DOMContentLoaded', () => { const s = document.createElement('style'); s.textContent = '::-webkit-scrollbar { display: none }'; document.head.append(s) })";

/// Must not run on the main thread, which builds the window and answers the web view.
pub fn capture(app: &tauri::AppHandle, url: &str, out: &Path) -> Result<(), String> {
    use crate::core::consts::{PAGE_CAPTURE_HEIGHT as H, PAGE_CAPTURE_WIDTH as W};
    use tauri::Manager;
    let parsed: tauri::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    if !may_visit(&parsed) || !resolves_public(&parsed) {
        return Err("only public web pages are drawn".into());
    }
    let _turn = Turn::take();
    let label = format!("{}capture{}", crate::desktop::protocol::WEB_WINDOW_PREFIX, fastrand::u32(..));
    let (loaded_tx, loaded_rx) = crossbeam_channel::bounded::<()>(1);
    let w = crate::desktop::viewer_page::web_window(app, &label, parsed)
        .visible(false)
        .focused(false)
        .skip_taskbar(true)
        .inner_size(W as f64, H as f64)
        .initialization_script(NO_SCROLLBARS)
        .on_navigation(may_visit)
        // a link to a file would be saved to Downloads without a word
        .on_download(|_, _| false)
        .on_page_load(move |_, p| {
            if matches!(p.event(), tauri::webview::PageLoadEvent::Finished) {
                let _ = loaded_tx.try_send(());
            }
        })
        .build()
        .map_err(|e| e.to_string())?;
    let r = (|| {
        w.with_webview(prepare).map_err(|e| e.to_string())?;
        if loaded_rx.recv_timeout(LOAD_WAIT).is_ok() {
            std::thread::sleep(SETTLE);
        }
        let (tx, rx) = crossbeam_channel::bounded::<Picture>(1);
        let fail = tx.clone();
        w.with_webview(move |pw| {
            if let Err(e) = start_picture(&pw, tx) {
                let _ = fail.send(Err(e));
            }
        })
        .map_err(|e| e.to_string())?;
        let png = rx.recv_timeout(PICTURE_WAIT).map_err(|_| "the page gave no picture".to_string())??;
        std::fs::write(out, png).map_err(|e| e.to_string())
    })();
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.with_webview(finish);
        let _ = w.destroy();
    }
    r
}

/// The page's PNG, or why there is none.
type Picture = Result<Vec<u8>, String>;

/// A hidden window's web view stops drawing unless told it shows; and it stays silent.
#[cfg(windows)]
fn prepare(pw: tauri::webview::PlatformWebview) {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_8;
    use windows::core::Interface;
    unsafe {
        let c = pw.controller();
        let _ = c.SetIsVisible(true);
        if let Ok(c8) = c.CoreWebView2().and_then(|core| core.cast::<ICoreWebView2_8>()) {
            let _ = c8.SetIsMuted(true);
        }
    }
}

/// WebKitGTK draws nothing for a window that isn't shown, so the page moves into an off-screen window of GTK's, which draws without a screen; it stays silent.
#[cfg(target_os = "linux")]
fn prepare(pw: tauri::webview::PlatformWebview) {
    use crate::core::consts::{PAGE_CAPTURE_HEIGHT as H, PAGE_CAPTURE_WIDTH as W};
    use gtk::prelude::{ContainerExt, GtkWindowExt, WidgetExt};
    use webkit2gtk::WebViewExt;
    let view = pw.inner();
    view.set_is_muted(true);
    if let Some(parent) = view.parent().and_then(|p| gtk::glib::Cast::downcast::<gtk::Container>(p).ok()) {
        parent.remove(&view);
    }
    let off = gtk::OffscreenWindow::new();
    off.set_default_size(W as i32, H as i32);
    view.set_size_request(W as i32, H as i32);
    off.add(&view);
    off.show_all();
}

/// The off-screen window goes before the window the page was made in, which ends the page.
#[cfg(target_os = "linux")]
fn finish(pw: tauri::webview::PlatformWebview) {
    use gtk::prelude::{ContainerExt, GtkWindowExt, WidgetExt};
    let view = pw.inner();
    if let Some(off) = view.toplevel().and_then(|t| gtk::glib::Cast::downcast::<gtk::OffscreenWindow>(t).ok()) {
        off.remove(&view);
        off.close();
    }
}

#[cfg(windows)]
fn finish(_: tauri::webview::PlatformWebview) {}

#[cfg(target_os = "linux")]
fn start_picture(pw: &tauri::webview::PlatformWebview, tx: crossbeam_channel::Sender<Picture>) -> Result<(), String> {
    use webkit2gtk::{SnapshotOptions, SnapshotRegion, WebViewExt};
    pw.inner().snapshot(SnapshotRegion::Visible, SnapshotOptions::NONE, None::<&gtk::gio::Cancellable>, move |r| {
        let png = r.map_err(|e| e.to_string()).and_then(|surface| {
            let image = gtk::cairo::ImageSurface::try_from(surface).map_err(|_| "the page's picture is not an image".to_string())?;
            let (w, h) = (image.width(), image.height());
            let pixbuf = gtk::gdk::pixbuf_get_from_surface(&image, 0, 0, w, h).ok_or("the page's picture could not be read")?;
            pixbuf.save_to_bufferv("png", &[]).map_err(|e| e.to_string())
        });
        let _ = tx.send(png);
    });
    Ok(())
}

#[cfg(windows)]
fn start_picture(pw: &tauri::webview::PlatformWebview, tx: crossbeam_channel::Sender<Picture>) -> Result<(), String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG;
    let core = unsafe { pw.controller().CoreWebView2() }.map_err(|e| e.to_string())?;
    let stream = unsafe { windows::Win32::UI::Shell::SHCreateMemStream(None) }.ok_or("no memory stream")?;
    let held = stream.clone();
    let done = webview2_com::CapturePreviewCompletedHandler::create(Box::new(move |r: windows::core::Result<()>| {
        let _ = tx.send(r.map_err(|e| e.to_string()).and_then(|_| read_all(&held)));
        Ok(())
    }));
    unsafe { core.CapturePreview(COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG, &stream, &done) }.map_err(|e| e.to_string())
}

#[cfg(windows)]
fn read_all(s: &windows::Win32::System::Com::IStream) -> Result<Vec<u8>, String> {
    use windows::Win32::System::Com::STREAM_SEEK_SET;
    let mut out = Vec::new();
    let mut buf = vec![0u8; 64 << 10];
    unsafe {
        s.Seek(0, STREAM_SEEK_SET, None).map_err(|e| e.to_string())?;
        loop {
            let mut n = 0u32;
            s.Read(buf.as_mut_ptr().cast(), buf.len() as u32, Some(&mut n)).ok().map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
    }
    Ok(out)
}
