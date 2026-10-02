use super::*;

#[test]
fn parses_category_and_name() {
    let parsed = ActivityName::parse("Work: Meeting").unwrap();
    assert_eq!(parsed.category.as_deref(), Some("Work"));
    assert_eq!(parsed.name, "Meeting");
}

#[test]
fn parses_name_without_category() {
    let parsed = ActivityName::parse("  Lunch ").unwrap();
    assert_eq!(parsed.category, None);
    assert_eq!(parsed.name, "Lunch");
}

#[test]
fn splits_only_at_first_separator() {
    let parsed = ActivityName::parse("Work: Review: PR").unwrap();
    assert_eq!(parsed.category.as_deref(), Some("Work"));
    assert_eq!(parsed.name, "Review: PR");
}

#[test]
fn colon_without_space_is_part_of_name() {
    let parsed = ActivityName::parse("10:30 standup").unwrap();
    assert_eq!(parsed.category, None);
    assert_eq!(parsed.name, "10:30 standup");
}

#[test]
fn empty_category_is_dropped() {
    let parsed = ActivityName::parse(": Meeting").unwrap();
    assert_eq!(parsed.category, None);
    assert_eq!(parsed.name, "Meeting");
}

#[test]
fn rejects_empty_names() {
    assert!(matches!(ActivityName::parse("   "), Err(Error::EmptyName)));
    assert!(matches!(
        ActivityName::parse("Work: "),
        Err(Error::EmptyName)
    ));
}

#[test]
fn displays_round_trip() {
    for input in ["Work: Meeting", "Lunch"] {
        assert_eq!(ActivityName::parse(input).unwrap().to_string(), input);
    }
}
