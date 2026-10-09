use super::*;
use soundboard_core::{Library, Sound};

fn library() -> Library {
    let mut library = Library::new_default();
    let first = library.tabs[0].id;
    let second = library.add_tab("Memes");
    library.add_sound(Sound::new("Airhorn", "a", first));
    library.add_sound(Sound::new("Bruh", "b", second));
    library.add_sound(Sound::new("Applause", "c", second));
    library
}

fn names(sounds: Vec<&Sound>) -> Vec<String> {
    sounds.into_iter().map(|s| s.name.clone()).collect()
}

#[test]
fn all_view_lists_every_tab_in_order() {
    let library = library();
    assert_eq!(
        names(visible_sounds(&library, None, "")),
        ["Airhorn", "Bruh", "Applause"]
    );
}

#[test]
fn tab_view_lists_only_that_tab() {
    let library = library();
    let memes = library.tabs[1].id;
    assert_eq!(
        names(visible_sounds(&library, Some(memes), "")),
        ["Bruh", "Applause"]
    );
}

#[test]
fn search_spans_all_tabs_case_insensitively() {
    let library = library();
    let first = library.tabs[0].id;
    assert_eq!(
        names(visible_sounds(&library, Some(first), "AP")),
        ["Applause"]
    );
    assert_eq!(names(visible_sounds(&library, None, " air ")), ["Airhorn"]);
}
