//! The global "ask now" shortcut, via `global-hotkey`. Shortcuts are stored in
//! `global-hotkey` syntax, e.g. `Alt+Shift+KeyT`.

use std::str::FromStr;

use anyhow::Context;
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;
use iced::keyboard::Modifiers;
use iced::keyboard::key::{Code, Physical};

/// Shown in Settings when a shortcut can't be registered (spec §7).
pub const IN_USE: &str = "That shortcut is in use. Pick another.";

pub struct Hotkey {
    manager: GlobalHotKeyManager,
    current: Option<HotKey>,
}

impl Hotkey {
    /// Must be created on the main thread, after the event loop has started.
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            manager: GlobalHotKeyManager::new().context("global shortcuts unavailable")?,
            current: None,
        })
    }

    /// Registers `spec` in place of the current shortcut. On failure the previous
    /// shortcut stays registered and the error says why.
    pub fn set(&mut self, spec: &str) -> Result<(), String> {
        let hotkey = HotKey::from_str(spec).map_err(|_| String::from(IN_USE))?;
        if self.current == Some(hotkey) {
            return Ok(());
        }
        if let Err(error) = self.manager.register(hotkey) {
            tracing::warn!(%error, spec, "could not register shortcut");
            return Err(String::from(IN_USE));
        }
        if let Some(old) = self.current.replace(hotkey) {
            let _ = self.manager.unregister(old);
        }
        Ok(())
    }

    /// The id that `GlobalHotKeyEvent`s carry for the current shortcut.
    pub fn id(&self) -> Option<u32> {
        self.current.map(|hotkey| hotkey.id())
    }
}

/// Builds a shortcut from a key press in the Settings recorder. Needs Ctrl, Alt or
/// Super (Shift alone would steal ordinary typing) and a non-modifier key that
/// `global-hotkey` understands.
pub fn spec_from_key(modifiers: Modifiers, physical: &Physical) -> Option<String> {
    let Physical::Code(code) = physical else {
        return None;
    };
    if is_modifier(*code) || !(modifiers.control() || modifiers.alt() || modifiers.logo()) {
        return None;
    }
    let mut parts = Vec::new();
    if modifiers.control() {
        parts.push(String::from("Ctrl"));
    }
    if modifiers.alt() {
        parts.push(String::from("Alt"));
    }
    if modifiers.shift() {
        parts.push(String::from("Shift"));
    }
    if modifiers.logo() {
        parts.push(String::from("Super"));
    }
    parts.push(format!("{code:?}"));
    let spec = parts.join("+");
    HotKey::from_str(&spec).is_ok().then_some(spec)
}

fn is_modifier(code: Code) -> bool {
    matches!(
        code,
        Code::ShiftLeft
            | Code::ShiftRight
            | Code::ControlLeft
            | Code::ControlRight
            | Code::AltLeft
            | Code::AltRight
            | Code::SuperLeft
            | Code::SuperRight
            | Code::CapsLock
            | Code::Fn
            | Code::FnLock
    )
}

/// The key caps to draw for a shortcut: `⌥ ⇧ T` on macOS, `Alt Shift T` elsewhere.
pub fn caps(spec: &str, mac: bool) -> Vec<String> {
    spec.split('+')
        .map(|part| {
            let symbol = match (part.to_ascii_lowercase().as_str(), mac) {
                ("alt" | "option", true) => "⌥",
                ("alt" | "option", false) => "Alt",
                ("shift", true) => "⇧",
                ("shift", false) => "Shift",
                ("ctrl" | "control", true) => "⌃",
                ("ctrl" | "control", false) => "Ctrl",
                ("super" | "cmd" | "command", true) => "⌘",
                ("super" | "cmd" | "command", false) => "Win",
                _ => {
                    let key = part
                        .strip_prefix("Key")
                        .or_else(|| part.strip_prefix("Digit"))
                        .unwrap_or(part);
                    return key.to_owned();
                }
            };
            symbol.to_owned()
        })
        .collect()
}

#[cfg(test)]
mod tests;
