//! Bridges the tray, tray-menu and global-shortcut callbacks into one iced subscription.
//!
//! The crates call their handlers on the main thread from inside the OS event loop.
//! Each handler only pushes into a channel; the subscription stream forwards to iced,
//! which wakes the event loop, so nothing waits for the 30 s tick.

use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use iced::Subscription;
use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, Stream, StreamExt};
use tray_icon::menu::MenuEvent;
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};

use super::tray::{MENU_ASK_NOW, MENU_OPEN, MENU_PAUSE, MENU_QUIT, TrayRect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    AskNow,
    PauseOrResume,
    Open,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlatformEvent {
    /// Left click (on release) on the tray icon.
    TrayClicked(TrayRect),
    Menu(MenuCommand),
    /// A global shortcut fired; carries the hotkey id.
    Hotkey(u32),
}

pub fn menu_command(id: &str) -> Option<MenuCommand> {
    match id {
        MENU_ASK_NOW => Some(MenuCommand::AskNow),
        MENU_PAUSE => Some(MenuCommand::PauseOrResume),
        MENU_OPEN => Some(MenuCommand::Open),
        MENU_QUIT => Some(MenuCommand::Quit),
        _ => None,
    }
}

pub fn subscription() -> Subscription<PlatformEvent> {
    Subscription::run(stream)
}

fn stream() -> impl Stream<Item = PlatformEvent> {
    iced::stream::channel(32, async |mut output| {
        let (sender, mut receiver) = mpsc::unbounded();

        let tray = sender.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                let _ = tray.unbounded_send(PlatformEvent::TrayClicked(rect.into()));
            }
        }));

        let menu = sender.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some(command) = menu_command(event.id.as_ref()) {
                let _ = menu.unbounded_send(PlatformEvent::Menu(command));
            }
        }));

        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state() == HotKeyState::Pressed {
                let _ = sender.unbounded_send(PlatformEvent::Hotkey(event.id()));
            }
        }));

        while let Some(event) = receiver.next().await {
            if output.send(event).await.is_err() {
                break;
            }
        }
    })
}

#[cfg(test)]
mod tests;
