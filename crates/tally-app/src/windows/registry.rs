//! Which iced window is which. tally has at most one window of each kind.

use std::collections::{HashMap, HashSet};

use iced::window;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowKind {
    Prompt,
    Dashboard,
    Main,
    DataError,
}

#[derive(Debug, Default)]
pub struct Registry {
    kinds: HashMap<window::Id, WindowKind>,
    /// Windows that have had keyboard focus since opening. A new window reports
    /// "unfocused" before it is first shown, which must not count as a click elsewhere.
    focused: HashSet<window::Id>,
}

impl Registry {
    /// Records a newly opened window, returning the id of any window of the same kind it
    /// replaces (the caller closes that one).
    pub fn insert(&mut self, id: window::Id, kind: WindowKind) -> Option<window::Id> {
        let replaced = self.id(kind);
        if let Some(old) = replaced {
            self.kinds.remove(&old);
            self.focused.remove(&old);
        }
        self.kinds.insert(id, kind);
        replaced
    }

    pub fn kind(&self, id: window::Id) -> Option<WindowKind> {
        self.kinds.get(&id).copied()
    }

    pub fn id(&self, kind: WindowKind) -> Option<window::Id> {
        self.kinds
            .iter()
            .find_map(|(id, k)| (*k == kind).then_some(*id))
    }

    pub fn is_open(&self, kind: WindowKind) -> bool {
        self.id(kind).is_some()
    }

    /// Forgets a closed window, returning what it was.
    pub fn remove(&mut self, id: window::Id) -> Option<WindowKind> {
        self.focused.remove(&id);
        self.kinds.remove(&id)
    }

    pub fn focused(&mut self, id: window::Id) {
        if self.kinds.contains_key(&id) {
            self.focused.insert(id);
        }
    }

    /// True when `id` lost focus after having had it: the user clicked elsewhere.
    pub fn lost_focus(&self, id: window::Id) -> bool {
        self.focused.contains(&id)
    }
}

#[cfg(test)]
mod tests;
