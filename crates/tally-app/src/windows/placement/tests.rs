use super::*;

const POPUP: Size = Size::new(400.0, 340.0);
const SCREEN: Size = Size::new(1512.0, 982.0);

fn icon(x: f64, y: f64) -> TrayRect {
    // A 2x display: a 30x24 pt icon is 60x48 physical pixels.
    TrayRect {
        x,
        y,
        width: 60.0,
        height: 48.0,
    }
}

#[test]
fn centres_under_the_icon_on_macos() {
    let origin = popover_origin(Some(icon(2000.0, 0.0)), 2.0, POPUP, Some(SCREEN), Edge::Top);
    assert_eq!(origin, Point::new(1015.0 - 200.0, 24.0 + 6.0));
}

#[test]
fn keeps_inside_the_right_edge() {
    let origin = popover_origin(Some(icon(2960.0, 0.0)), 2.0, POPUP, Some(SCREEN), Edge::Top);
    assert_eq!(origin.x, 1512.0 - 400.0 - 8.0);
}

#[test]
fn keeps_inside_the_left_edge() {
    let origin = popover_origin(Some(icon(0.0, 0.0)), 2.0, POPUP, Some(SCREEN), Edge::Top);
    assert_eq!(origin.x, 8.0);
}

#[test]
fn opens_above_a_bottom_taskbar() {
    let rect = TrayRect {
        x: 1700.0,
        y: 1040.0,
        width: 24.0,
        height: 40.0,
    };
    let screen = Size::new(1920.0, 1080.0);
    let origin = popover_origin(Some(rect), 1.0, POPUP, Some(screen), Edge::Bottom);
    assert_eq!(origin, Point::new(1712.0 - 200.0, 1040.0 - 340.0 - 6.0));
}

#[test]
fn falls_back_to_the_corner_without_an_icon() {
    let top = popover_origin(None, 2.0, POPUP, Some(SCREEN), Edge::Top);
    assert_eq!(top, Point::new(1512.0 - 400.0 - 8.0, 32.0));
    let screen = Size::new(1920.0, 1080.0);
    let bottom = popover_origin(None, 1.0, POPUP, Some(screen), Edge::Bottom);
    assert_eq!(
        bottom,
        Point::new(1920.0 - 400.0 - 8.0, 1080.0 - 340.0 - 56.0)
    );
}

#[test]
fn survives_a_zero_scale_and_no_screen() {
    let origin = popover_origin(Some(icon(100.0, 0.0)), 0.0, POPUP, None, Edge::Top);
    assert!(origin.x >= 8.0);
}
