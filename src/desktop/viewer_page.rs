//! A link's page in the viewer's own web view, placed over the viewer and handing its keys back.

use crate::desktop::background;
use crate::desktop::R;
use crate::desktop::protocol::WEB_WINDOW_PREFIX;
use tauri::Manager;
use tauri::Emitter;
use crate::desktop::protocol::browser_args;
use crate::desktop::config::own_data;
use crate::desktop::app_windows::keep_behind;

/// A window showing a web page, sharing the web view's profile with Curio's other windows; its label must start with WEB_WINDOW_PREFIX, which keeps Curio's commands and protocols from it.
pub(crate) fn web_window<'a>(app: &'a tauri::AppHandle, label: &str, url: tauri::Url) -> tauri::WebviewWindowBuilder<'a, tauri::Wry, tauri::AppHandle> {
    own_data(tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::External(url))).additional_browser_args(&browser_args())
}

/// The label of the viewer's page in a window: under WEB_WINDOW_PREFIX, so it never reaches Curio's commands or files.
fn viewer_page_label(window: &str) -> String {
    format!("{WEB_WINDOW_PREFIX}-in-{window}")
}

/// A link's page in the viewer: a web view of its own laid over the viewer's stage, in physical pixels of the window. `open` loads `url` there, `place` moves it or, with no `rect`, hides it under an overlay, `close` ends it.
#[tauri::command]
pub(super) async fn viewer_page(window: tauri::Window, op: String, url: Option<String>, rect: Option<[f64; 4]>) -> R<()> {
    // the viewer shows its own card when the page fails, so the reason is only in the log
    let done = page_op(&window, &op, url, rect);
    if let Err(e) = &done {
        crate::core::system::logging::warn(&format!("link page in the viewer, {op}: {e}"));
    }
    done
}

fn page_op(window: &tauri::Window, op: &str, url: Option<String>, rect: Option<[f64; 4]>) -> R<()> {
    let label = viewer_page_label(window.label());
    let page = window.app_handle().get_webview(&label);
    let bounds = |[x, y, w, h]: [f64; 4]| tauri::Rect { position: tauri::PhysicalPosition::new(x, y).into(), size: tauri::PhysicalSize::new(w.max(1.0), h.max(1.0)).into() };
    match op {
        "close" => {
            if let Some(p) = page {
                let _ = p.close();
                keys_to_window(window);
                #[cfg(target_os = "macos")]
                {
                    let label = window.label().to_string();
                    let _ = window.app_handle().run_on_main_thread(move || forget_escape(&label));
                }
            }
            Ok(())
        }
        "place" => {
            let Some(p) = page else { return Ok(()) };
            match rect {
                Some(r) => {
                    place_page(&p, r)?;
                    p.show().map_err(|e| e.to_string())
                }
                None => p.hide().map_err(|e| e.to_string()),
            }
        }
        "open" => {
            let parsed: tauri::Url = url.unwrap_or_default().parse().map_err(|e: url::ParseError| e.to_string())?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err("only web links can be opened".into());
            }
            let rect = rect.ok_or("no place for the page")?;
            let r = bounds(rect);
            if let Some(p) = page {
                if p.url().ok().as_ref() != Some(&parsed) {
                    p.navigate(parsed).map_err(|e| e.to_string())?;
                }
                place_page(&p, rect)?;
                p.show().map_err(|e| e.to_string())?;
                keys_to_window(&window);
                return Ok(());
            }
            let app = window.app_handle().clone();
            let here = label.clone();
            let mut b = tauri::webview::WebviewBuilder::new(&label, tauri::WebviewUrl::External(parsed))
                .additional_browser_args(&browser_args())
                // a link that asks for a new window opens in place, as the viewer shows one page
                .on_new_window(move |url, _| {
                    if !matches!(url.scheme(), "http" | "https") {
                        return tauri::webview::NewWindowResponse::Deny;
                    }
                    if let Some(p) = app.get_webview(&here) {
                        let _ = p.navigate(url);
                    }
                    tauri::webview::NewWindowResponse::Deny
                });
            if let Some(d) = crate::core::system::app_dirs::webview_dir() {
                b = b.data_directory(d);
            }
            let p = window.add_child(b, r.position, r.size).map_err(|e| e.to_string())?;
            #[cfg(target_os = "linux")]
            place_page(&p, rect)?;
            escape_closes_viewer(&p, window.label());
            keys_to_window(&window);
            Ok(())
        }
        _ => Err(format!("unknown op {op}")),
    }
}

/// Where the viewer's page lies, in physical pixels of its window.
#[cfg(not(target_os = "linux"))]
fn place_page(p: &tauri::Webview, [x, y, w, h]: [f64; 4]) -> R<()> {
    p.set_bounds(tauri::Rect { position: tauri::PhysicalPosition::new(x, y).into(), size: tauri::PhysicalSize::new(w.max(1.0), h.max(1.0)).into() }).map_err(|e| e.to_string())
}

/// GTK stacks a window's web views one under another in its box, so the page goes into an overlay over the window's own page instead, placed by its margins.
#[cfg(target_os = "linux")]
fn place_page(p: &tauri::Webview, [x, y, w, h]: [f64; 4]) -> R<()> {
    p.with_webview(move |pw| {
        use gtk::glib::prelude::{Cast, ObjectExt};
        use gtk::prelude::{BoxExt, ContainerExt, OverlayExt, WidgetExt};
        let view = pw.inner();
        if let Some(vbox) = view.parent().and_then(|w| w.downcast::<gtk::Box>().ok()) {
            vbox.remove(&view);
            let overlay = match vbox.children().into_iter().find_map(|c| c.downcast::<gtk::Overlay>().ok()) {
                Some(o) => o,
                None => {
                    let o = gtk::Overlay::new();
                    if let Some(own) = vbox.children().into_iter().find(|c| c.is::<webkit2gtk::WebView>()) {
                        vbox.remove(&own);
                        o.add(&own);
                    }
                    vbox.pack_start(&o, true, true, 0);
                    o.show();
                    o
                }
            };
            view.set_halign(gtk::Align::Start);
            view.set_valign(gtk::Align::Start);
            overlay.add_overlay(&view);
        }
        let k = view.scale_factor().max(1) as f64;
        view.set_margin_start((x / k).round() as i32);
        view.set_margin_top((y / k).round() as i32);
        view.set_size_request((w / k).round().max(1.0) as i32, (h / k).round().max(1.0) as i32);
    })
    .map_err(|e| e.to_string())
}

/// The window's own page keeps the keys when a link's page opens, so the arrows go on stepping through items until the page is clicked.
fn keys_to_window(window: &tauri::Window) {
    if let Some(main) = window.app_handle().get_webview(window.label()) {
        let _ = main.set_focus();
    }
}

/// Escape in the viewer's page closes the viewer, as it does over a picture: the page has the keys while it has the focus.
#[cfg(windows)]
fn escape_closes_viewer(page: &tauri::Webview, window: &str) {
    let app = page.app_handle().clone();
    let window = window.to_string();
    let _ = page.with_webview(move |pw| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN;
        let handler = webview2_com::AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                let mut kind = Default::default();
                let mut key = 0u32;
                args.KeyEventKind(&mut kind)?;
                args.VirtualKey(&mut key)?;
                if kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN && key == 0x1B {
                    args.SetHandled(true)?;
                    let _ = app.emit_to(window.as_str(), "viewer-page-key", "Escape");
                }
            }
            Ok(())
        }));
        let mut token = Default::default();
        let _ = pw.controller().add_AcceleratorKeyPressed(&handler, &mut token);
    });
}

#[cfg(target_os = "linux")]
fn escape_closes_viewer(page: &tauri::Webview, window: &str) {
    let app = page.app_handle().clone();
    let window = window.to_string();
    let _ = page.with_webview(move |pw| {
        use gtk::prelude::WidgetExt;
        pw.inner().connect_key_press_event(move |_, ev| {
            if ev.keyval() == gtk::gdk::keys::constants::Escape {
                let _ = app.emit_to(window.as_str(), "viewer-page-key", "Escape");
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
    });
}

// AppKit gives a key press to the focused view with nothing for its owner to catch, so a monitor of this process's key presses takes Escape while the page has the focus; one per window, replaced when its page is made again.
#[cfg(target_os = "macos")]
thread_local! {
    static ESCAPE_MONITORS: std::cell::RefCell<std::collections::HashMap<String, objc2::rc::Retained<objc2::runtime::AnyObject>>> = Default::default();
}

#[cfg(target_os = "macos")]
fn escape_closes_viewer(page: &tauri::Webview, window: &str) {
    let app = page.app_handle().clone();
    let window = window.to_string();
    let _ = page.with_webview(move |pw| {
        use objc2_app_kit::{NSEvent, NSEventMask, NSView};
        const ESCAPE_KEY_CODE: u16 = 53;
        let view: &NSView = unsafe { &*(pw.inner() as *const NSView) };
        let page = objc2::rc::Weak::new(view);
        let label = window.clone();
        let handler = block2::RcBlock::new(move |event: std::ptr::NonNull<NSEvent>| -> *mut NSEvent {
            let ev = unsafe { event.as_ref() };
            let focused = || {
                let (Some(page), Some(mtm)) = (page.load(), objc2::MainThreadMarker::new()) else { return false };
                let Some(responder) = ev.window(mtm).and_then(|w| w.firstResponder()) else { return false };
                responder.downcast::<NSView>().is_ok_and(|v| v.isDescendantOf(&page))
            };
            if ev.keyCode() == ESCAPE_KEY_CODE && focused() {
                let _ = app.emit_to(label.as_str(), "viewer-page-key", "Escape");
                return std::ptr::null_mut();
            }
            event.as_ptr()
        });
        let Some(monitor) = (unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyDown, &handler) }) else { return };
        forget_escape(&window);
        ESCAPE_MONITORS.with(|m| m.borrow_mut().insert(window, monitor));
    });
}

#[cfg(target_os = "macos")]
fn forget_escape(window: &str) {
    if let Some(old) = ESCAPE_MONITORS.with(|m| m.borrow_mut().remove(window)) {
        unsafe { objc2_app_kit::NSEvent::removeMonitor(&old) };
    }
}

/// Open a web page inside the app (its own window), for link items.
#[tauri::command]
pub(super) async fn open_web(app: tauri::AppHandle, url: String, title: Option<String>) -> R<()> {
    let parsed: tauri::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("only web links can be opened".into());
    }
    let label = format!("{WEB_WINDOW_PREFIX}{}", fastrand::u32(..));
    let w = crate::desktop::system::material::sized(web_window(&app, &label, parsed), &app, 1280.0, 860.0)
        .focused(!background())
        .always_on_bottom(background())
        .visible(!background())
        .title(title.unwrap_or_else(|| url.clone()))
        .build()
        .map_err(|e| e.to_string())?;
    keep_behind(&w.as_ref().window());
    Ok(())
}
