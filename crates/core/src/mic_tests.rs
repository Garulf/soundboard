use super::*;
use crate::clip::ENGINE_RATE;

#[test]
fn drift_ratio_rises_when_ring_is_too_full() {
    let mut drift = DriftController::new(1000);
    let mut ratio = 1.0;
    for _ in 0..200 {
        ratio = drift.update(1500);
    }
    assert!(ratio > 1.0, "{ratio}");
}

#[test]
fn drift_ratio_falls_when_ring_is_too_empty() {
    let mut drift = DriftController::new(1000);
    let mut ratio = 1.0;
    for _ in 0..200 {
        ratio = drift.update(500);
    }
    assert!(ratio < 1.0, "{ratio}");
}

#[test]
fn drift_ratio_is_clamped() {
    let mut drift = DriftController::new(1000);
    let mut ratio = 1.0;
    for _ in 0..10_000 {
        ratio = drift.update(1_000_000);
    }
    assert!(ratio <= 1.0 + MAX_DRIFT + 1e-12, "{ratio}");
    for _ in 0..10_000 {
        ratio = drift.update(0);
    }
    assert!(ratio >= 1.0 - MAX_DRIFT - 1e-12, "{ratio}");
}

fn feed(producer: &mut rtrb::Producer<f32>, frames: usize, value: f32) {
    for _ in 0..frames {
        producer.push(value).unwrap();
        producer.push(value).unwrap();
    }
}

#[test]
fn mic_waits_until_target_fill_before_playing() {
    let (mut producer, mut mic) = mic_ring(ENGINE_RATE);
    feed(&mut producer, 10, 0.5);
    let mut out = vec![0.0; 64];
    mic.read_into(&mut out, 1.0);
    assert!(out.iter().all(|&s| s == 0.0));
}

#[test]
fn mic_at_same_rate_passes_samples_through() {
    let (mut producer, mut mic) = mic_ring(ENGINE_RATE);
    feed(&mut producer, 4800, 0.5);
    let mut out = vec![0.0; 512];
    mic.read_into(&mut out, 1.0);
    assert!(out[64..].iter().all(|&s| (s - 0.5).abs() < 1e-6));
}

#[test]
fn mic_at_44k_consumes_input_at_the_input_rate() {
    let (mut producer, mut mic) = mic_ring(44_100);
    feed(&mut producer, 44_100 / 5, 0.25);
    let before = mic.buffered_frames();
    let mut out = vec![0.0; 4800 * 2];
    mic.read_into(&mut out, 1.0);
    let consumed = (before - mic.buffered_frames()) as f64;
    assert!((consumed - 4410.0).abs() < 4410.0 * 0.01, "{consumed}");
}

#[test]
fn mic_underrun_outputs_silence_without_panicking() {
    let (mut producer, mut mic) = mic_ring(ENGINE_RATE);
    feed(&mut producer, 1000, 0.5);
    let mut out = vec![0.0; 2 * 4000];
    mic.read_into(&mut out, 1.0);
    assert_eq!(out[out.len() - 1], 0.0);
}
