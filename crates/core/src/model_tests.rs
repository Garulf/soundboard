use super::*;

fn sound_in(library: &mut Library, tab: TabId, name: &str) -> SoundId {
    let sound = Sound::new(name, format!("{name}.wav"), tab);
    let id = sound.id;
    library.add_sound(sound);
    id
}

#[test]
fn new_library_has_one_tab() {
    let library = Library::new_default();
    assert_eq!(library.tabs.len(), 1);
    assert!(library.sounds.is_empty());
}

#[test]
fn add_sound_appends_to_its_tab_order() {
    let mut library = Library::new_default();
    let tab = library.tabs[0].id;
    let a = sound_in(&mut library, tab, "a");
    let b = sound_in(&mut library, tab, "b");
    assert_eq!(library.tabs[0].order, vec![a, b]);
    let names: Vec<_> = library
        .sounds_in(tab)
        .iter()
        .map(|s| s.name.clone())
        .collect();
    assert_eq!(names, ["a", "b"]);
}

#[test]
fn deleting_last_tab_is_refused() {
    let mut library = Library::new_default();
    let tab = library.tabs[0].id;
    assert_eq!(library.delete_tab(tab), Err(LibraryError::LastTab));
}

#[test]
fn deleting_tab_moves_its_sounds_to_first_remaining_tab() {
    let mut library = Library::new_default();
    let first = library.tabs[0].id;
    let second = library.add_tab("Second");
    let kept = sound_in(&mut library, first, "kept");
    let moved = sound_in(&mut library, second, "moved");

    library.delete_tab(second).unwrap();

    assert_eq!(library.tabs.len(), 1);
    assert_eq!(library.tabs[0].order, vec![kept, moved]);
    assert_eq!(library.sound(moved).unwrap().tab, first);
}

#[test]
fn move_sound_between_tabs_at_index() {
    let mut library = Library::new_default();
    let first = library.tabs[0].id;
    let second = library.add_tab("Second");
    let a = sound_in(&mut library, first, "a");
    let b = sound_in(&mut library, second, "b");
    let c = sound_in(&mut library, second, "c");

    library.move_sound(a, second, 1);

    assert!(library.tabs[0].order.is_empty());
    assert_eq!(library.tabs[1].order, vec![b, a, c]);
    assert_eq!(library.sound(a).unwrap().tab, second);
}

#[test]
fn move_sound_index_past_end_appends() {
    let mut library = Library::new_default();
    let tab = library.tabs[0].id;
    let a = sound_in(&mut library, tab, "a");
    let b = sound_in(&mut library, tab, "b");
    library.move_sound(a, tab, 99);
    assert_eq!(library.tabs[0].order, vec![b, a]);
}

#[test]
fn delete_sound_removes_from_tab_order() {
    let mut library = Library::new_default();
    let tab = library.tabs[0].id;
    let a = sound_in(&mut library, tab, "a");
    library.delete_sound(a);
    assert!(library.sound(a).is_none());
    assert!(library.tabs[0].order.is_empty());
}

#[test]
fn effective_route_falls_back_to_default() {
    let mut library = Library::new_default();
    let tab = library.tabs[0].id;
    let a = sound_in(&mut library, tab, "a");
    library.settings.default_route = Route::Monitor;
    assert_eq!(
        library.effective_route(library.sound(a).unwrap()),
        Route::Monitor
    );
    library.sound_mut(a).unwrap().route = Some(Route::Mic);
    assert_eq!(
        library.effective_route(library.sound(a).unwrap()),
        Route::Mic
    );
}

#[test]
fn move_tab_reorders() {
    let mut library = Library::new_default();
    let first = library.tabs[0].id;
    let second = library.add_tab("Second");
    library.move_tab(second, 0);
    assert_eq!(library.tabs[0].id, second);
    assert_eq!(library.tabs[1].id, first);
}

#[test]
fn new_sounds_get_a_palette_color_stable_per_name() {
    let tab = TabId::new();
    let a = Sound::new("Airhorn", "a.wav", tab);
    let again = Sound::new("Airhorn", "b.wav", tab);
    assert_eq!(a.color, again.color);
    assert!(SOUND_PALETTE.contains(&a.color));
    let colors: std::collections::HashSet<_> = ["a", "b", "c", "d", "e", "f", "g", "h"]
        .iter()
        .map(|n| Sound::new(*n, "x", tab).color)
        .collect();
    assert!(colors.len() > 2);
}
