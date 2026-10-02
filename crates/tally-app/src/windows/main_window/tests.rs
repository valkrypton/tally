use iced::keyboard::{Key, Modifiers};
use jiff::civil::date;

use super::*;

#[test]
fn notice_only_while_failing() {
    assert_eq!(pending_notice(3, None), None);
    assert_eq!(
        pending_notice(3, Some("disk I/O error")).unwrap(),
        "tally couldn't save 3 entries (disk I/O error). It keeps them in memory and retries every 30 seconds."
    );
    assert!(
        pending_notice(1, Some("x"))
            .unwrap()
            .starts_with("tally couldn't save 1 entry ")
    );
    assert!(
        pending_notice(0, Some("x"))
            .unwrap()
            .starts_with("tally couldn't save its state")
    );
}

#[test]
fn command_digits_switch_sections() {
    let mut window = MainWindow::new(date(2026, 10, 7), Settings::default(), String::new());
    let press = KeyPress::new(Key::Character("3".into()), Modifiers::COMMAND);
    assert_eq!(window.key(&press), Some(Outcome::Opened(Section::Settings)));
    assert_eq!(window.section, Section::Settings);
}
