//! The four kinds of window and the pieces they share.

pub mod away;
pub mod dashboard;
pub mod data_error;
pub mod main_window;
pub mod placement;
pub mod prompt;
pub mod registry;
pub mod widgets;

use iced::keyboard::{Key, Modifiers, key};

/// A key press routed to the window that has focus. Kept even when a text field
/// already handled the key (Esc, for example, is consumed by the field).
#[derive(Debug, Clone, PartialEq)]
pub struct KeyPress {
    pub key: Key,
    pub modifiers: Modifiers,
    pub physical: key::Physical,
    /// A widget (usually a focused text field) already handled this key.
    pub captured: bool,
}

impl KeyPress {
    /// For tests: a key with no physical code.
    #[cfg(test)]
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            modifiers,
            physical: key::Physical::Unidentified(key::NativeCode::Unidentified),
            captured: false,
        }
    }

    pub fn is(&self, named: key::Named) -> bool {
        self.key == Key::Named(named)
    }
}
