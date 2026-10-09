use super::*;

#[test]
fn finds_vb_cable_input() {
    let names = ["Speakers (Realtek)", "CABLE Input (VB-Audio Virtual Cable)"];
    assert_eq!(find_cable(&names), Some(1));
}

#[test]
fn finds_virtual_audio_driver_speaker() {
    let names = ["Headphones", "Speakers (Virtual Audio Driver by MTT)"];
    assert_eq!(find_cable(&names), Some(1));
}

#[test]
fn ignores_cable_output_side() {
    let names = ["CABLE Output (VB-Audio Virtual Cable)", "Speakers"];
    assert_eq!(find_cable(&names), None);
}

#[test]
fn returns_none_without_a_cable() {
    assert_eq!(find_cable(&["Speakers", "HDMI"]), None);
}

#[test]
fn stereo_frames_upmix_mono_and_drop_extra_channels() {
    let mut out = Vec::new();
    push_stereo(&[0.5, 0.25], 1, |s| out.push(s));
    assert_eq!(out, [0.5, 0.5, 0.25, 0.25]);
    out.clear();
    push_stereo(&[0.1, 0.2, 0.3, 0.4, 0.5, 0.6], 3, |s| out.push(s));
    assert_eq!(out, [0.1, 0.2, 0.4, 0.5]);
}

#[test]
fn writes_stereo_into_wider_and_mono_layouts() {
    let stereo = [0.2, 0.4];
    let mut quad = [9.0f32; 4];
    write_frames(&mut quad, 4, &stereo);
    assert_eq!(quad, [0.2, 0.4, 0.0, 0.0]);
    let mut mono = [9.0f32; 1];
    write_frames(&mut mono, 1, &stereo);
    assert!((mono[0] - 0.3).abs() < 1e-6);
}
