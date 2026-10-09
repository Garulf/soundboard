use super::*;
use crate::clip::tests::{sine, write_wav};
use crate::ids::TabId;
use crate::mixer::{Meters, Mixer, mixer};
use crate::model::Sound;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
struct FakeAudio {
    configs: Arc<Mutex<Vec<AudioConfig>>>,
}

impl AudioControl for FakeAudio {
    fn configure(&mut self, cfg: &AudioConfig) {
        self.configs.lock().unwrap().push(cfg.clone());
    }
}

struct Rig {
    dir: tempfile::TempDir,
    controller: Controller,
    rx: Receiver<Command>,
    mixer: Mixer,
    audio: FakeAudio,
}

impl Rig {
    fn new() -> Self {
        Self::with_store(|_| {})
    }

    fn with_store(prepare: impl FnOnce(&Path)) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cfg");
        std::fs::create_dir_all(&root).unwrap();
        prepare(&root);
        let store = LibraryStore::new(root);
        let outcome = store.load().unwrap();
        let (handle, mixer) = mixer(Arc::new(Meters::default()));
        let audio = FakeAudio::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let controller = Controller::new(
            store,
            outcome,
            handle,
            Box::new(audio.clone()),
            tx,
            Box::new(|| {}),
        );
        Self {
            dir,
            controller,
            rx,
            mixer,
            audio,
        }
    }

    fn tab(&self) -> TabId {
        self.controller.library().tabs[0].id
    }

    fn wav(&self, name: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        write_wav(&path, 48_000, 2, &sine(48_000, 2, 0.5, 0.5));
        path
    }

    fn import(&mut self, path: PathBuf) {
        let tab = self.tab();
        self.controller.handle(Command::Import {
            paths: vec![path],
            tab,
        });
    }

    fn finish_jobs(&mut self, count: usize) {
        for _ in 0..count {
            let cmd = self
                .rx
                .recv_timeout(Duration::from_secs(10))
                .expect("job result");
            self.controller.handle(cmd);
        }
    }

    fn only_sound(&self) -> Sound {
        let library = self.controller.library();
        assert_eq!(library.sounds.len(), 1);
        library.sounds[0].clone()
    }

    fn render(&mut self, frames: usize) -> (Vec<f32>, Vec<f32>) {
        let mut mic = vec![0.0; frames * 2];
        let mut monitor = vec![0.0; frames * 2];
        self.mixer.render(&mut mic, &mut monitor);
        (mic, monitor)
    }
}

fn loud(buf: &[f32]) -> bool {
    buf.iter().any(|s| s.abs() > 0.01)
}

#[test]
fn import_adds_a_loading_sound_then_marks_it_ready() {
    let mut rig = Rig::new();
    let path = rig.wav("Air Horn.wav");
    rig.import(path);

    let sound = rig.only_sound();
    assert_eq!(sound.name, "Air Horn");
    assert!(matches!(
        rig.controller.snapshot().clips.get(&sound.id),
        Some(ClipStatus::Loading)
    ));

    rig.finish_jobs(1);

    let sound = rig.only_sound();
    assert!(!sound.file.is_empty());
    assert!(sound.lufs.is_some());
    assert_eq!(rig.controller.library().tabs[0].order, vec![sound.id]);
    assert!(matches!(
        rig.controller.snapshot().clips.get(&sound.id),
        Some(ClipStatus::Ready(_))
    ));
}

#[test]
fn failed_import_removes_the_sound_and_adds_a_notice() {
    let mut rig = Rig::new();
    let path = rig.dir.path().join("notes.wav");
    std::fs::write(&path, "not audio").unwrap();
    rig.import(path);
    rig.finish_jobs(1);

    assert!(rig.controller.library().sounds.is_empty());
    assert!(rig.controller.library().tabs[0].order.is_empty());
    let notices = &rig.controller.snapshot().notices;
    assert_eq!(notices.len(), 1);
    assert!(notices[0].contains("notes.wav"), "{notices:?}");
}

#[test]
fn importing_a_folder_imports_its_audio_files() {
    let mut rig = Rig::new();
    let folder = rig.dir.path().join("pack");
    std::fs::create_dir(&folder).unwrap();
    write_wav(&folder.join("a.wav"), 48_000, 1, &sine(48_000, 1, 0.1, 0.5));
    write_wav(&folder.join("b.wav"), 48_000, 1, &sine(48_000, 1, 0.1, 0.5));
    std::fs::write(folder.join("readme.txt"), "hi").unwrap();
    rig.import(folder);
    rig.finish_jobs(2);
    assert_eq!(rig.controller.library().sounds.len(), 2);
}

#[test]
fn playing_a_ready_sound_reaches_the_mixer() {
    let mut rig = Rig::new();
    let path = rig.wav("a.wav");
    rig.import(path);
    rig.finish_jobs(1);
    let id = rig.only_sound().id;

    rig.controller.handle(Command::Play(id));
    let (mic, monitor) = rig.render(4800);

    assert!(loud(&mic));
    assert!(loud(&monitor));
}

#[test]
fn playing_a_loading_sound_is_ignored() {
    let mut rig = Rig::new();
    let path = rig.wav("a.wav");
    rig.import(path);
    let id = rig.only_sound().id;

    rig.controller.handle(Command::Play(id));
    let (mic, monitor) = rig.render(4800);

    assert!(!loud(&mic) && !loud(&monitor));
}

#[test]
fn preview_plays_on_monitor_only() {
    let mut rig = Rig::new();
    let path = rig.wav("a.wav");
    rig.import(path);
    rig.finish_jobs(1);
    let id = rig.only_sound().id;

    rig.controller.handle(Command::Preview {
        sound: id,
        trim_start_ms: 0,
        trim_end_ms: None,
    });
    let (mic, monitor) = rig.render(4800);

    assert!(!loud(&mic));
    assert!(loud(&monitor));
}

#[test]
fn deleting_a_playing_sound_is_safe_and_silences_it() {
    let mut rig = Rig::new();
    let path = rig.wav("a.wav");
    rig.import(path);
    rig.finish_jobs(1);
    let id = rig.only_sound().id;
    rig.controller.handle(Command::Play(id));
    rig.render(256);

    rig.controller.handle(Command::DeleteSound(id));
    rig.render(4800);
    let (mic, monitor) = rig.render(256);

    assert!(rig.controller.library().sounds.is_empty());
    assert!(!loud(&mic) && !loud(&monitor));
    assert!(!rig.controller.snapshot().clips.contains_key(&id));
}

#[test]
fn deleting_the_last_tab_adds_a_notice() {
    let mut rig = Rig::new();
    let tab = rig.tab();
    rig.controller.handle(Command::DeleteTab(tab));
    assert_eq!(rig.controller.library().tabs.len(), 1);
    assert_eq!(rig.controller.snapshot().notices.len(), 1);
}

#[test]
fn settings_change_reconfigures_audio_without_touching_hotkeys() {
    let mut rig = Rig::new();
    let before = rig.controller.snapshot().hotkey_revision;
    let mut settings = rig.controller.library().settings.clone();
    settings.monitor_device = Some("headphones".into());

    rig.controller
        .handle(Command::UpdateSettings(Box::new(settings)));

    let configs = rig.audio.configs.lock().unwrap();
    assert_eq!(
        configs.last().unwrap().monitor_device.as_deref(),
        Some("headphones")
    );
    assert_eq!(rig.controller.snapshot().hotkey_revision, before);
}

#[test]
fn hotkey_changes_bump_the_hotkey_revision() {
    let mut rig = Rig::new();
    let before = rig.controller.snapshot().hotkey_revision;
    let mut settings = rig.controller.library().settings.clone();
    settings.stop_all_hotkey = Some("Ctrl+F12".into());
    rig.controller
        .handle(Command::UpdateSettings(Box::new(settings)));
    let after_settings = rig.controller.snapshot().hotkey_revision;
    assert!(after_settings > before);

    let path = rig.wav("a.wav");
    rig.import(path);
    rig.finish_jobs(1);
    let mut sound = rig.only_sound();
    sound.hotkey = Some("Ctrl+1".into());
    rig.controller.handle(Command::UpdateSound(Box::new(sound)));
    assert!(rig.controller.snapshot().hotkey_revision > after_settings);
}

#[test]
fn toggle_passthrough_flips_the_setting_and_reconfigures() {
    let mut rig = Rig::new();
    let before = rig.controller.library().settings.passthrough;
    rig.controller.handle(Command::TogglePassthrough);
    assert_eq!(rig.controller.library().settings.passthrough, !before);
    let configs = rig.audio.configs.lock().unwrap();
    assert_eq!(configs.last().unwrap().passthrough, !before);
}

#[test]
fn mic_ready_attaches_capture_to_the_mic_bus() {
    let mut rig = Rig::new();
    let (mut producer, mic) = crate::mic::mic_ring(48_000);
    for _ in 0..4800 * 2 {
        producer.push(0.25).unwrap();
    }
    rig.controller.handle(Command::MicReady(Box::new(mic)));
    let (mic_bus, monitor) = rig.render(512);
    assert!(loud(&mic_bus));
    assert!(!loud(&monitor));
}

#[test]
fn saves_are_debounced() {
    let mut rig = Rig::new();
    let file = rig.controller.store().root().join("library.toml");
    rig.controller.handle(Command::AddTab("One".into()));
    rig.controller.handle(Command::AddTab("Two".into()));

    rig.controller.tick(Instant::now());
    assert!(!file.exists());

    rig.controller
        .tick(Instant::now() + SAVE_DEBOUNCE + Duration::from_millis(50));
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("One") && text.contains("Two"));
}

#[test]
fn recovered_library_is_reported_as_a_notice() {
    let rig = Rig::with_store(|root| {
        std::fs::write(root.join("library.toml"), "= broken =").unwrap();
    });
    let notices = &rig.controller.snapshot().notices;
    assert_eq!(notices.len(), 1);
    assert!(notices[0].contains("library.toml.broken-"), "{notices:?}");
}

#[test]
fn dismissing_a_notice_removes_it() {
    let mut rig = Rig::new();
    let tab = rig.tab();
    rig.controller.handle(Command::DeleteTab(tab));
    rig.controller.handle(Command::DismissNotice(0));
    assert!(rig.controller.snapshot().notices.is_empty());
}

#[test]
fn shutdown_stops_the_loop() {
    let mut rig = Rig::new();
    assert!(rig.controller.handle(Command::StopAll));
    assert!(!rig.controller.handle(Command::Shutdown));
}

#[test]
fn play_gain_applies_volume_and_normalization() {
    let mut sound = Sound::new("a", "a.wav", TabId::new());
    sound.volume = 0.5;
    sound.lufs = Some(-24.0);
    sound.normalize = true;
    assert!((play_gain(&sound, -18.0) - 0.5 * 1.995).abs() < 0.01);
    sound.normalize = false;
    assert_eq!(play_gain(&sound, -18.0), 0.5);
    sound.normalize = true;
    sound.lufs = None;
    assert_eq!(play_gain(&sound, -18.0), 0.5);
}

#[test]
fn trim_window_converts_milliseconds_to_frames() {
    assert_eq!(trim_frames(500, Some(1000), 96_000), (24_000, 48_000));
    assert_eq!(trim_frames(0, None, 96_000), (0, 96_000));
    assert_eq!(trim_frames(0, Some(10_000), 96_000), (0, 96_000));
}
