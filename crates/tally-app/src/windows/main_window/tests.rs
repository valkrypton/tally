use iced::keyboard::{Key, Modifiers};
use jiff::civil::date;
use jiff::tz::TimeZone;

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

fn main_window() -> MainWindow {
    MainWindow::new(
        date(2026, 10, 7),
        TimeZone::UTC,
        Settings::default(),
        String::new(),
    )
}

fn cmd(c: &str) -> KeyPress {
    KeyPress::new(Key::Character(c.into()), Modifiers::COMMAND)
}

#[test]
fn command_digits_switch_sections() {
    let mut window = main_window();
    assert_eq!(
        window.key(&cmd("4")),
        Some(Outcome::Opened(Section::Settings))
    );
    assert_eq!(window.section, Section::Settings);
}

#[test]
fn opens_on_the_calendar_and_cmd_1_returns_to_it() {
    let mut window = main_window();
    assert_eq!(window.section, Section::Calendar);
    window.update(Message::Section(Section::Export));
    assert_eq!(
        window.key(&cmd("1")),
        Some(Outcome::Opened(Section::Calendar))
    );
}
