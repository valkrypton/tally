//! The tray (menu bar) icon and its right-click menu.

use anyhow::Context;
use tally_core::scheduler::State;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub const MENU_ASK_NOW: &str = "ask-now";
pub const MENU_PAUSE: &str = "pause";
pub const MENU_END_DAY: &str = "end-day";
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
            &MenuItem::with_id(MENU_END_DAY, "End day", true, None),
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

/// Draws the icon as RGBA, anti-aliased: the tally mark from the app icon (four strokes
/// and a slash) while tracking, the same mark dimmed off schedule, two bars when paused,
/// and a dot in the top-right corner (cut out from the shape) while writes are failing.
pub fn icon_pixels(look: TrayLook, color: [u8; 3]) -> Vec<u8> {
    let size = ICON_SIZE as usize;
    let mut pixels = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let mut alpha = match look.mode {
                TrayMode::Tracking => tally_mark(p),
                TrayMode::OffSchedule => tally_mark(p) * OFF_SCHEDULE_ALPHA,
                TrayMode::Paused => PAUSE_BARS
                    .iter()
                    .map(|&(a, b)| cover(distance(p, a, b), PAUSE_RADIUS))
                    .fold(0.0, f32::max),
            };
            if look.failing {
                let d = distance(p, BADGE, BADGE);
                alpha = (alpha * (1.0 - cover(d, BADGE_RADIUS + GAP))).max(cover(d, BADGE_RADIUS));
            }
            let i = (y * size + x) * 4;
            pixels[i..i + 3].copy_from_slice(&color);
            pixels[i + 3] = (alpha * 255.0).round() as u8;
        }
    }
    pixels
}

type Point = (f32, f32);

const STROKE_RADIUS: f32 = 1.75;
const GAP: f32 = 1.25;
const BARS: [(Point, Point); 4] = [
    ((7.75, 6.0), (7.75, 26.0)),
    ((13.25, 6.0), (13.25, 26.0)),
    ((18.75, 6.0), (18.75, 26.0)),
    ((24.25, 6.0), (24.25, 26.0)),
];
const SLASH: (Point, Point) = ((4.25, 22.25), (27.75, 9.75));
const OFF_SCHEDULE_ALPHA: f32 = 0.4;
const PAUSE_RADIUS: f32 = 2.25;
const PAUSE_BARS: [(Point, Point); 2] = [((11.5, 7.5), (11.5, 24.5)), ((20.5, 7.5), (20.5, 24.5))];
const BADGE: Point = (25.0, 6.0);
const BADGE_RADIUS: f32 = 5.0;

/// The strokes stop a little short of the slash, so the mark stays readable at 16pt.
fn tally_mark(p: Point) -> f32 {
    let to_slash = distance(p, SLASH.0, SLASH.1);
    let bars = BARS
        .iter()
        .map(|&(a, b)| cover(distance(p, a, b), STROKE_RADIUS))
        .fold(0.0, f32::max);
    let slash = cover(to_slash, STROKE_RADIUS);
    (bars * (1.0 - cover(to_slash, STROKE_RADIUS + GAP))).max(slash)
}

/// How much of a pixel a round-capped stroke covers, from the pixel centre's distance
/// to the stroke's centre line (a one-pixel ramp gives the anti-aliasing).
fn cover(distance: f32, radius: f32) -> f32 {
    (radius - distance + 0.5).clamp(0.0, 1.0)
}

/// Distance from `p` to the segment `a`–`b`.
fn distance(p: Point, a: Point, b: Point) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length_sq = dx * dx + dy * dy;
    let t = if length_sq == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length_sq).clamp(0.0, 1.0)
    };
    ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
}

#[cfg(test)]
mod tests;
