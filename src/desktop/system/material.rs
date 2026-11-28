//! The material the system compositor draws behind a window's see-through page.

use serde::Deserialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Material {
    #[default]
    Off,
    Mica,
    Acrylic,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Look {
    pub material: Material,
    /// dark title-bar and material tint; `None` leaves the system's choice
    pub dark: Option<bool>,
    /// the material stays as on an active window while another window is in front
    pub keep: bool,
}

/// The materials this system can draw, as the page names them.
pub fn materials() -> Vec<&'static str> {
    if imp::supported() { vec!["off", "mica", "acrylic"] } else { vec!["off"] }
}

/// Curio's windows draw their own frame and buttons over a see-through page, a Mac's too (`WindowControls.vue`): Tauri ignores where the system's buttons go in a window that can hold child web views (the `unstable` feature).
pub(crate) fn frame<'a, R: tauri::Runtime, M: tauri::Manager<R>>(b: tauri::WebviewWindowBuilder<'a, R, M>) -> tauri::WebviewWindowBuilder<'a, R, M> {
    b.decorations(false).transparent(true)
}

/// The window at `w` by `h`, or filling the screen's free area where that is smaller: no system fits a window without its frame to the screen, and one larger than it opens with its edges off it.
pub(crate) fn sized<'a>(b: tauri::WebviewWindowBuilder<'a, tauri::Wry, tauri::AppHandle>, app: &tauri::AppHandle, w: f64, h: f64) -> tauri::WebviewWindowBuilder<'a, tauri::Wry, tauri::AppHandle> {
    match app.primary_monitor().ok().flatten().map(|m| m.work_area().size.to_logical::<f64>(m.scale_factor())) {
        Some(area) if area.width < w || area.height < h => b.inner_size(w.min(area.width), h.min(area.height)).center(),
        _ => b.inner_size(w, h),
    }
}

pub fn apply(window: &tauri::WebviewWindow, look: Look) {
    // the window keeps a mode chosen in Curio: on a change to the system's settings (a new wallpaper, its accent colour) the window layer otherwise sets its light or dark back to the system's, and Mica turns light under a dark page
    let _ = window.set_theme(look.dark.map(|d| if d { tauri::Theme::Dark } else { tauri::Theme::Light }));
    imp::apply(window, look)
}

/// The window has gone to the background: one set to keep its material is drawn as active again.
pub fn deactivated(window: &tauri::Window) {
    imp::deactivated(window)
}

#[cfg(windows)]
mod imp {
    use super::{Look, Material};
    use windows::core::BOOL;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE, DWM_SYSTEMBACKDROP_TYPE};
    use windows::Win32::UI::WindowsAndMessaging::{DefWindowProcW, WM_NCACTIVATE};

    /// the windows whose material stays as on an active window
    static KEEP: std::sync::Mutex<Vec<isize>> = std::sync::Mutex::new(Vec::new());

    /// `DWMWA_SYSTEMBACKDROP_TYPE` arrived in Windows 11 22H2.
    const FIRST_BUILD: u32 = 22621;

    pub fn supported() -> bool {
        static SUPPORTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *SUPPORTED.get_or_init(|| {
            use winreg::enums::HKEY_LOCAL_MACHINE;
            let key = winreg::RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");
            key.and_then(|k| k.get_value::<String, _>("CurrentBuildNumber")).ok().and_then(|b| b.parse::<u32>().ok()).is_some_and(|b| b >= FIRST_BUILD)
        })
    }

    pub fn apply(window: &tauri::WebviewWindow, look: Look) {
        let Ok(hwnd) = window.hwnd() else { return };
        let h = hwnd.0 as isize;
        if let Ok(mut keep) = KEEP.lock() {
            keep.retain(|&x| x != h);
            if look.keep && look.material != Material::Off {
                keep.push(h);
            }
        }
        let _ = window.run_on_main_thread(move || unsafe { set(HWND(h as _), look) });
    }

    // The compositor draws the material of a window whose frame looks inactive as a flat grey. Telling the default handling that the frame is active again, once the window has really gone to the background, keeps the material; the window itself stays inactive.
    pub fn deactivated(window: &tauri::Window) {
        let Ok(hwnd) = window.hwnd() else { return };
        let h = hwnd.0 as isize;
        if !KEEP.lock().is_ok_and(|k| k.contains(&h)) {
            return;
        }
        let _ = window.run_on_main_thread(move || unsafe {
            DefWindowProcW(HWND(h as _), WM_NCACTIVATE, WPARAM(1), LPARAM(0));
        });
    }

    unsafe fn set(hwnd: HWND, look: Look) {
        if let Some(dark) = look.dark {
            let v = BOOL::from(dark);
            let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &v as *const _ as _, size_of::<BOOL>() as u32);
        }
        if !supported() {
            return;
        }
        let backdrop: DWM_SYSTEMBACKDROP_TYPE = match look.material {
            Material::Off => DWMSBT_NONE,
            Material::Mica => DWMSBT_MAINWINDOW,
            Material::Acrylic => DWMSBT_TRANSIENTWINDOW,
        };
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, &backdrop as *const _ as _, size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32);
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Look;

    pub fn supported() -> bool {
        false
    }

    pub fn apply(_window: &tauri::WebviewWindow, _look: Look) {}

    pub fn deactivated(_window: &tauri::Window) {}
}
