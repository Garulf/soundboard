use super::*;
use soundboard_core::{Library, Sound};

#[test]
fn bindings_include_sounds_and_global_actions() {
    let mut library = Library::new_default();
    let tab = library.tabs[0].id;
    let mut sound = Sound::new("Airhorn", "a.wav", tab);
    sound.hotkey = Some("Ctrl+1".into());
    let id = sound.id;
    library.add_sound(sound);
    library.add_sound(Sound::new("No key", "b.wav", tab));
    library.settings.stop_all_hotkey = Some("Ctrl+0".into());

    let bindings = bindings_from(&library);

    let ids: Vec<_> = bindings.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(ids, [format!("sound:{id}").as_str(), STOP_ALL]);
    assert_eq!(bindings[0].description, "Play Airhorn");
}

#[test]
fn unparseable_hotkeys_are_skipped() {
    let mut library = Library::new_default();
    library.settings.passthrough_hotkey = Some("Ctrl+Banana".into());
    assert!(bindings_from(&library).is_empty());
}

#[test]
fn binding_ids_map_to_commands() {
    let id = soundboard_core::SoundId::new();
    assert!(matches!(command_for(&format!("sound:{id}")), Some(Command::Play(x)) if x == id));
    assert!(matches!(command_for(STOP_ALL), Some(Command::StopAll)));
    assert!(matches!(
        command_for(PASSTHROUGH),
        Some(Command::TogglePassthrough)
    ));
    assert!(command_for("sound:nope").is_none());
    assert!(command_for("other").is_none());
}
