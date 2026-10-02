use super::*;

fn code(code: Code) -> Physical {
    Physical::Code(code)
}

#[test]
fn records_a_shortcut_in_global_hotkey_syntax() {
    let spec = spec_from_key(Modifiers::ALT | Modifiers::SHIFT, &code(Code::KeyT));
    assert_eq!(spec.as_deref(), Some("Alt+Shift+KeyT"));
    let spec = spec_from_key(Modifiers::CTRL | Modifiers::LOGO, &code(Code::Digit1));
    assert_eq!(spec.as_deref(), Some("Ctrl+Super+Digit1"));
    assert_eq!(
        spec_from_key(Modifiers::LOGO, &code(Code::F5)).as_deref(),
        Some("Super+F5")
    );
}

#[test]
fn the_default_setting_parses() {
    let default = tally_core::settings::Settings::default().shortcut;
    assert!(HotKey::from_str(&default).is_ok());
}

#[test]
fn rejects_shortcuts_without_a_real_modifier() {
    assert_eq!(spec_from_key(Modifiers::empty(), &code(Code::KeyT)), None);
    assert_eq!(spec_from_key(Modifiers::SHIFT, &code(Code::KeyT)), None);
}

#[test]
fn ignores_lone_modifier_presses() {
    assert_eq!(spec_from_key(Modifiers::ALT, &code(Code::AltLeft)), None);
    assert_eq!(
        spec_from_key(Modifiers::LOGO, &code(Code::SuperRight)),
        None
    );
}

#[test]
fn caps_use_symbols_on_macos_and_words_elsewhere() {
    assert_eq!(caps("Alt+Shift+KeyT", true), ["⌥", "⇧", "T"]);
    assert_eq!(caps("Alt+Shift+KeyT", false), ["Alt", "Shift", "T"]);
    assert_eq!(caps("Ctrl+Super+Digit1", true), ["⌃", "⌘", "1"]);
    assert_eq!(caps("Ctrl+Super+F5", false), ["Ctrl", "Win", "F5"]);
}
