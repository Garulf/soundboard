use super::*;
use soundboard_core::{Library, Snapshot, Sound, sound_key};

#[test]
fn state_reflects_tabs_sounds_and_playing() {
    let mut library = Library::new_default();
    let tab = library.tabs[0].id;
    let mut sound = Sound::new("Airhorn", "a", tab);
    sound.color = [255, 0, 16];
    let id = sound.id;
    library.add_sound(sound);
    library.add_sound(Sound::new("Quiet", "b", tab));
    let snapshot = Snapshot {
        library,
        ..Snapshot::default()
    };

    let state = remote_state(&snapshot, &[(sound_key(id), 0.5)]);

    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].sounds.len(), 2);
    assert_eq!(state.sounds[0].name, "Airhorn");
    assert_eq!(state.sounds[0].color, "#ff0010");
    assert!(state.sounds[0].playing);
    assert!(!state.sounds[1].playing);
}

#[test]
fn tokens_are_64_hex_characters_and_unique() {
    let a = generate_token();
    let b = generate_token();
    assert_eq!(a.len(), 64);
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(a, b);
}

#[test]
fn remote_url_includes_host_port_and_token() {
    assert_eq!(
        remote_url("192.168.1.5", 7373, "abc"),
        "http://192.168.1.5:7373/?token=abc"
    );
}
