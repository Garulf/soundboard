use super::*;
use crate::clip::Clip;
use crate::ids::SoundId;
use crate::mixer::{Meters, MixerMsg, PlayRequest, mixer};
use crate::model::{PlayMode, Route};
use std::sync::Arc;

fn ramp_clip(frames: usize) -> Arc<Clip> {
    let samples = (0..frames)
        .flat_map(|i| {
            let v = (i % 1000) as f32 / 2000.0;
            [v, v]
        })
        .collect();
    Arc::new(Clip { samples })
}

fn playing_pump(frames: usize) -> (crate::mixer::MixerHandle, Arc<BusPump>) {
    let (mut handle, mixer) = mixer(Arc::new(Meters::default()));
    let clip = ramp_clip(frames);
    assert!(handle.send(MixerMsg::Play(PlayRequest {
        sound: SoundId::new(),
        end_frame: clip.frames(),
        clip,
        start_frame: 0,
        gain: 0.5,
        route: Route::Both,
        mode: PlayMode::Overlap,
        looping: true,
    })));
    (handle, BusPump::new(mixer))
}

#[test]
fn lockstep_pulls_deliver_the_same_audio_on_both_buses() {
    let (_handle, pump) = playing_pump(48_000);
    let mut mic = vec![0.0; 2 * 300];
    let mut monitor = vec![0.0; 2 * 300];
    for _ in 0..20 {
        pump.pull(Bus::Mic, &mut mic);
        pump.pull(Bus::Monitor, &mut monitor);
        assert_eq!(mic, monitor);
    }
    assert!(mic.iter().any(|&s| s != 0.0));
}

#[test]
fn pulled_audio_is_continuous_across_calls() {
    let (_handle, pump) = playing_pump(48_000);
    let mut collected = Vec::new();
    let mut buf = vec![0.0; 2 * 100];
    for _ in 0..10 {
        pump.pull(Bus::Monitor, &mut buf);
        collected.extend_from_slice(&buf);
    }
    for frame in 1..1000 {
        let expected = (frame % 1000) as f32 / 2000.0 * 0.5;
        assert!(
            (collected[frame * 2] - expected).abs() < 1e-6,
            "frame {frame}"
        );
    }
}

#[test]
fn an_unconsumed_bus_stays_bounded() {
    let (_handle, pump) = playing_pump(48_000);
    let mut monitor = vec![0.0; 2 * 512];
    for _ in 0..2000 {
        pump.pull(Bus::Monitor, &mut monitor);
    }
    assert!(pump.queued_frames(Bus::Mic) <= MAX_QUEUED_FRAMES);
}

#[test]
fn rate_adapter_at_engine_rate_is_transparent() {
    let mut adapter = RateAdapter::new(crate::clip::ENGINE_RATE);
    let mut n = 0.0f32;
    let mut out = vec![0.0; 2 * 64];
    adapter.pull(&mut out, |buf| {
        for s in buf.iter_mut() {
            *s = n;
            n += 1.0;
        }
    });
    assert_eq!(out[0], 0.0);
    assert_eq!(out[127], 127.0);
}

#[test]
fn rate_adapter_at_44k_consumes_source_at_engine_rate() {
    let mut adapter = RateAdapter::new(44_100);
    let mut source_frames = 0usize;
    let mut out = vec![0.0; 2 * 441];
    for _ in 0..100 {
        adapter.pull(&mut out, |buf| source_frames += buf.len() / 2);
    }
    let diff = source_frames as i64 - 48_000;
    assert!(
        diff.abs() <= 2 * RATE_ADAPTER_BLOCK as i64,
        "{source_frames}"
    );
}

#[test]
fn rate_adapter_interpolates_between_frames() {
    let mut adapter = RateAdapter::new(24_000);
    let mut n = 0.0f32;
    let mut out = vec![0.0; 2 * 8];
    adapter.pull(&mut out, |buf| {
        for frame in buf.chunks_mut(2) {
            frame.fill(n);
            n += 1.0;
        }
    });
    let left: Vec<f32> = out.iter().step_by(2).copied().collect();
    let deltas: Vec<f32> = left.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(
        deltas[2..].iter().all(|d| (d - 2.0).abs() < 1e-4),
        "{left:?}"
    );
}

#[test]
fn a_slower_consumer_does_not_accumulate_latency() {
    let (_handle, pump) = playing_pump(48_000);
    let mut fast = vec![0.0; 2 * 480];
    let mut slow = vec![0.0; 2 * 440];
    for _ in 0..500 {
        pump.pull(Bus::Monitor, &mut fast);
        pump.pull(Bus::Mic, &mut slow);
    }
    assert!(
        pump.queued_frames(Bus::Mic) <= 440 + 2 * PUMP_BLOCK_FRAMES,
        "{}",
        pump.queued_frames(Bus::Mic)
    );
}
