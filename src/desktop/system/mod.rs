//! What the shell does with the operating system: starting with it, drops from other programs, the Dock, window materials, notifications, the curio:// scheme, the tray and pages drawn off screen.

pub(super) mod autostart;
#[cfg(target_os = "macos")]
pub(super) mod dock;
pub(super) mod dropin;
pub mod material;
pub(super) mod notifications;
pub mod scheme;
pub(super) mod tray;
#[cfg(any(windows, target_os = "linux"))]
pub(super) mod webshot;
