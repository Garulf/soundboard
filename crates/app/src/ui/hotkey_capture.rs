use egui::{Event, Key, Modifiers, Ui};
use soundboard_platform::hotkey::{Accelerator, Key as HotKey};

pub fn egui_key_to_accelerator(key: Key, mods: Modifiers) -> Option<Accelerator> {
    let key = HotKey::from_name(key.name())?;
    Some(Accelerator {
        ctrl: mods.ctrl,
        shift: mods.shift,
        alt: mods.alt,
        logo: mods.mac_cmd,
        key,
    })
}

/// A button that shows the current hotkey and, when clicked, records the next
/// key press. Escape cancels, Backspace or Delete clears. Returns the new
/// value when it changed.
pub fn hotkey_button(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    current: &Option<String>,
) -> Option<Option<String>> {
    let id = ui.make_persistent_id(id_salt);
    let mut capturing = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    let label = if capturing {
        "Press a key...".to_string()
    } else {
        current.clone().unwrap_or_else(|| "Set hotkey".into())
    };
    let response = ui
        .button(label)
        .on_hover_text("Click, then press the key combination. Esc cancels, Backspace clears.");
    if response.clicked() {
        capturing = !capturing;
    }
    let mut result = None;
    if capturing {
        let pressed = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
        });
        match pressed {
            Some((Key::Escape, _)) => capturing = false,
            Some((Key::Backspace | Key::Delete, m)) if m.is_none() => {
                capturing = false;
                result = Some(None);
            }
            Some((key, mods)) => {
                if let Some(acc) = egui_key_to_accelerator(key, mods) {
                    capturing = false;
                    result = Some(Some(acc.to_string()));
                }
            }
            None => {}
        }
        if response.clicked_elsewhere() {
            capturing = false;
        }
    }
    ui.data_mut(|d| d.insert_temp(id, capturing));
    result.filter(|new| new != current)
}

#[cfg(test)]
#[path = "hotkey_capture_tests.rs"]
mod tests;
