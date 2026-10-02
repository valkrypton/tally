use super::*;

/// WCAG 2.x contrast ratio between two opaque colours.
fn contrast(a: Color, b: Color) -> f32 {
    fn channel(c: f32) -> f32 {
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    fn luminance(c: Color) -> f32 {
        0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
    }
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[test]
fn primary_and_muted_text_pass_aa_on_every_surface() {
    let surfaces = [
        CONSOLE_DEEP,
        CONSOLE_NIGHT,
        CONSOLE_RAISED,
        CONSOLE_HOVER,
        CONSOLE_FLOATING,
        CONSOLE_SELECTED,
    ];
    for surface in surfaces {
        for text in [TEXT_PRIMARY, TEXT_MUTED] {
            let ratio = contrast(text, surface);
            assert!(ratio >= 4.5, "{text:?} on {surface:?} is {ratio:.2}");
        }
    }
}

#[test]
fn amber_pairs_pass_aa() {
    assert!(contrast(CONSOLE_NIGHT, LAMP_AMBER) >= 4.5);
    assert!(contrast(CONSOLE_NIGHT, LAMP_AMBER_HOVER) >= 4.5);
    assert!(contrast(LAMP_AMBER_INK, LAMP_AMBER_WASH) >= 4.5);
    assert!(contrast(LAMP_AMBER_INK_MUTED, LAMP_AMBER_WASH) >= 4.5);
    assert!(contrast(ALERT, ALERT_WASH) >= 4.5);
}

#[test]
fn placeholder_passes_inside_fields() {
    assert!(contrast(TEXT_PLACEHOLDER, CONSOLE_RAISED) >= 4.5);
    assert!(contrast(TEXT_PLACEHOLDER, CONSOLE_NIGHT) >= 4.5);
}

#[test]
fn fonts_are_bundled() {
    for bytes in FONT_FILES {
        // Every TrueType file starts with the 0x00010000 sfnt version.
        assert_eq!(&bytes[..4], &[0, 1, 0, 0]);
    }
}
