pub mod bottom_bar;
pub mod editor;
pub mod grid;
pub mod hotkey_capture;
pub mod qr;
pub mod settings;
pub mod tabs;
pub mod theme;
pub mod waveform;

use crate::hotkeys::sound_binding_id;
use soundboard_core::Sound;
use std::collections::HashMap;

/// The trigger the hotkey backend reports (the portal may let users change
/// it), falling back to the stored accelerator.
pub fn hotkey_label(sound: &Sound, triggers: &HashMap<String, String>) -> Option<String> {
    triggers
        .get(&sound_binding_id(sound.id))
        .cloned()
        .or_else(|| sound.hotkey.clone())
}
