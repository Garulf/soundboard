use crate::clip::{CHANNELS, ENGINE_RATE};

/// -1 dBFS.
pub const LIMITER_CEILING: f32 = 0.891_250_9;
const RELEASE_SECONDS: f32 = 0.05;

/// Zero-latency peak limiter: gain drops instantly to keep each frame under
/// the ceiling and recovers smoothly afterwards.
pub struct Limiter {
    gain: f32,
    release: f32,
}

impl Default for Limiter {
    fn default() -> Self {
        Self {
            gain: 1.0,
            release: 1.0 - (-1.0 / (RELEASE_SECONDS * ENGINE_RATE as f32)).exp(),
        }
    }
}

impl Limiter {
    pub fn process(&mut self, buf: &mut [f32]) {
        for frame in buf.as_chunks_mut::<CHANNELS>().0 {
            let peak = frame.iter().fold(0.0f32, |p, s| p.max(s.abs()));
            let needed = if peak > LIMITER_CEILING {
                LIMITER_CEILING / peak
            } else {
                1.0
            };
            if needed < self.gain {
                self.gain = needed;
            } else {
                self.gain += (needed - self.gain) * self.release;
            }
            for s in frame.iter_mut() {
                *s = (*s * self.gain).clamp(-LIMITER_CEILING, LIMITER_CEILING);
            }
        }
    }
}
