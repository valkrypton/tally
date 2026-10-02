//! Where popovers go: under the tray icon on macOS (menu bar at the top), above it on
//! Windows and Linux (taskbar usually at the bottom).

use iced::{Point, Size};

use crate::platform::tray::TrayRect;

/// Gap between the tray icon and the popover, and the minimum gap to the screen edge.
const GAP: f32 = 6.0;
const MARGIN: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// The bar is at the top: open below the icon.
    Top,
    /// The bar is at the bottom: open above the icon.
    Bottom,
}

impl Edge {
    pub fn for_platform() -> Self {
        if cfg!(target_os = "macos") {
            Edge::Top
        } else {
            Edge::Bottom
        }
    }
}

/// The popover's top-left corner in logical pixels.
///
/// `anchor` is the tray icon in physical pixels (`scale` converts it), `screen` the
/// logical size of the monitor if known. Without an anchor (the tray icon's position
/// is unknown) the popover goes in the corner nearest the bar.
pub fn popover_origin(
    anchor: Option<TrayRect>,
    scale: f32,
    size: Size,
    screen: Option<Size>,
    edge: Edge,
) -> Point {
    let scale = if scale > 0.0 { f64::from(scale) } else { 1.0 };
    let screen_width = screen.map(|s| s.width);
    let screen_height = screen.map(|s| s.height);
    let clamp_x = |x: f32| match screen_width {
        Some(width) => x.min(width - size.width - MARGIN).max(MARGIN),
        None => x.max(MARGIN),
    };
    match anchor {
        Some(rect) => {
            let center = ((rect.x + rect.width / 2.0) / scale) as f32;
            let x = clamp_x(center - size.width / 2.0);
            let y = match edge {
                Edge::Top => ((rect.y + rect.height) / scale) as f32 + GAP,
                Edge::Bottom => ((rect.y / scale) as f32 - size.height - GAP).max(MARGIN),
            };
            Point::new(x, y)
        }
        None => {
            let x =
                clamp_x(screen_width.unwrap_or(size.width + 2.0 * MARGIN) - size.width - MARGIN);
            let y = match (edge, screen_height) {
                (Edge::Top, _) => 32.0,
                (Edge::Bottom, Some(height)) => (height - size.height - 56.0).max(MARGIN),
                (Edge::Bottom, None) => MARGIN,
            };
            Point::new(x, y)
        }
    }
}

#[cfg(test)]
mod tests;
