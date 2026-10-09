use super::*;
use crate::model::{Library, Sound};
use std::fs;

fn store() -> (tempfile::TempDir, LibraryStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = LibraryStore::new(dir.path().join("cfg"));
    (dir, store)
}

#[test]
fn missing_library_loads_default() {
    let (_dir, store) = store();
    let outcome = store.load().unwrap();
    assert_eq!(outcome.library.tabs.len(), 1);
    assert!(outcome.recovered_from.is_none());
}

#[test]
fn save_then_load_round_trips() {
    let (_dir, store) = store();
    let mut library = Library::new_default();
    let tab = library.tabs[0].id;
    library.add_sound(Sound::new("Airhorn", "abc.wav", tab));
    library.settings.mic_volume = 0.5;

    store.save(&library).unwrap();
    let loaded = store.load().unwrap().library;

    assert_eq!(loaded, library);
}

#[test]
fn save_leaves_no_temp_files() {
    let (_dir, store) = store();
    store.save(&Library::new_default()).unwrap();
    let names: Vec<_> = fs::read_dir(store.root())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, ["library.toml"]);
}

#[test]
fn corrupt_library_is_set_aside_and_default_loaded() {
    let (_dir, store) = store();
    fs::create_dir_all(store.root()).unwrap();
    fs::write(store.root().join("library.toml"), "this is = = not toml").unwrap();

    let outcome = store.load().unwrap();

    let broken = outcome.recovered_from.expect("should report recovery");
    assert!(
        broken
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("library.toml.broken-")
    );
    assert!(broken.exists());
    assert!(!store.root().join("library.toml").exists());
    assert_eq!(outcome.library.tabs.len(), 1);
}

#[test]
fn import_copies_file_named_by_content_hash() {
    let (dir, store) = store();
    let src = dir.path().join("Air Horn.WAV");
    fs::write(&src, b"RIFF fake audio").unwrap();

    let name = store.import_file(&src).unwrap();

    assert!(name.ends_with(".wav"), "{name}");
    assert_eq!(
        fs::read(store.sound_path(&name)).unwrap(),
        b"RIFF fake audio"
    );
}

#[test]
fn importing_identical_content_twice_reuses_the_file() {
    let (dir, store) = store();
    let a = dir.path().join("a.mp3");
    let b = dir.path().join("b.mp3");
    fs::write(&a, b"same").unwrap();
    fs::write(&b, b"same").unwrap();

    let first = store.import_file(&a).unwrap();
    let second = store.import_file(&b).unwrap();

    assert_eq!(first, second);
    assert_eq!(fs::read_dir(store.sounds_dir()).unwrap().count(), 1);
}

#[test]
fn importing_missing_file_errors() {
    let (dir, store) = store();
    let err = store.import_file(&dir.path().join("nope.wav")).unwrap_err();
    assert!(matches!(err, StoreError::Io { .. }), "{err:?}");
}

#[test]
fn concurrent_imports_of_identical_content_all_succeed() {
    let (dir, store) = store();
    for round in 0..10 {
        let sources: Vec<_> = (0..8)
            .map(|i| {
                let path = dir.path().join(format!("r{round}-{i}.wav"));
                let mut bytes = vec![round as u8; 4 << 20];
                bytes[0] = 1;
                fs::write(&path, bytes).unwrap();
                path
            })
            .collect();
        let results: Vec<_> = std::thread::scope(|scope| {
            let handles: Vec<_> = sources
                .iter()
                .map(|src| scope.spawn(|| store.import_file(src)))
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let names: Vec<_> = results.into_iter().map(|r| r.unwrap()).collect();
        assert!(names.windows(2).all(|w| w[0] == w[1]));
    }
}
