//! tally: asks what you're doing, every few minutes, from the menu bar.

// Release builds on Windows are GUI apps: no console window opens next to tally.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod clock;
mod format;
mod icons;
mod logging;
mod pending;
mod platform;
mod status;
mod theme;
mod windows;

use app::App;
use platform::paths::Paths;

fn main() -> anyhow::Result<()> {
    let paths = Paths::locate()?;
    // If this fails, `App::boot` reports it in the data-file error window.
    let _ = paths.create_data_dir();
    let _log = logging::init(&paths);
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "starting");
    let _instance = match paths.lock_instance() {
        Ok(Some(lock)) => Some(lock),
        Ok(None) => {
            tracing::warn!("tally is already running; exiting");
            return Ok(());
        }
        // Can't tell (e.g. the folder is unwritable): carry on, boot shows the problem.
        Err(error) => {
            tracing::warn!(%error, "could not take the single-instance lock");
            None
        }
    };

    let mut daemon = iced::daemon(move || App::boot(paths.clone()), App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .style(App::style)
        .subscription(App::subscription)
        .default_font(theme::UI);
    for font in theme::FONT_FILES {
        daemon = daemon.font(font);
    }
    daemon.run()?;
    Ok(())
}
