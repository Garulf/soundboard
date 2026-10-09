use super::*;

fn parse(s: &str) -> Accelerator {
    s.parse().unwrap()
}

#[test]
fn parses_and_displays_in_canonical_order() {
    let acc = parse("shift+ctrl+f1");
    assert!(acc.ctrl && acc.shift && !acc.alt && !acc.logo);
    assert_eq!(acc.to_string(), "Ctrl+Shift+F1");
}

#[test]
fn display_round_trips() {
    for s in [
        "Ctrl+A",
        "Shift+Alt+7",
        "Super+Space",
        "Ctrl+Alt+Num5",
        "F24",
        "Ctrl+Minus",
    ] {
        assert_eq!(parse(s).to_string(), s);
    }
}

#[test]
fn modifier_aliases_are_accepted() {
    assert_eq!(parse("Control+Option+Meta+x"), parse("Ctrl+Alt+Super+X"));
    assert_eq!(parse("Win+Q"), parse("Super+Q"));
}

#[test]
fn key_aliases_are_accepted() {
    assert_eq!(parse("Ctrl+Return"), parse("Ctrl+Enter"));
    assert_eq!(parse("Ctrl+Esc"), parse("Ctrl+Escape"));
}

#[test]
fn invalid_input_is_rejected() {
    assert!("".parse::<Accelerator>().is_err());
    assert!("Ctrl+".parse::<Accelerator>().is_err());
    assert!("Ctrl+Banana".parse::<Accelerator>().is_err());
    assert!("Ctrl+Shift".parse::<Accelerator>().is_err());
    assert!("A+B".parse::<Accelerator>().is_err());
}

#[test]
fn portal_trigger_uses_xdg_shortcut_names() {
    assert_eq!(parse("Ctrl+Shift+A").to_portal_trigger(), "CTRL+SHIFT+a");
    assert_eq!(parse("Super+F1").to_portal_trigger(), "LOGO+F1");
    assert_eq!(parse("Alt+Enter").to_portal_trigger(), "ALT+Return");
    assert_eq!(parse("Num0").to_portal_trigger(), "KP_0");
}

#[test]
fn converts_to_global_hotkey() {
    let hk = parse("Ctrl+Shift+K").to_global_hotkey();
    assert_eq!(hk.key, global_hotkey::hotkey::Code::KeyK);
    assert!(hk.mods.contains(global_hotkey::hotkey::Modifiers::CONTROL));
    assert!(hk.mods.contains(global_hotkey::hotkey::Modifiers::SHIFT));
}
