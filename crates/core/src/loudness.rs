use crate::clip::{CHANNELS, Clip, ENGINE_RATE};
use ebur128::{EbuR128, Mode};

pub const MAX_NORMALIZATION_DB: f32 = 24.0;

/// Integrated loudness in LUFS, or `None` for silence.
pub fn measure_lufs(clip: &Clip) -> Option<f64> {
    let mut meter = EbuR128::new(CHANNELS as u32, ENGINE_RATE, Mode::I).ok()?;
    meter.add_frames_f32(&clip.samples).ok()?;
    meter.loudness_global().ok().filter(|l| l.is_finite())
}

pub fn normalization_gain_db(lufs: f64, target: f64) -> f32 {
    ((target - lufs) as f32).clamp(-MAX_NORMALIZATION_DB, MAX_NORMALIZATION_DB)
}

pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

#[cfg(test)]
#[path = "loudness_tests.rs"]
mod tests;
