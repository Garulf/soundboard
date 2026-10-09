use soundboard_core::{Command, Library, SoundId};
use soundboard_platform::hotkey::{Accelerator, Binding};

pub const STOP_ALL: &str = "stop_all";
pub const PASSTHROUGH: &str = "passthrough";
const SOUND_PREFIX: &str = "sound:";

pub fn sound_binding_id(id: SoundId) -> String {
    format!("{SOUND_PREFIX}{id}")
}

fn binding(id: String, description: String, hotkey: &Option<String>) -> Option<Binding> {
    let accelerator: Accelerator = hotkey.as_deref()?.parse().ok()?;
    Some(Binding {
        id,
        description,
        accelerator,
    })
}

pub fn bindings_from(library: &Library) -> Vec<Binding> {
    let sounds = library.sounds.iter().filter_map(|sound| {
        binding(
            sound_binding_id(sound.id),
            format!("Play {}", sound.name),
            &sound.hotkey,
        )
    });
    let settings = &library.settings;
    let globals = [
        binding(
            STOP_ALL.into(),
            "Stop all sounds".into(),
            &settings.stop_all_hotkey,
        ),
        binding(
            PASSTHROUGH.into(),
            "Toggle microphone passthrough".into(),
            &settings.passthrough_hotkey,
        ),
    ];
    sounds.chain(globals.into_iter().flatten()).collect()
}

pub fn command_for(binding_id: &str) -> Option<Command> {
    match binding_id {
        STOP_ALL => Some(Command::StopAll),
        PASSTHROUGH => Some(Command::TogglePassthrough),
        other => other
            .strip_prefix(SOUND_PREFIX)?
            .parse()
            .ok()
            .map(Command::Play),
    }
}

#[cfg(test)]
#[path = "hotkeys_tests.rs"]
mod tests;
