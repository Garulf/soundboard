use super::*;

#[test]
fn sinks_are_outputs_and_sources_are_inputs() {
    assert_eq!(
        device_kind("Audio/Sink", "alsa_output.usb"),
        Some(DeviceKind::Output)
    );
    assert_eq!(
        device_kind("Audio/Source", "alsa_input.usb"),
        Some(DeviceKind::Input)
    );
    assert_eq!(
        device_kind("Audio/Source/Virtual", "other_virtual"),
        Some(DeviceKind::Input)
    );
}

#[test]
fn our_own_virtual_mic_and_streams_are_hidden() {
    assert_eq!(device_kind("Audio/Source/Virtual", VIRTUAL_MIC_NODE), None);
    assert_eq!(device_kind("Stream/Output/Audio", "firefox"), None);
    assert_eq!(device_kind("Video/Source", "v4l2"), None);
}

#[test]
fn sink_monitors_are_not_offered_as_microphones() {
    assert_eq!(device_kind("Audio/Source", "alsa_output.pci.monitor"), None);
}

#[test]
fn label_prefers_description_then_nick_then_name() {
    assert_eq!(
        device_label(Some("Headset"), Some("hs"), "alsa.x"),
        "Headset"
    );
    assert_eq!(device_label(None, Some("hs"), "alsa.x"), "hs");
    assert_eq!(device_label(Some(""), None, "alsa.x"), "alsa.x");
}
