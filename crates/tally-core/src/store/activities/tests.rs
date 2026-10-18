use super::super::test_support::*;
use super::*;

fn id_of(store: &Store, name: &str) -> ActivityId {
    find(&store.conn, &act(name)).unwrap().unwrap()
}

#[test]
fn ensure_reuses_existing_activities() {
    let mut store = Store::open_in_memory().unwrap();
    let first = store.ensure_activity(&act("Work: Meeting")).unwrap();
    let again = store.ensure_activity(&act("Work: Meeting")).unwrap();
    let other = store.ensure_activity(&act("Meeting")).unwrap();
    assert_eq!(first, again);
    assert_ne!(
        first, other,
        "same name without a category is a different activity"
    );
}

#[test]
fn names_match_case_insensitively_and_keep_original_spelling() {
    let mut store = Store::open_in_memory().unwrap();
    let first = store.ensure_activity(&act("Work: Meeting")).unwrap();
    assert_eq!(store.ensure_activity(&act("work: MEETING")).unwrap(), first);
    assert_eq!(store.suggestions(10).unwrap(), vec![act("Work: Meeting")]);
}

#[test]
fn suggestions_are_most_recent_first_and_skip_archived() {
    let mut store = Store::open_in_memory().unwrap();
    record(&mut store, "Work: Code", at(0, 9, 0), at(0, 9, 15));
    record(&mut store, "Lunch", at(0, 12, 0), at(0, 12, 30));
    record(&mut store, "Work: Meeting", at(0, 10, 0), at(0, 10, 15));
    assert_eq!(
        store.suggestions(10).unwrap(),
        vec![act("Lunch"), act("Work: Meeting"), act("Work: Code")]
    );
    assert_eq!(store.suggestions(1).unwrap(), vec![act("Lunch")]);

    let lunch = id_of(&store, "Lunch");
    store.set_archived(lunch, true, at(0, 13, 0)).unwrap();
    assert_eq!(
        store.suggestions(10).unwrap(),
        vec![act("Work: Meeting"), act("Work: Code")]
    );
}

#[test]
fn typing_an_archived_name_unarchives_it() {
    let mut store = Store::open_in_memory().unwrap();
    let id = store.ensure_activity(&act("Gym")).unwrap();
    store.set_archived(id, true, at(0, 9, 0)).unwrap();
    record(&mut store, "Gym", at(0, 9, 0), at(0, 10, 0));
    assert!(!store.activities().unwrap()[0].archived);
}

#[test]
fn lists_activities_with_totals() {
    let mut store = Store::open_in_memory().unwrap();
    record(&mut store, "Work: Code", at(0, 9, 0), at(0, 10, 0));
    record(&mut store, "Lunch", at(0, 12, 0), at(0, 12, 30));
    let rows = store.activities().unwrap();
    let summary: Vec<_> = rows
        .iter()
        .map(|r| (r.name.to_string(), r.total_seconds))
        .collect();
    assert_eq!(
        summary,
        vec![("Lunch".into(), 1800), ("Work: Code".into(), 3600)]
    );
}

#[test]
fn rename_moves_between_categories() {
    let mut store = Store::open_in_memory().unwrap();
    record(&mut store, "Mtg", at(0, 9, 0), at(0, 9, 15));
    let id = id_of(&store, "Mtg");
    assert_eq!(
        store.rename(id, &act("Work: Meeting")).unwrap(),
        Rename::Done
    );
    assert_eq!(store.suggestions(10).unwrap(), vec![act("Work: Meeting")]);
}

#[test]
fn rename_into_existing_name_reports_conflict() {
    let mut store = Store::open_in_memory().unwrap();
    let meeting = store.ensure_activity(&act("Work: Meeting")).unwrap();
    let typo = store.ensure_activity(&act("Work: Meetings")).unwrap();
    assert_eq!(
        store.rename(typo, &act("Work: Meeting")).unwrap(),
        Rename::Conflict(meeting)
    );
    assert_eq!(
        store.rename(meeting, &act("Work: Meeting")).unwrap(),
        Rename::Done
    );
}

#[test]
fn merge_moves_time_and_removes_the_source() {
    let mut store = Store::open_in_memory().unwrap();
    record(&mut store, "Work: Meeting", at(0, 9, 0), at(0, 10, 0));
    record(&mut store, "Mtg", at(0, 11, 0), at(0, 11, 15));
    let (from, into) = (id_of(&store, "Mtg"), id_of(&store, "Work: Meeting"));
    store.merge(from, into).unwrap();
    let rows = store.activities().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].total_seconds, 4500);
}

#[test]
fn unknown_ids_are_errors() {
    let mut store = Store::open_in_memory().unwrap();
    let id = store.ensure_activity(&act("A")).unwrap();
    assert!(matches!(
        store.merge(999, id),
        Err(Error::UnknownActivity(999))
    ));
    assert!(matches!(
        store.rename(999, &act("B")),
        Err(Error::UnknownActivity(999))
    ));
    assert!(matches!(
        store.set_archived(999, true, at(0, 9, 0)),
        Err(Error::UnknownActivity(999))
    ));
}

#[test]
fn empty_categories_are_cleaned_up() {
    let mut store = Store::open_in_memory().unwrap();
    let id = store.ensure_activity(&act("Old: Thing")).unwrap();
    store.rename(id, &act("Thing")).unwrap();
    let count: i64 = store
        .conn
        .query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn renaming_a_category_moves_its_activities_and_merges_clashes() {
    let mut store = Store::open_in_memory().unwrap();
    record(&mut store, "Work: Meeting", at(0, 9, 0), at(0, 10, 0));
    record(&mut store, "Work: Code", at(0, 10, 0), at(0, 10, 30));
    record(&mut store, "Job: Meeting", at(0, 11, 0), at(0, 11, 15));
    store.rename_category("work", "Job").unwrap();
    let rows = store.activities().unwrap();
    let names: Vec<String> = rows.iter().map(|r| r.name.to_string()).collect();
    assert_eq!(rows.len(), 2, "{names:?}");
    assert_eq!(
        rows.iter()
            .find(|r| r.name == act("Job: Meeting"))
            .unwrap()
            .total_seconds,
        4500
    );
    assert!(rows.iter().any(|r| r.name == act("Job: Code")));
    store.rename_category("Job", "JOB").unwrap();
    assert!(
        store
            .activities()
            .unwrap()
            .iter()
            .any(|r| r.name == act("JOB: Code"))
    );
}
