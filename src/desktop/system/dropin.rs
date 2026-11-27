//! What other programs drop on a window, beyond files with paths: a browser hands a picture as file contents without a path, and a link as an address.
//! Tauri's own handler takes files with paths only, so on Windows a drop target of Curio's own takes its place and tells the page `drop-in` events.
//! On Linux Tauri's handler passes a browser's addresses on as if they were paths; the page tells them apart.

// the platforms without a drop target of Curio's own use none of what follows but `take`
#![cfg_attr(not(windows), allow(dead_code))]

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Files dropped as contents are written here before they are added, each drop in a folder of its own.
fn drop_dir() -> PathBuf {
    crate::core::system::app_dirs::data_dir().join("dropped")
}

// a drop's files are added by then; what is older is left from an earlier drop
const KEEP_DROPPED: Duration = Duration::from_secs(24 * 3600);

/// A name a dropped file can be written under: its last part, without what Windows refuses in names.
fn file_name(given: &str, n: usize) -> String {
    let last = given.rsplit(['/', '\\']).next().unwrap_or("").trim();
    let clean: String = last.chars().map(|c| if c.is_control() || "<>:\"|?*".contains(c) { '_' } else { c }).collect();
    let clean = clean.trim_matches(['.', ' ']).to_string();
    if clean.is_empty() { format!("dropped {}", n + 1) } else { clean }
}

/// Writes files dropped as contents into a new folder and returns their paths; names met twice are numbered.
pub fn save(files: Vec<(String, Vec<u8>)>) -> Vec<PathBuf> {
    let root = drop_dir();
    if let Ok(old) = std::fs::read_dir(&root) {
        let now = SystemTime::now();
        for e in old.flatten() {
            let stale = e.metadata().and_then(|m| m.modified()).is_ok_and(|t| now.duration_since(t).unwrap_or_default() > KEEP_DROPPED);
            if stale {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
    let stamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos();
    let dir = root.join(stamp.to_string());
    if std::fs::create_dir_all(&dir).is_err() {
        return vec![];
    }
    let mut out = vec![];
    for (i, (name, bytes)) in files.into_iter().enumerate() {
        let name = file_name(&name, i);
        let path = free_path(&dir, &name);
        if std::fs::write(&path, bytes).is_ok() {
            out.push(path);
        }
    }
    out
}

fn free_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    (2..).map(|n| dir.join(format!("{stem} ({n}){ext}"))).find(|p| !p.exists()).unwrap()
}

#[derive(serde::Serialize, Clone)]
struct DropIn {
    #[serde(rename = "type")]
    kind: &'static str,
    paths: Vec<PathBuf>,
    urls: Vec<String>,
    /// the paths are copies Curio wrote, to be copied in and never linked
    made: bool,
    position: Option<Pos>,
}

#[derive(serde::Serialize, Clone)]
struct Pos {
    x: i32,
    y: i32,
}

/// Takes over what is dropped on this window from other programs, where Curio has a drop target of its own.
pub fn take(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    win::install(window);
    #[cfg(not(windows))]
    let _ = window;
}

#[cfg(windows)]
mod win {
    use super::{DropIn, Pos};
    use std::cell::Cell;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use tauri::{Emitter, EventTarget, WebviewWindow};
    use windows::Win32::Foundation::{DRAGDROP_E_INVALIDHWND, HGLOBAL, HWND, LPARAM, POINT, POINTL, S_OK};
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, STGMEDIUM, TYMED_HGLOBAL, TYMED_ISTREAM};
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows::Win32::System::Ole::{CF_HDROP, CF_UNICODETEXT, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_NONE, IDropTarget, IDropTarget_Impl, RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop};
    use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
    use windows::Win32::UI::Shell::{DragQueryFileW, FD_ATTRIBUTES, FD_FILESIZE, FILEDESCRIPTORW, HDROP};
    use windows::Win32::UI::WindowsAndMessaging::EnumChildWindows;
    use windows::core::{BOOL, Ref, implement, w};

    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;

    struct Formats {
        group: u16,
        contents: u16,
        url: u16,
    }

    fn formats() -> Formats {
        unsafe { Formats { group: RegisterClipboardFormatW(w!("FileGroupDescriptorW")) as u16, contents: RegisterClipboardFormatW(w!("FileContents")) as u16, url: RegisterClipboardFormatW(w!("UniformResourceLocatorW")) as u16 } }
    }

    fn format(cf: u16, lindex: i32, tymed: i32) -> FORMATETC {
        FORMATETC { cfFormat: cf, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0, lindex, tymed: tymed as u32 }
    }

    unsafe fn has(data: &IDataObject, cf: u16) -> bool {
        unsafe { data.QueryGetData(&format(cf, -1, TYMED_HGLOBAL.0)) == S_OK }
    }

    unsafe fn global(h: HGLOBAL) -> Vec<u8> {
        unsafe {
            let p = GlobalLock(h) as *const u8;
            if p.is_null() {
                return vec![];
            }
            let v = std::slice::from_raw_parts(p, GlobalSize(h)).to_vec();
            let _ = GlobalUnlock(h);
            v
        }
    }

    /// The medium's bytes, from memory or a stream; the medium is released.
    unsafe fn bytes(mut m: STGMEDIUM) -> Option<Vec<u8>> {
        unsafe {
            let out = if m.tymed == TYMED_HGLOBAL.0 as u32 {
                Some(global(m.u.hGlobal))
            } else if m.tymed == TYMED_ISTREAM.0 as u32 {
                (*m.u.pstm).as_ref().map(|s| {
                    let mut out = Vec::new();
                    let mut buf = vec![0u8; 1 << 16];
                    loop {
                        let mut got = 0u32;
                        if s.Read(buf.as_mut_ptr() as _, buf.len() as u32, Some(&mut got)).is_err() || got == 0 {
                            break;
                        }
                        out.extend_from_slice(&buf[..got as usize]);
                    }
                    out
                })
            } else {
                None
            };
            ReleaseStgMedium(&mut m);
            out
        }
    }

    unsafe fn paths(data: &IDataObject) -> Vec<PathBuf> {
        unsafe {
            let Ok(mut m) = data.GetData(&format(CF_HDROP.0, -1, TYMED_HGLOBAL.0)) else { return vec![] };
            let hdrop = HDROP(m.u.hGlobal.0);
            let mut out = vec![];
            for i in 0..DragQueryFileW(hdrop, u32::MAX, None) {
                let n = DragQueryFileW(hdrop, i, None) as usize;
                let mut buf = vec![0u16; n + 1];
                DragQueryFileW(hdrop, i, Some(&mut buf));
                out.push(OsString::from_wide(&buf[..n]).into());
            }
            ReleaseStgMedium(&mut m);
            out
        }
    }

    /// Files offered as contents, as a browser offers a picture: their names and bytes.
    unsafe fn contents(data: &IDataObject, f: &Formats) -> Vec<(String, Vec<u8>)> {
        unsafe {
            let Some(group) = data.GetData(&format(f.group, -1, TYMED_HGLOBAL.0)).ok().and_then(|m| bytes(m)) else { return vec![] };
            if group.len() < 4 {
                return vec![];
            }
            let count = u32::from_le_bytes([group[0], group[1], group[2], group[3]]) as usize;
            let size = std::mem::size_of::<FILEDESCRIPTORW>();
            let mut out = vec![];
            for i in 0..count {
                let at = 4 + i * size;
                if at + size > group.len() {
                    break;
                }
                let d: FILEDESCRIPTORW = std::ptr::read_unaligned(group[at..].as_ptr() as *const FILEDESCRIPTORW);
                let flags = d.dwFlags;
                if flags & FD_ATTRIBUTES.0 as u32 != 0 && d.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
                    continue;
                }
                let name = { d.cFileName };
                let name = String::from_utf16_lossy(&name[..name.iter().position(|&c| c == 0).unwrap_or(name.len())]);
                let Some(mut body) = data.GetData(&format(f.contents, i as i32, TYMED_HGLOBAL.0 | TYMED_ISTREAM.0)).ok().and_then(|m| bytes(m)) else { continue };
                // memory handed over can be longer than the file
                if flags & FD_FILESIZE.0 as u32 != 0 {
                    body.truncate(((d.nFileSizeHigh as usize) << 32) | d.nFileSizeLow as usize);
                }
                out.push((name, body));
            }
            out
        }
    }

    /// Web addresses offered: a link's or a picture's.
    unsafe fn urls(data: &IDataObject, f: &Formats) -> Vec<String> {
        for cf in [f.url, CF_UNICODETEXT.0] {
            let Some(raw) = (unsafe { data.GetData(&format(cf, -1, TYMED_HGLOBAL.0)).ok().and_then(|m| bytes(m)) }) else { continue };
            let wide: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
            let text = String::from_utf16_lossy(&wide);
            let list: Vec<String> = text.lines().map(str::trim).filter(|l| l.starts_with("https://") || l.starts_with("http://")).map(String::from).collect();
            if !list.is_empty() {
                return list;
            }
        }
        vec![]
    }

    #[implement(IDropTarget)]
    struct Target {
        hwnd: HWND,
        window: WebviewWindow,
        effect: Cell<DROPEFFECT>,
    }

    impl Target {
        fn tell(&self, kind: &'static str, paths: Vec<PathBuf>, urls: Vec<String>, made: bool, pt: Option<&POINTL>) {
            let position = pt.map(|pt| {
                let mut p = POINT { x: pt.x, y: pt.y };
                let _ = unsafe { ScreenToClient(self.hwnd, &mut p) };
                Pos { x: p.x, y: p.y }
            });
            let _ = self.window.emit_to(EventTarget::webview(self.window.label()), "drop-in", DropIn { kind, paths, urls, made, position });
        }
    }

    impl IDropTarget_Impl for Target_Impl {
        fn DragEnter(&self, data: Ref<'_, IDataObject>, _keys: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> windows::core::Result<()> {
            let f = formats();
            let (takes, files) = match data.as_ref() {
                Some(d) => unsafe { (has(d, CF_HDROP.0) || has(d, f.group) || has(d, f.url) || !urls(d, &f).is_empty(), paths(d)) },
                None => (false, vec![]),
            };
            let allowed = unsafe { *effect };
            let chosen = if !takes { DROPEFFECT_NONE } else if allowed.0 & DROPEFFECT_COPY.0 != 0 { DROPEFFECT_COPY } else { DROPEFFECT(allowed.0 & DROPEFFECT_LINK.0) };
            self.effect.set(chosen);
            unsafe { *effect = chosen };
            if chosen != DROPEFFECT_NONE {
                self.tell("enter", files, vec![], false, Some(pt));
            }
            Ok(())
        }

        fn DragOver(&self, _keys: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> windows::core::Result<()> {
            let chosen = self.effect.get();
            unsafe { *effect = chosen };
            if chosen != DROPEFFECT_NONE {
                self.tell("over", vec![], vec![], false, Some(pt));
            }
            Ok(())
        }

        fn DragLeave(&self) -> windows::core::Result<()> {
            if self.effect.get() != DROPEFFECT_NONE {
                self.tell("leave", vec![], vec![], false, None);
            }
            self.effect.set(DROPEFFECT_NONE);
            Ok(())
        }

        fn Drop(&self, data: Ref<'_, IDataObject>, _keys: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> windows::core::Result<()> {
            let chosen = self.effect.replace(DROPEFFECT_NONE);
            unsafe { *effect = chosen };
            if chosen == DROPEFFECT_NONE {
                return Ok(());
            }
            let f = formats();
            let (files, links, made) = match data.as_ref() {
                Some(d) => unsafe {
                    let files = paths(d);
                    if !files.is_empty() {
                        (files, vec![], false)
                    } else {
                        // the bytes the browser holds come first: nothing is fetched again, and pictures behind a sign-in come too
                        let made = super::save(contents(d, &f));
                        if made.is_empty() { (vec![], urls(d, &f), false) } else { (made, vec![], true) }
                    }
                },
                None => (vec![], vec![], false),
            };
            self.tell("drop", files, links, made, Some(pt));
            Ok(())
        }
    }

    /// Puts Curio's drop target on the web view's own windows in place of Tauri's, as Tauri puts its own there.
    pub fn install(window: &WebviewWindow) {
        let w = window.clone();
        let _ = window.run_on_main_thread(move || {
            let Ok(parent) = w.hwnd() else { return };
            let mut children: Vec<HWND> = vec![];
            unsafe extern "system" fn each(h: HWND, l: LPARAM) -> BOOL {
                unsafe { (*(l.0 as *mut Vec<HWND>)).push(h) };
                true.into()
            }
            unsafe {
                let _ = EnumChildWindows(Some(HWND(parent.0 as _)), Some(each), LPARAM(&mut children as *mut _ as isize));
            }
            for h in children {
                let target: IDropTarget = Target { hwnd: h, window: w.clone(), effect: Cell::new(DROPEFFECT_NONE) }.into();
                unsafe {
                    if RevokeDragDrop(h) != Err(DRAGDROP_E_INVALIDHWND.into()) {
                        let _ = RegisterDragDrop(h, &target);
                    }
                }
            }
        });
    }
}
