//! macOS: a process is in the Dock only while one of its windows is, so a library kept in the menu bar or held in the background for the API has no icon there.

use std::time::{Duration, Instant};
use tauri::Manager;

const LOOK_EVERY: Duration = Duration::from_millis(400);
/// tao ignores hiding the icon within a second of showing it (a macOS bug leaves extra icons otherwise), so hiding is asked again for a while.
const HIDE_AGAIN_FOR: Duration = Duration::from_millis(2500);

/// Shows or hides this process's Dock icon as its windows come and go, until the process ends.
pub(in crate::desktop) fn follow_windows(app: &tauri::AppHandle) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("dock".into()).spawn(move || {
        let mut shown: Option<bool> = None;
        let mut changed = Instant::now();
        loop {
            // a minimised window is in the Dock too
            let any = app.webview_windows().values().any(|w| w.is_visible().unwrap_or(false) || w.is_minimized().unwrap_or(false));
            if shown != Some(any) {
                shown = Some(any);
                changed = Instant::now();
                let _ = app.set_dock_visibility(any);
            } else if !any && changed.elapsed() < HIDE_AGAIN_FOR {
                let _ = app.set_dock_visibility(false);
            }
            std::thread::sleep(LOOK_EVERY);
        }
    });
}
