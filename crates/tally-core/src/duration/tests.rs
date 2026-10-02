use super::*;

#[test]
fn parses_set_values() {
    let cases = [
        ("90", 90 * 60),
        ("45m", 45 * 60),
        ("3h", 3 * 3600),
        ("1h 30", 90 * 60),
        ("1h30m", 90 * 60),
        ("1H 30M", 90 * 60),
        (" 2h 05 ", 125 * 60),
        ("0", 0),
        ("0h", 0),
    ];
    for (input, seconds) in cases {
        assert_eq!(
            parse_edit(input).unwrap(),
            DurationEdit::Set(seconds),
            "{input}"
        );
    }
}

#[test]
fn parses_relative_values() {
    assert_eq!(parse_edit("-30").unwrap(), DurationEdit::Subtract(30 * 60));
    assert_eq!(parse_edit("+15").unwrap(), DurationEdit::Add(15 * 60));
    assert_eq!(parse_edit("+1h 30").unwrap(), DurationEdit::Add(90 * 60));
}

#[test]
fn rejects_bad_input() {
    for input in [
        "", "  ", "3hh", "abc", "1h 75", "-0", "+0", "--5", "h", "1.5h", "-",
    ] {
        assert!(parse_edit(input).is_err(), "{input:?} should be rejected");
    }
}

#[test]
fn formats_hours_and_minutes() {
    assert_eq!(format_hm(0), "0h 00m");
    assert_eq!(format_hm(50 * 60), "0h 50m");
    assert_eq!(format_hm(3 * 3600 + 5 * 60 + 59), "3h 05m");
    assert_eq!(format_hm(-10), "0h 00m");
}
