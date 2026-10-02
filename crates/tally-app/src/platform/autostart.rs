//! Start at login, via `auto-launch` (Launch Agent on macOS, the Run registry key on
//! Windows, an XDG autostart entry on Linux).

use anyhow::Context;
use auto_launch::{AutoLaunch, AutoLaunchBuilder};

fn launcher() -> anyhow::Result<AutoLaunch> {
    let exe = std::env::current_exe().context("could not find tally's own path")?;
    let exe = exe.to_str().context("tally's path is not valid UTF-8")?;
    AutoLaunchBuilder::new()
        .set_app_name("tally")
        .set_app_path(exe)
        .build()
        .context("could not set up start at login")
}

/// Makes the OS match the setting. Returns an error message for the Settings screen.
pub fn apply(enabled: bool) -> Result<(), String> {
    let result = launcher().and_then(|launcher| {
        let current = launcher.is_enabled().unwrap_or(false);
        match (enabled, current) {
            (true, false) => launcher.enable().context("could not enable start at login"),
            (false, true) => launcher
                .disable()
                .context("could not disable start at login"),
            _ => Ok(()),
        }
    });
    result.map_err(|error| {
        tracing::warn!(error = %format!("{error:#}"), "start at login");
        format!("{error:#}")
    })
}
