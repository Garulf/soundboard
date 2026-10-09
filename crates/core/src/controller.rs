use crate::audio::{AudioConfig, AudioControl, BackendStatus, DeviceList};
use crate::clip::{ENGINE_RATE, decode_file};
use crate::command::{Command, LoadedClip};
use crate::ids::{SoundId, TabId};
use crate::jobs::JobPool;
use crate::loudness::{db_to_gain, measure_lufs, normalization_gain_db};
use crate::mixer::{Bus, Meters, MixerHandle, MixerMsg, PlayRequest};
use crate::model::{Library, PlayMode, Route, Settings, Sound};
use crate::snapshot::{ClipStatus, SharedSnapshot, Snapshot};
use crate::store::{LibraryStore, LoadOutcome};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread;
use std::time::{Duration, Instant};

pub const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);
const TICK: Duration = Duration::from_millis(100);
const AUDIO_EXTENSIONS: &[&str] = &[
    "wav", "wave", "mp3", "flac", "ogg", "oga", "m4a", "aac", "mp4", "caf", "aif", "aiff", "mkv",
    "webm", "opus",
];

pub type ChangeCallback = Box<dyn Fn() + Send>;

pub fn play_gain(sound: &Sound, target_lufs: f64) -> f32 {
    let normalization = match (sound.normalize, sound.lufs) {
        (true, Some(lufs)) => db_to_gain(normalization_gain_db(lufs, target_lufs)),
        _ => 1.0,
    };
    sound.volume * normalization
}

fn ms_to_frames(ms: u64) -> usize {
    (ms * ENGINE_RATE as u64 / 1000) as usize
}

/// Converts a trim window in milliseconds into a frame range clamped to the clip.
pub fn trim_frames(start_ms: u64, end_ms: Option<u64>, clip_frames: usize) -> (usize, usize) {
    let end = end_ms.map_or(clip_frames, ms_to_frames).min(clip_frames);
    (ms_to_frames(start_ms), end)
}

fn load_clip(path: &Path) -> Result<LoadedClip, String> {
    let clip = decode_file(path).map_err(|e| e.to_string())?;
    let lufs = measure_lufs(&clip);
    Ok(LoadedClip {
        clip: Arc::new(clip),
        lufs,
    })
}

fn expand_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(&path)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .collect();
            entries.sort();
            files.extend(entries.into_iter().filter(|p| {
                p.is_dir()
                    || p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                        AUDIO_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str())
                    })
            }));
        } else {
            files.push(path);
        }
    }
    if files.iter().any(|p| p.is_dir()) {
        expand_paths(files)
    } else {
        files
    }
}

fn display_name(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Sound".into())
}

fn hotkeys_of(settings: &Settings) -> (&Option<String>, &Option<String>) {
    (&settings.stop_all_hotkey, &settings.passthrough_hotkey)
}

pub struct Controller {
    library: Library,
    store: LibraryStore,
    clips: HashMap<SoundId, ClipStatus>,
    pending_imports: HashSet<SoundId>,
    mixer: MixerHandle,
    audio: Box<dyn AudioControl>,
    jobs: JobPool,
    tx: Sender<Command>,
    notices: Vec<String>,
    backend: BackendStatus,
    devices: DeviceList,
    revision: u64,
    hotkey_revision: u64,
    last_change: Option<Instant>,
    snapshot: SharedSnapshot,
    on_change: ChangeCallback,
}

impl Controller {
    pub fn new(
        store: LibraryStore,
        outcome: LoadOutcome,
        mixer: MixerHandle,
        audio: Box<dyn AudioControl>,
        tx: Sender<Command>,
        on_change: ChangeCallback,
    ) -> Self {
        let mut controller = Self {
            library: outcome.library,
            store,
            clips: HashMap::new(),
            pending_imports: HashSet::new(),
            mixer,
            audio,
            jobs: JobPool::new(),
            tx,
            notices: Vec::new(),
            backend: BackendStatus::Starting,
            devices: DeviceList::default(),
            revision: 0,
            hotkey_revision: 1,
            last_change: None,
            snapshot: SharedSnapshot::default(),
            on_change,
        };
        if let Some(path) = outcome.recovered_from {
            controller.notices.push(format!(
                "Your library file could not be read and was moved to {}. Starting with an empty library.",
                path.display()
            ));
        }
        controller.apply_mix_settings();
        controller
            .audio
            .configure(&AudioConfig::from_settings(&controller.library.settings));
        let sounds: Vec<(SoundId, String)> = controller
            .library
            .sounds
            .iter()
            .map(|s| (s.id, s.file.clone()))
            .collect();
        for (id, file) in sounds {
            controller.load_existing(id, &file);
        }
        controller.publish();
        controller
    }

    pub fn library(&self) -> &Library {
        &self.library
    }

    pub fn store(&self) -> &LibraryStore {
        &self.store
    }

    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.snapshot.get()
    }

    pub fn shared_snapshot(&self) -> SharedSnapshot {
        self.snapshot.clone()
    }

    fn send_mixer(&mut self, msg: MixerMsg) {
        if !self.mixer.send(msg) {
            tracing::warn!("mixer message queue is full; dropping message");
        }
    }

    fn apply_mix_settings(&mut self) {
        let s = &self.library.settings;
        let msgs = [
            MixerMsg::BusGain(Bus::Mic, s.mic_bus_volume),
            MixerMsg::BusGain(Bus::Monitor, s.monitor_bus_volume),
            MixerMsg::MicGain(s.mic_volume),
        ];
        for msg in msgs {
            self.send_mixer(msg);
        }
    }

    fn load_existing(&mut self, id: SoundId, file: &str) {
        self.clips.insert(id, ClipStatus::Loading);
        let path = self.store.sound_path(file);
        let tx = self.tx.clone();
        self.jobs.submit(move || {
            let result = load_clip(&path);
            let _ = tx.send(Command::ClipLoaded {
                sound: id,
                file: None,
                result,
            });
        });
    }

    fn import(&mut self, paths: Vec<PathBuf>, tab: TabId) {
        for path in expand_paths(paths) {
            let sound = Sound::new(display_name(&path), String::new(), tab);
            let id = sound.id;
            self.library.add_sound(sound);
            self.clips.insert(id, ClipStatus::Loading);
            self.pending_imports.insert(id);
            let store = self.store.clone();
            let tx = self.tx.clone();
            self.jobs.submit(move || {
                let label = path.display().to_string();
                let (file, result) = match store.import_file(&path) {
                    Ok(file) => {
                        let result = load_clip(&store.sound_path(&file));
                        (Some(file), result)
                    }
                    Err(e) => (None, Err(e.to_string())),
                };
                let result = result.map_err(|e| format!("Could not import {label}: {e}"));
                let _ = tx.send(Command::ClipLoaded {
                    sound: id,
                    file,
                    result,
                });
            });
        }
        self.mark_dirty();
    }

    fn clip_loaded(
        &mut self,
        id: SoundId,
        file: Option<String>,
        result: Result<LoadedClip, String>,
    ) {
        let is_import = self.pending_imports.remove(&id);
        if self.library.sound(id).is_none() {
            self.clips.remove(&id);
            return;
        }
        match result {
            Ok(loaded) => {
                if let Some(sound) = self.library.sound_mut(id) {
                    if let Some(file) = file {
                        sound.file = file;
                    }
                    if sound.lufs.is_none() {
                        sound.lufs = loaded.lufs;
                    }
                }
                self.clips.insert(id, ClipStatus::Ready(loaded.clip));
                self.mark_dirty();
            }
            Err(message) if is_import => {
                self.library.delete_sound(id);
                self.clips.remove(&id);
                self.notices.push(message);
                self.mark_dirty();
            }
            Err(message) => {
                self.clips.insert(id, ClipStatus::Failed(message));
            }
        }
    }

    fn play(&mut self, id: SoundId, preview: Option<(u64, Option<u64>)>) {
        let Some(sound) = self.library.sound(id) else {
            return;
        };
        let Some(ClipStatus::Ready(clip)) = self.clips.get(&id) else {
            return;
        };
        let (trim_start, trim_end) = preview.unwrap_or((sound.trim_start_ms, sound.trim_end_ms));
        let (start_frame, end_frame) = trim_frames(trim_start, trim_end, clip.frames());
        let gain = play_gain(sound, self.library.settings.normalize_target_lufs);
        let (route, mode, looping) = match preview {
            Some(_) => (Route::Monitor, PlayMode::Restart, false),
            None => (
                self.library.effective_route(sound),
                sound.mode,
                sound.looping,
            ),
        };
        let request = PlayRequest {
            sound: id,
            clip: clip.clone(),
            start_frame,
            end_frame,
            gain,
            route,
            mode,
            looping,
        };
        self.send_mixer(MixerMsg::Play(request));
    }

    fn update_sound(&mut self, updated: Sound) {
        let Some(sound) = self.library.sound_mut(updated.id) else {
            return;
        };
        let hotkey_changed = sound.hotkey != updated.hotkey;
        let tab_changed = sound.tab != updated.tab;
        let target_tab = updated.tab;
        *sound = Sound {
            tab: sound.tab,
            ..updated
        };
        let id = sound.id;
        if tab_changed {
            self.library.move_sound(id, target_tab, usize::MAX);
        }
        if hotkey_changed {
            self.hotkey_revision += 1;
        }
        self.mark_dirty();
    }

    fn delete_sound(&mut self, id: SoundId) {
        if let Some(sound) = self.library.sound(id)
            && sound.hotkey.is_some()
        {
            self.hotkey_revision += 1;
        }
        self.send_mixer(MixerMsg::Stop(id));
        self.library.delete_sound(id);
        self.clips.remove(&id);
        self.mark_dirty();
    }

    fn update_settings(&mut self, settings: Settings) {
        let old = std::mem::replace(&mut self.library.settings, settings);
        if hotkeys_of(&old) != hotkeys_of(&self.library.settings) {
            self.hotkey_revision += 1;
        }
        let new_audio = AudioConfig::from_settings(&self.library.settings);
        if AudioConfig::from_settings(&old) != new_audio {
            self.audio.configure(&new_audio);
        }
        self.apply_mix_settings();
        self.mark_dirty();
    }

    fn toggle_passthrough(&mut self) {
        let settings = &mut self.library.settings;
        settings.passthrough = !settings.passthrough;
        self.audio
            .configure(&AudioConfig::from_settings(&self.library.settings));
        self.mark_dirty();
    }

    fn mark_dirty(&mut self) {
        self.last_change = Some(Instant::now());
    }

    /// Handles one command. Returns `false` when the controller should stop.
    pub fn handle(&mut self, cmd: Command) -> bool {
        tracing::trace!("command {cmd:?}");
        match cmd {
            Command::Play(id) => self.play(id, None),
            Command::Preview {
                sound,
                trim_start_ms,
                trim_end_ms,
            } => self.play(sound, Some((trim_start_ms, trim_end_ms))),
            Command::Stop(id) => self.send_mixer(MixerMsg::Stop(id)),
            Command::StopAll => self.send_mixer(MixerMsg::StopAll),
            Command::TogglePassthrough => self.toggle_passthrough(),
            Command::Import { paths, tab } => self.import(paths, tab),
            Command::UpdateSound(sound) => self.update_sound(*sound),
            Command::DeleteSound(id) => self.delete_sound(id),
            Command::MoveSound { sound, tab, index } => {
                self.library.move_sound(sound, tab, index);
                self.mark_dirty();
            }
            Command::AddTab(name) => {
                self.library.add_tab(name);
                self.mark_dirty();
            }
            Command::RenameTab(id, name) => {
                self.library.rename_tab(id, name);
                self.mark_dirty();
            }
            Command::DeleteTab(id) => match self.library.delete_tab(id) {
                Ok(()) => self.mark_dirty(),
                Err(e) => self.notices.push(format!("Could not delete tab: {e}.")),
            },
            Command::MoveTab(id, index) => {
                self.library.move_tab(id, index);
                self.mark_dirty();
            }
            Command::UpdateSettings(settings) => self.update_settings(*settings),
            Command::DismissNotice(index) => {
                if index < self.notices.len() {
                    self.notices.remove(index);
                }
            }
            Command::Notice(text) => self.notices.push(text),
            Command::BackendStatus(status) => self.backend = status,
            Command::Devices(devices) => self.devices = devices,
            Command::MicReady(mic) => self.send_mixer(MixerMsg::AttachMic(mic)),
            Command::MicClosed => self.send_mixer(MixerMsg::DetachMic),
            Command::ClipLoaded {
                sound,
                file,
                result,
            } => self.clip_loaded(sound, file, result),
            Command::Shutdown => return false,
        }
        self.publish();
        true
    }

    fn publish(&mut self) {
        self.revision += 1;
        self.snapshot.set(Snapshot {
            revision: self.revision,
            library: self.library.clone(),
            clips: self.clips.clone(),
            backend: self.backend.clone(),
            devices: self.devices.clone(),
            notices: self.notices.clone(),
            hotkey_revision: self.hotkey_revision,
        });
        (self.on_change)();
    }

    /// Periodic housekeeping: frees finished clips and writes debounced saves.
    pub fn tick(&mut self, now: Instant) {
        self.mixer.collect_garbage();
        if let Some(changed) = self.last_change
            && now.duration_since(changed) >= SAVE_DEBOUNCE
        {
            self.flush();
        }
    }

    pub fn flush(&mut self) {
        if self.last_change.take().is_some()
            && let Err(e) = self.store.save(&self.library)
        {
            tracing::error!("saving library failed: {e}");
            self.notices.push(format!("Saving the library failed: {e}"));
            self.publish();
        }
    }
}

#[derive(Clone)]
pub struct ControllerHandle {
    commands: Sender<Command>,
    snapshot: SharedSnapshot,
    meters: Arc<Meters>,
}

impl ControllerHandle {
    pub fn send(&self, cmd: Command) {
        let _ = self.commands.send(cmd);
    }

    pub fn sender(&self) -> Sender<Command> {
        self.commands.clone()
    }

    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.snapshot.get()
    }

    pub fn meters(&self) -> &Arc<Meters> {
        &self.meters
    }
}

/// Starts the controller on its own thread. The returned join handle
/// finishes after a `Shutdown` command, once the library is saved.
pub fn spawn_controller(
    store: LibraryStore,
    outcome: LoadOutcome,
    mixer: MixerHandle,
    meters: Arc<Meters>,
    audio_factory: impl FnOnce(Sender<Command>) -> Box<dyn AudioControl> + Send + 'static,
    on_change: ChangeCallback,
) -> (ControllerHandle, thread::JoinHandle<()>) {
    let (tx, rx) = channel();
    let snapshot = SharedSnapshot::default();
    let shared = snapshot.clone();
    let controller_tx = tx.clone();
    let join = thread::Builder::new()
        .name("controller".into())
        .spawn(move || {
            let audio = audio_factory(controller_tx.clone());
            let mut controller =
                Controller::new(store, outcome, mixer, audio, controller_tx, on_change);
            controller.snapshot = shared;
            controller.publish();
            loop {
                match rx.recv_timeout(TICK) {
                    Ok(cmd) => {
                        if !controller.handle(cmd) {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                controller.tick(Instant::now());
            }
            controller.flush();
        })
        .expect("spawn controller thread");
    (
        ControllerHandle {
            commands: tx,
            snapshot,
            meters,
        },
        join,
    )
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
