use super::*;
use crate::clip::Clip;

fn sine_clip(amp: f32, seconds: f32) -> Clip {
    let frames = (crate::clip::ENGINE_RATE as f32 * seconds) as usize;
    let samples = (0..frames)
        .flat_map(|i| {
            let v = amp
                * (std::f32::consts::TAU * 1000.0 * i as f32 / crate::clip::ENGINE_RATE as f32)
                    .sin();
            [v, v]
        })
        .collect();
    Clip { samples }
}

#[test]
fn full_scale_stereo_sine_measures_near_zero_lufs() {
    let lufs = measure_lufs(&sine_clip(1.0, 3.0)).unwrap();
    assert!((lufs - 0.0).abs() < 1.0, "lufs {lufs}");
}

#[test]
fn halving_amplitude_drops_about_six_lu() {
    let full = measure_lufs(&sine_clip(1.0, 3.0)).unwrap();
    let half = measure_lufs(&sine_clip(0.5, 3.0)).unwrap();
    assert!(((full - half) - 6.02).abs() < 0.5, "{full} {half}");
}

#[test]
fn silence_has_no_loudness() {
    assert_eq!(
        measure_lufs(&Clip {
            samples: vec![0.0; 96_000]
        }),
        None
    );
}

#[test]
fn gain_moves_clip_to_target() {
    assert_eq!(normalization_gain_db(-24.0, -18.0), 6.0);
    assert_eq!(normalization_gain_db(-10.0, -18.0), -8.0);
}

#[test]
fn gain_is_clamped_to_24_db() {
    assert_eq!(normalization_gain_db(-70.0, -18.0), 24.0);
    assert_eq!(normalization_gain_db(20.0, -18.0), -24.0);
}

#[test]
fn db_to_gain_converts() {
    assert!((db_to_gain(0.0) - 1.0).abs() < 1e-6);
    assert!((db_to_gain(-6.0206) - 0.5).abs() < 1e-3);
}
