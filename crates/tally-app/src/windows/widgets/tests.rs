use super::*;

#[test]
fn bars_are_proportional() {
    assert_eq!(bar_portions(275, 395), (696, 304));
    assert_eq!(bar_portions(395, 395), (1000, 0));
    assert_eq!(bar_portions(500, 395), (1000, 0));
}

#[test]
fn empty_values_draw_an_empty_track() {
    assert_eq!(bar_portions(0, 395), (0, 1000));
    assert_eq!(bar_portions(10, 0), (0, 1000));
    assert_eq!(bar_portions(-5, 100), (0, 1000));
}

#[test]
fn tiny_values_still_show() {
    assert_eq!(bar_portions(1, 1_000_000), (1, 999));
}

#[test]
fn field_looks() {
    assert_eq!(look(true, true), FieldLook::Error);
    assert_eq!(look(false, true), FieldLook::Active);
    assert_eq!(look(false, false), FieldLook::Resting);
}
