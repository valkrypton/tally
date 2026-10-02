//! The tray (menu bar) icon and its right-click menu.

use anyhow::Context;
use tally_core::scheduler::State;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub const MENU_ASK_NOW: &str = "ask-now";
pub const MENU_PAUSE: &str = "pause";
pub const MENU_OPEN: &str = "open";
pub const MENU_QUIT: &str = "quit";

/// The tray icon's bounds in physical pixels, top-left origin, as tray-icon reports them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrayRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl From<tray_icon::Rect> for TrayRect {
    fn from(rect: tray_icon::Rect) -> Self {
        Self {
            x: rect.position.x,
            y: rect.position.y,
            width: f64::from(rect.size.width),
            height: f64::from(rect.size.height),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayMode {
    Tracking,
    Paused,
    OffSchedule,
}

impl TrayMode {
    pub fn from_state(state: &State) -> Self {
        match state {
            State::Paused { .. } => TrayMode::Paused,
            State::OffSchedule => TrayMode::OffSchedule,
            _ => TrayMode::Tracking,
        }
    }
}

/// What the icon shows: the mode, plus a badge while writes are failing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayLook {
    pub mode: TrayMode,
    pub failing: bool,
}

pub struct Tray {
    icon: TrayIcon,
    pause: MenuItem,
    look: Option<TrayLook>,
}

impl Tray {
    /// Must be called on the main thread after the event loop has started.
    pub fn new() -> anyhow::Result<Self> {
        let pause = MenuItem::with_id(MENU_PAUSE, "Pause", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &MenuItem::with_id(MENU_ASK_NOW, "Ask now", true, None),
            &pause,
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(MENU_OPEN, "Open tally", true, None),
            &MenuItem::with_id(MENU_QUIT, "Quit tally", true, None),
        ])
        .context("could not build the tray menu")?;
        let look = TrayLook {
            mode: TrayMode::OffSchedule,
            failing: false,
        };
        let builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("tally");
        // macOS draws template icons in the menu bar's own colour (light or dark).
        let builder = if cfg!(target_os = "macos") {
            builder.with_icon_templated(icon(look)?)
        } else {
            builder.with_icon(icon(look)?)
        };
        let icon = builder.build().context("could not create the tray icon")?;
        Ok(Self {
            icon,
            pause,
            look: Some(look),
        })
    }

    /// Updates the icon and the Pause/Resume item. Cheap when nothing changed.
    pub fn show(&mut self, look: TrayLook) {
        if self.look == Some(look) {
            return;
        }
        self.pause.set_text(if look.mode == TrayMode::Paused {
            "Resume"
        } else {
            "Pause"
        });
        match icon(look) {
            Ok(image) => {
                let result = if cfg!(target_os = "macos") {
                    self.icon.set_icon_templated(Some(image))
                } else {
                    self.icon.set_icon(Some(image))
                };
                if let Err(error) = result {
                    tracing::warn!(%error, "could not update tray icon");
                }
            }
            Err(error) => tracing::warn!(%error, "could not draw tray icon"),
        }
        self.look = Some(look);
    }

    /// Where the icon is now, for placing popovers.
    pub fn rect(&self) -> Option<TrayRect> {
        self.icon.rect().map(TrayRect::from)
    }
}

pub const ICON_SIZE: u32 = 32;

fn icon(look: TrayLook) -> anyhow::Result<Icon> {
    // macOS template icons only use alpha; elsewhere draw in a light grey that reads on
    // dark and light taskbars.
    let color = if cfg!(target_os = "macos") {
        [0, 0, 0]
    } else {
        [0xE8, 0xEA, 0xED]
    };
    Icon::from_rgba(icon_pixels(look, color), ICON_SIZE, ICON_SIZE).context("invalid tray icon")
}

/// Draws the icon as RGBA: a filled rounded square while tracking (the tally mark), an
/// outlined square off schedule, two bars when paused, and a dot in the top-right
/// corner (cut out from the shape) while writes are failing.
pub fn icon_pixels(look: TrayLook, color: [u8; 3]) -> Vec<u8> {
    let size = ICON_SIZE as i32;
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let inside_rounded = |x: i32, y: i32, x0: i32, y0: i32, x1: i32, y1: i32, r: i32| {
        if x < x0 || x > x1 || y < y0 || y > y1 {
            return false;
        }
        let cx = x.clamp(x0 + r, x1 - r);
        let cy = y.clamp(y0 + r, y1 - r);
        (x - cx).pow(2) + (y - cy).pow(2) <= r * r
    };
    let badge = |x: i32, y: i32, extra: i32| (x - 25).pow(2) + (y - 6).pow(2) <= (5 + extra).pow(2);
    for y in 0..size {
        for x in 0..size {
            let shape = match look.mode {
                TrayMode::Tracking => inside_rounded(x, y, 6, 6, 25, 25, 4),
                TrayMode::OffSchedule => {
                    inside_rounded(x, y, 6, 6, 25, 25, 4) && !inside_rounded(x, y, 9, 9, 22, 22, 2)
                }
                TrayMode::Paused => {
                    inside_rounded(x, y, 8, 6, 13, 25, 1) || inside_rounded(x, y, 18, 6, 23, 25, 1)
                }
            };
            let on = if look.failing {
                badge(x, y, 0) || (shape && !badge(x, y, 2))
            } else {
                shape
            };
            if on {
                let i = ((y * size + x) * 4) as usize;
                pixels[i..i + 3].copy_from_slice(&color);
                pixels[i + 3] = 255;
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests;
