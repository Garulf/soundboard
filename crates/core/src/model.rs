use crate::ids::{SoundId, TabId};
use serde::{Deserialize, Serialize};

pub const LIBRARY_VERSION: u32 = 1;
pub const DEFAULT_REMOTE_PORT: u16 = 7373;
pub const DEFAULT_TARGET_LUFS: f64 = -18.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    Mic,
    Monitor,
    Both,
}

impl Route {
    pub fn to_mic(self) -> bool {
        matches!(self, Route::Mic | Route::Both)
    }

    pub fn to_monitor(self) -> bool {
        matches!(self, Route::Monitor | Route::Both)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayMode {
    #[default]
    Overlap,
    Restart,
    Toggle,
    Exclusive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sound {
    pub id: SoundId,
    pub name: String,
    pub file: String,
    pub color: [u8; 3],
    pub volume: f32,
    pub route: Option<Route>,
    pub mode: PlayMode,
    pub looping: bool,
    pub trim_start_ms: u64,
    pub trim_end_ms: Option<u64>,
    pub normalize: bool,
    pub lufs: Option<f64>,
    pub hotkey: Option<String>,
    pub tab: TabId,
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            id: SoundId::new(),
            name: String::new(),
            file: String::new(),
            color: [70, 90, 140],
            volume: 1.0,
            route: None,
            mode: PlayMode::Overlap,
            looping: false,
            trim_start_ms: 0,
            trim_end_ms: None,
            normalize: true,
            lufs: None,
            hotkey: None,
            tab: TabId::new(),
        }
    }
}

impl Sound {
    pub fn new(name: impl Into<String>, file: impl Into<String>, tab: TabId) -> Self {
        Self {
            name: name.into(),
            file: file.into(),
            tab,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tab {
    pub id: TabId,
    pub name: String,
    #[serde(default)]
    pub order: Vec<SoundId>,
}

impl Tab {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: TabId::new(),
            name: name.into(),
            order: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoteSettings {
    pub enabled: bool,
    pub port: u16,
    pub token: String,
}

impl Default for RemoteSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_REMOTE_PORT,
            token: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub default_route: Route,
    pub mic_bus_volume: f32,
    pub monitor_bus_volume: f32,
    pub passthrough: bool,
    pub mic_volume: f32,
    pub normalize_target_lufs: f64,
    pub monitor_device: Option<String>,
    pub mic_device: Option<String>,
    pub cable_device: Option<String>,
    pub stop_all_hotkey: Option<String>,
    pub passthrough_hotkey: Option<String>,
    pub remote: RemoteSettings,
    pub start_minimized: bool,
    pub close_to_tray: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_route: Route::Both,
            mic_bus_volume: 1.0,
            monitor_bus_volume: 0.7,
            passthrough: true,
            mic_volume: 1.0,
            normalize_target_lufs: DEFAULT_TARGET_LUFS,
            monitor_device: None,
            mic_device: None,
            cable_device: None,
            stop_all_hotkey: None,
            passthrough_hotkey: None,
            remote: RemoteSettings::default(),
            start_minimized: false,
            close_to_tray: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LibraryError {
    #[error("the last tab cannot be deleted")]
    LastTab,
    #[error("no such tab")]
    UnknownTab,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Library {
    pub version: u32,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub tabs: Vec<Tab>,
    #[serde(default)]
    pub sounds: Vec<Sound>,
}

impl Default for Library {
    fn default() -> Self {
        Self::new_default()
    }
}

impl Library {
    pub fn new_default() -> Self {
        Self {
            version: LIBRARY_VERSION,
            settings: Settings::default(),
            tabs: vec![Tab::new("Sounds")],
            sounds: Vec::new(),
        }
    }

    pub fn sound(&self, id: SoundId) -> Option<&Sound> {
        self.sounds.iter().find(|s| s.id == id)
    }

    pub fn sound_mut(&mut self, id: SoundId) -> Option<&mut Sound> {
        self.sounds.iter_mut().find(|s| s.id == id)
    }

    pub fn tab(&self, id: TabId) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    fn tab_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    pub fn effective_route(&self, sound: &Sound) -> Route {
        sound.route.unwrap_or(self.settings.default_route)
    }

    pub fn add_tab(&mut self, name: impl Into<String>) -> TabId {
        let tab = Tab::new(name);
        let id = tab.id;
        self.tabs.push(tab);
        id
    }

    pub fn rename_tab(&mut self, id: TabId, name: impl Into<String>) {
        if let Some(tab) = self.tab_mut(id) {
            tab.name = name.into();
        }
    }

    pub fn move_tab(&mut self, id: TabId, index: usize) {
        if let Some(from) = self.tabs.iter().position(|t| t.id == id) {
            let tab = self.tabs.remove(from);
            let index = index.min(self.tabs.len());
            self.tabs.insert(index, tab);
        }
    }

    pub fn delete_tab(&mut self, id: TabId) -> Result<(), LibraryError> {
        let index = self
            .tabs
            .iter()
            .position(|t| t.id == id)
            .ok_or(LibraryError::UnknownTab)?;
        if self.tabs.len() == 1 {
            return Err(LibraryError::LastTab);
        }
        let removed = self.tabs.remove(index);
        let target = self.tabs[0].id;
        for sound_id in &removed.order {
            if let Some(sound) = self.sound_mut(*sound_id) {
                sound.tab = target;
            }
        }
        self.tabs[0].order.extend(removed.order);
        Ok(())
    }

    /// Adds the sound to the end of its tab, falling back to the first tab if
    /// the sound names a tab that does not exist.
    pub fn add_sound(&mut self, mut sound: Sound) {
        if self.tab(sound.tab).is_none() {
            sound.tab = self.tabs[0].id;
        }
        let id = sound.id;
        let tab = sound.tab;
        self.sounds.push(sound);
        if let Some(tab) = self.tab_mut(tab) {
            tab.order.push(id);
        }
    }

    pub fn delete_sound(&mut self, id: SoundId) {
        self.sounds.retain(|s| s.id != id);
        for tab in &mut self.tabs {
            tab.order.retain(|s| *s != id);
        }
    }

    pub fn move_sound(&mut self, id: SoundId, to: TabId, index: usize) {
        if self.sound(id).is_none() || self.tab(to).is_none() {
            return;
        }
        for tab in &mut self.tabs {
            tab.order.retain(|s| *s != id);
        }
        if let Some(sound) = self.sound_mut(id) {
            sound.tab = to;
        }
        if let Some(tab) = self.tab_mut(to) {
            let index = index.min(tab.order.len());
            tab.order.insert(index, id);
        }
    }

    pub fn sounds_in(&self, tab: TabId) -> Vec<&Sound> {
        self.tab(tab)
            .map(|t| t.order.iter().filter_map(|id| self.sound(*id)).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
