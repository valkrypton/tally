use iced::keyboard::Modifiers;

use super::*;

fn row(id: ActivityId, name: &str, archived: bool, minutes: i64) -> ActivityRow {
    ActivityRow {
        id,
        name: ActivityName::parse(name).unwrap(),
        archived,
        total_seconds: minutes * 60,
    }
}

/// In the store's order: uncategorised first, then by category and name.
fn rows() -> Vec<ActivityRow> {
    vec![
        row(1, "Gym", true, 240),
        row(2, "Lunch", false, 900),
        row(3, "Study: Rust book", false, 765),
        row(4, "Work: Code review", false, 2530),
        row(5, "Work: Meeting", false, 1110),
        row(6, "Work: Meetings", false, 200),
    ]
}

fn activities() -> Activities {
    let mut activities = Activities::default();
    activities.set_rows(rows());
    activities
}

fn key(key: Key, modifiers: Modifiers) -> KeyPress {
    KeyPress::new(key, modifiers)
}

#[test]
fn groups_list_categories_then_no_category() {
    let rows = rows();
    let groups = groups(&rows, "", false);
    let names: Vec<Option<&str>> = groups.iter().map(|g| g.category.as_deref()).collect();
    assert_eq!(names, [Some("Study"), Some("Work"), None]);
    assert_eq!(groups[1].seconds, (2530 + 1110 + 200) * 60);
    assert_eq!(groups[2].rows.len(), 1, "archived Gym is hidden");
}

#[test]
fn show_archived_and_search_filter() {
    let rows = rows();
    assert_eq!(groups(&rows, "", true).last().unwrap().rows.len(), 2);
    let found = groups(&rows, "meet", false);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rows.len(), 2);
    assert!(groups(&rows, "zzz", true).is_empty());
}

#[test]
fn finds_names_case_insensitively() {
    let rows = rows();
    assert_eq!(
        find_by_name(&rows, &ActivityName::parse("work: meeting").unwrap()),
        Some(5)
    );
    assert_eq!(
        find_by_name(&rows, &ActivityName::parse("Nope").unwrap()),
        None
    );
}

#[test]
fn rename_emits_the_new_name() {
    let mut activities = activities();
    assert_eq!(
        activities.update(Message::Rename(6)),
        Some(Outcome::Focus(editor_id()))
    );
    activities.update(Message::EditorInput("Work: Standups".into()));
    assert_eq!(
        activities.update(Message::SubmitEditor),
        Some(Outcome::Rename {
            id: 6,
            to: ActivityName::parse("Work: Standups").unwrap()
        })
    );
}

#[test]
fn a_rename_conflict_offers_merge() {
    let mut activities = activities();
    activities.update(Message::Rename(6));
    activities.update(Message::EditorInput("Work: Meeting".into()));
    activities.update(Message::SubmitEditor);
    activities.conflict(5);
    // Enter now confirms the merge.
    assert_eq!(
        activities.update(Message::SubmitEditor),
        Some(Outcome::Merge { from: 6, into: 5 })
    );
}

#[test]
fn merge_into_looks_up_the_target() {
    let mut activities = activities();
    activities.update(Message::MergeInto(6));
    activities.update(Message::EditorInput("work: meeting".into()));
    assert_eq!(activities.update(Message::SubmitEditor), None);
    assert_eq!(
        activities.update(Message::ConfirmMerge),
        Some(Outcome::Merge { from: 6, into: 5 })
    );
}

#[test]
fn merge_into_unknown_or_self_is_a_problem() {
    let mut activities = activities();
    activities.update(Message::MergeInto(6));
    activities.update(Message::EditorInput("Nope".into()));
    assert_eq!(activities.update(Message::SubmitEditor), None);
    assert!(activities.editor.as_ref().unwrap().problem.is_some());
    activities.update(Message::EditorInput("Work: Meetings".into()));
    assert_eq!(activities.update(Message::SubmitEditor), None);
    assert!(activities.editor.as_ref().unwrap().problem.is_some());
}

#[test]
fn empty_names_are_flagged() {
    let mut activities = activities();
    activities.update(Message::Rename(2));
    activities.update(Message::EditorInput("  ".into()));
    assert_eq!(activities.update(Message::SubmitEditor), None);
    assert!(activities.editor.as_ref().unwrap().problem.is_some());
}

#[test]
fn archive_and_restore() {
    let mut activities = activities();
    assert_eq!(
        activities.update(Message::Archive(2)),
        Some(Outcome::SetArchived {
            id: 2,
            archived: true
        })
    );
    assert_eq!(
        activities.update(Message::Restore(1)),
        Some(Outcome::SetArchived {
            id: 1,
            archived: false
        })
    );
}

#[test]
fn keyboard_selects_renames_and_archives() {
    let mut activities = activities();
    let down = key(Key::Named(Named::ArrowDown), Modifiers::empty());
    activities.key(&down);
    activities.key(&down);
    assert_eq!(activities.selected, Some(4));
    assert_eq!(
        activities.key(&key(Key::Named(Named::Backspace), Modifiers::COMMAND)),
        Some(Outcome::SetArchived {
            id: 4,
            archived: true
        })
    );
    assert_eq!(
        activities.key(&key(Key::Named(Named::Enter), Modifiers::empty())),
        Some(Outcome::Focus(editor_id()))
    );
    activities.key(&key(Key::Named(Named::Escape), Modifiers::empty()));
    assert!(activities.editor.is_none());
}

#[test]
fn failed_saves_keep_the_editor() {
    let mut activities = activities();
    activities.update(Message::Rename(2));
    activities.saved(Err("locked".into()));
    assert!(activities.editor.is_some());
    activities.saved(Ok(()));
    assert!(activities.editor.is_none());
}
