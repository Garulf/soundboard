use super::*;
use crate::clip::Clip;
use crate::ids::SoundId;
use crate::model::{PlayMode, Route};
use std::sync::Arc;

const BLOCK: usize = 256;

fn dc_clip(value: f32, frames: usize) -> Arc<Clip> {
    Arc::new(Clip {
        samples: vec![value; frames * 2],
    })
}

fn request(sound: SoundId, clip: &Arc<Clip>) -> PlayRequest {
    PlayRequest {
        sound,
        clip: clip.clone(),
        start_frame: 0,
        end_frame: clip.frames(),
        gain: 1.0,
        route: Route::Both,
        mode: PlayMode::Overlap,
        looping: false,
    }
}

struct Rig {
    handle: MixerHandle,
    mixer: Mixer,
    meters: Arc<Meters>,
}

impl Rig {
    fn new() -> Self {
        let meters = Arc::new(Meters::default());
        let (handle, mixer) = mixer(meters.clone());
        let mut rig = Self {
            handle,
            mixer,
            meters,
        };
        rig.send(MixerMsg::BusGain(Bus::Mic, 1.0));
        rig.send(MixerMsg::BusGain(Bus::Monitor, 1.0));
        rig
    }

    fn send(&mut self, msg: MixerMsg) {
        assert!(self.handle.send(msg));
    }

    fn play(&mut self, req: PlayRequest) {
        self.send(MixerMsg::Play(req));
    }

    fn render(&mut self, frames: usize) -> (Vec<f32>, Vec<f32>) {
        let mut mic = vec![0.0; frames * 2];
        let mut monitor = vec![0.0; frames * 2];
        self.mixer.render(&mut mic, &mut monitor);
        (mic, monitor)
    }
}

fn last(buf: &[f32]) -> f32 {
    buf[buf.len() - 1]
}

#[test]
fn route_mic_writes_only_the_mic_bus() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.25, 1000);
    rig.play(PlayRequest {
        route: Route::Mic,
        ..request(SoundId::new(), &clip)
    });
    let (mic, monitor) = rig.render(BLOCK);
    assert_eq!(mic[0], 0.25);
    assert!(monitor.iter().all(|&s| s == 0.0));
}

#[test]
fn route_both_writes_both_buses() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.25, 1000);
    rig.play(request(SoundId::new(), &clip));
    let (mic, monitor) = rig.render(BLOCK);
    assert_eq!(mic[10], 0.25);
    assert_eq!(monitor[10], 0.25);
}

#[test]
fn bus_gain_scales_output() {
    let mut rig = Rig::new();
    rig.send(MixerMsg::BusGain(Bus::Monitor, 0.5));
    let clip = dc_clip(0.25, 1000);
    rig.play(request(SoundId::new(), &clip));
    let (mic, monitor) = rig.render(BLOCK);
    assert_eq!(mic[0], 0.25);
    assert_eq!(monitor[0], 0.125);
}

#[test]
fn overlap_mode_sums_two_triggers() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.2, 1000);
    let id = SoundId::new();
    rig.play(request(id, &clip));
    rig.play(request(id, &clip));
    let (_, monitor) = rig.render(BLOCK);
    assert!((monitor[0] - 0.4).abs() < 1e-6);
}

#[test]
fn restart_mode_keeps_a_single_voice() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.2, 48_000);
    let id = SoundId::new();
    let restart = PlayRequest {
        mode: PlayMode::Restart,
        ..request(id, &clip)
    };
    rig.play(restart.clone());
    rig.render(BLOCK);
    rig.play(restart);
    rig.render(FADE_FRAMES + BLOCK);
    let (_, monitor) = rig.render(BLOCK);
    assert!((monitor[0] - 0.2).abs() < 1e-6, "{}", monitor[0]);
}

#[test]
fn toggle_mode_second_trigger_stops() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.2, 48_000);
    let toggle = PlayRequest {
        mode: PlayMode::Toggle,
        ..request(SoundId::new(), &clip)
    };
    rig.play(toggle.clone());
    rig.render(BLOCK);
    rig.play(toggle);
    rig.render(FADE_FRAMES);
    let (_, monitor) = rig.render(BLOCK);
    assert!(monitor.iter().all(|&s| s == 0.0));
}

#[test]
fn exclusive_mode_stops_other_sounds() {
    let mut rig = Rig::new();
    let long = dc_clip(0.3, 48_000);
    let other = dc_clip(0.1, 48_000);
    rig.play(request(SoundId::new(), &long));
    rig.render(BLOCK);
    rig.play(PlayRequest {
        mode: PlayMode::Exclusive,
        ..request(SoundId::new(), &other)
    });
    rig.render(FADE_FRAMES);
    let (_, monitor) = rig.render(BLOCK);
    assert!((monitor[0] - 0.1).abs() < 1e-6, "{}", monitor[0]);
}

#[test]
fn looping_voice_wraps_to_start() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.2, 100);
    rig.play(PlayRequest {
        looping: true,
        ..request(SoundId::new(), &clip)
    });
    let (_, monitor) = rig.render(1000);
    assert!(monitor.iter().all(|&s| (s - 0.2).abs() < 1e-6));
}

#[test]
fn non_looping_voice_ends_with_silence() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.2, 100);
    rig.play(request(SoundId::new(), &clip));
    let (_, monitor) = rig.render(200);
    assert_eq!(monitor[2 * 99], 0.2);
    assert_eq!(monitor[2 * 100], 0.0);
}

#[test]
fn stop_fades_out_over_fade_frames() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.5, 48_000);
    let id = SoundId::new();
    rig.play(request(id, &clip));
    rig.render(BLOCK);
    rig.send(MixerMsg::Stop(id));
    let (_, monitor) = rig.render(FADE_FRAMES + 10);
    assert!(monitor[0] > 0.45 && monitor[0] <= 0.5);
    assert!(monitor[2 * (FADE_FRAMES / 2)] < 0.3);
    assert_eq!(last(&monitor), 0.0);
}

#[test]
fn stop_all_silences_everything() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.5, 48_000);
    rig.play(request(SoundId::new(), &clip));
    rig.play(request(SoundId::new(), &clip));
    rig.render(BLOCK);
    rig.send(MixerMsg::StopAll);
    rig.render(FADE_FRAMES);
    let (mic, monitor) = rig.render(BLOCK);
    assert!(mic.iter().chain(&monitor).all(|&s| s == 0.0));
}

#[test]
fn voices_beyond_the_limit_steal_the_oldest() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.001, 48_000);
    let first = SoundId::new();
    rig.play(request(first, &clip));
    for _ in 0..MAX_VOICES {
        rig.play(request(SoundId::new(), &clip));
    }
    rig.render(FADE_FRAMES + BLOCK);
    let playing = rig.meters.playing();
    assert_eq!(playing.len(), MAX_VOICES);
    assert!(!playing.iter().any(|(key, _)| *key == sound_key(first)));
}

#[test]
fn trim_window_is_respected() {
    let mut rig = Rig::new();
    let mut samples = vec![0.0; 2 * 300];
    samples[2 * 100..2 * 200].fill(0.7);
    let clip = Arc::new(Clip { samples });
    rig.play(PlayRequest {
        start_frame: 100,
        end_frame: 200,
        ..request(SoundId::new(), &clip)
    });
    let (_, monitor) = rig.render(150);
    assert!(monitor[..200].iter().all(|&s| s == 0.7));
    assert!(monitor[200..].iter().all(|&s| s == 0.0));
}

#[test]
fn trim_end_before_start_plays_silence() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.7, 300);
    rig.play(PlayRequest {
        start_frame: 200,
        end_frame: 100,
        ..request(SoundId::new(), &clip)
    });
    let (mic, monitor) = rig.render(BLOCK);
    assert!(mic.iter().chain(&monitor).all(|&s| s == 0.0));
    assert!(rig.meters.playing().is_empty());
}

#[test]
fn trim_start_past_clip_end_plays_silence() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.7, 300);
    rig.play(PlayRequest {
        start_frame: 5000,
        end_frame: 9000,
        ..request(SoundId::new(), &clip)
    });
    let (_, monitor) = rig.render(BLOCK);
    assert!(monitor.iter().all(|&s| s == 0.0));
}

#[test]
fn trim_end_past_clip_is_clamped() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.7, 100);
    rig.play(PlayRequest {
        end_frame: 10_000,
        ..request(SoundId::new(), &clip)
    });
    let (_, monitor) = rig.render(200);
    assert_eq!(monitor[2 * 99], 0.7);
    assert_eq!(monitor[2 * 100], 0.0);
}

#[test]
fn limiter_keeps_stacked_clips_under_ceiling_on_mic() {
    let mut rig = Rig::new();
    let clip = dc_clip(1.0, 48_000);
    for _ in 0..4 {
        rig.play(request(SoundId::new(), &clip));
    }
    let (mic, monitor) = rig.render(4800);
    assert!(mic.iter().all(|s| s.abs() <= LIMITER_CEILING + 1e-6));
    assert!(monitor.iter().all(|s| s.abs() <= 1.0));
}

#[test]
fn finished_clips_are_returned_as_garbage() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.1, 10);
    rig.play(request(SoundId::new(), &clip));
    rig.render(BLOCK);
    assert_eq!(
        Arc::strong_count(&clip),
        2,
        "mixer must not drop the clip itself"
    );
    rig.handle.collect_garbage();
    assert_eq!(Arc::strong_count(&clip), 1);
}

#[test]
fn meters_report_progress_and_peaks() {
    let mut rig = Rig::new();
    let clip = dc_clip(0.5, 1000);
    let id = SoundId::new();
    rig.play(request(id, &clip));
    rig.render(500);
    let playing = rig.meters.playing();
    assert_eq!(playing.len(), 1);
    assert_eq!(playing[0].0, sound_key(id));
    assert!((playing[0].1 - 0.5).abs() < 0.01);
    assert!((rig.meters.peak(Bus::Monitor) - 0.5).abs() < 1e-6);
}

#[test]
fn attached_mic_is_mixed_into_mic_bus_only() {
    let mut rig = Rig::new();
    let (mut producer, mic) = crate::mic::mic_ring(crate::clip::ENGINE_RATE);
    for _ in 0..crate::clip::ENGINE_RATE / 10 {
        producer.push(0.3).unwrap();
        producer.push(0.3).unwrap();
    }
    rig.send(MixerMsg::AttachMic(Box::new(mic)));
    rig.send(MixerMsg::MicGain(0.5));
    let (mic_bus, monitor) = rig.render(BLOCK);
    assert!((mic_bus[100] - 0.15).abs() < 1e-3, "{}", mic_bus[100]);
    assert!(monitor.iter().all(|&s| s == 0.0));
}
