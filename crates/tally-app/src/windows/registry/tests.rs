use super::*;

#[test]
fn tracks_one_window_per_kind() {
    let mut registry = Registry::default();
    let (a, b) = (window::Id::unique(), window::Id::unique());
    assert_eq!(registry.insert(a, WindowKind::Prompt), None);
    assert_eq!(registry.kind(a), Some(WindowKind::Prompt));
    assert_eq!(registry.id(WindowKind::Prompt), Some(a));
    assert!(!registry.is_open(WindowKind::Dashboard));

    assert_eq!(registry.insert(b, WindowKind::Prompt), Some(a));
    assert_eq!(registry.kind(a), None);
    assert_eq!(registry.id(WindowKind::Prompt), Some(b));

    assert_eq!(registry.remove(b), Some(WindowKind::Prompt));
    assert!(!registry.is_open(WindowKind::Prompt));
    assert_eq!(registry.remove(b), None);
}

#[test]
fn only_counts_focus_loss_after_focus() {
    let mut registry = Registry::default();
    let id = window::Id::unique();
    registry.insert(id, WindowKind::Dashboard);
    assert!(
        !registry.lost_focus(id),
        "new windows report unfocused before showing"
    );
    registry.focused(id);
    assert!(registry.lost_focus(id));
    registry.remove(id);
    assert!(!registry.lost_focus(id));
    registry.focused(id);
    assert!(!registry.lost_focus(id), "closed windows are ignored");
}
