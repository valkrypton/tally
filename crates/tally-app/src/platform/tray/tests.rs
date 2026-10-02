use jiff::Timestamp;

use super::*;

fn alpha(pixels: &[u8], x: u32, y: u32) -> u8 {
    pixels[((y * ICON_SIZE + x) * 4 + 3) as usize]
}

fn look(mode: TrayMode, failing: bool) -> TrayLook {
    TrayLook { mode, failing }
}

#[test]
fn tracking_is_a_filled_square() {
    let pixels = icon_pixels(look(TrayMode::Tracking, false), [0, 0, 0]);
    assert_eq!(pixels.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
    assert_eq!(alpha(&pixels, 16, 16), 255);
    assert_eq!(alpha(&pixels, 1, 1), 0);
}

#[test]
fn off_schedule_is_hollow() {
    let pixels = icon_pixels(look(TrayMode::OffSchedule, false), [0, 0, 0]);
    assert_eq!(alpha(&pixels, 16, 16), 0);
    assert_eq!(alpha(&pixels, 7, 16), 255);
}

#[test]
fn paused_shows_two_bars() {
    let pixels = icon_pixels(look(TrayMode::Paused, false), [0, 0, 0]);
    assert_eq!(alpha(&pixels, 10, 16), 255);
    assert_eq!(alpha(&pixels, 16, 16), 0);
    assert_eq!(alpha(&pixels, 20, 16), 255);
}

#[test]
fn failing_adds_a_corner_badge() {
    for mode in [TrayMode::Tracking, TrayMode::Paused, TrayMode::OffSchedule] {
        let plain = icon_pixels(look(mode, false), [0, 0, 0]);
        let badged = icon_pixels(look(mode, true), [0, 0, 0]);
        assert_eq!(alpha(&badged, 25, 6), 255, "{mode:?}");
        assert_ne!(plain, badged, "{mode:?}");
    }
}

#[test]
fn mode_follows_scheduler_state() {
    let at = Timestamp::UNIX_EPOCH;
    assert_eq!(
        TrayMode::from_state(&State::Paused { until: None }),
        TrayMode::Paused
    );
    assert_eq!(
        TrayMode::from_state(&State::OffSchedule),
        TrayMode::OffSchedule
    );
    assert_eq!(
        TrayMode::from_state(&State::Tracking { next_prompt_at: at }),
        TrayMode::Tracking
    );
    assert_eq!(
        TrayMode::from_state(&State::Away { since: at }),
        TrayMode::Tracking
    );
}
