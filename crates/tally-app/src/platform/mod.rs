//! OS integration: tray, idle time, global shortcut, start at login, data folder and
//! macOS activation policy. Everything here must run on the main thread unless noted.

pub mod autostart;
pub mod events;
pub mod hotkey;
pub mod idle;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod paths;
pub mod tray;

use std::path::Path;
use std::process::Command;

/// Opens a folder in Finder / Explorer / the file manager.
pub fn reveal(folder: &Path) {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    if let Err(error) = Command::new(program).arg(folder).spawn() {
        tracing::warn!(%error, folder = %folder.display(), "could not open folder");
    }
}

/// Makes tally a menu-bar-only app (no Dock icon). No-op off macOS.
pub fn hide_dock_icon() {
    #[cfg(target_os = "macos")]
    macos::set_accessory();
}
