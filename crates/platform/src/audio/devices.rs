pub const VIRTUAL_MIC_NODE: &str = "soundboard_mic";
pub const VIRTUAL_MIC_DESCRIPTION: &str = "Soundboard Mic";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Output,
    Input,
}

/// Classifies a PipeWire node as a user-selectable device, hiding streams,
/// sink monitors and our own virtual microphone.
pub fn device_kind(media_class: &str, node_name: &str) -> Option<DeviceKind> {
    if node_name == VIRTUAL_MIC_NODE || node_name.ends_with(".monitor") {
        return None;
    }
    match media_class {
        "Audio/Sink" => Some(DeviceKind::Output),
        "Audio/Source" | "Audio/Source/Virtual" => Some(DeviceKind::Input),
        _ => None,
    }
}

pub fn device_label(description: Option<&str>, nick: Option<&str>, name: &str) -> String {
    [description, nick]
        .into_iter()
        .flatten()
        .find(|s| !s.is_empty())
        .unwrap_or(name)
        .to_string()
}

#[cfg(test)]
#[path = "devices_tests.rs"]
mod tests;
