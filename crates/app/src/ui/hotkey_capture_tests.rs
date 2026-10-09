use super::*;
use egui::{Key, Modifiers};

#[test]
fn letter_with_modifiers_becomes_an_accelerator() {
    let mods = Modifiers {
        ctrl: true,
        shift: true,
        ..Modifiers::NONE
    };
    let acc = egui_key_to_accelerator(Key::K, mods).unwrap();
    assert_eq!(acc.to_string(), "Ctrl+Shift+K");
}

#[test]
fn punctuation_and_function_keys_map() {
    assert_eq!(
        egui_key_to_accelerator(Key::OpenBracket, Modifiers::ALT)
            .unwrap()
            .to_string(),
        "Alt+BracketLeft"
    );
    assert_eq!(
        egui_key_to_accelerator(Key::F13, Modifiers::NONE)
            .unwrap()
            .to_string(),
        "F13"
    );
}

#[test]
fn keys_without_a_global_equivalent_are_rejected() {
    assert!(egui_key_to_accelerator(Key::Copy, Modifiers::NONE).is_none());
    assert!(egui_key_to_accelerator(Key::F30, Modifiers::NONE).is_none());
}
