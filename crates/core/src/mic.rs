use crate::clip::{CHANNELS, ENGINE_RATE};
use rtrb::{Consumer, Producer, RingBuffer};

pub const MAX_DRIFT: f64 = 0.005;
pub const TARGET_LATENCY_MS: u32 = 20;
const SMOOTHING: f64 = 0.05;
const GAIN: f64 = 0.01;

/// Turns ring-buffer fill level into a resampling ratio correction so a
/// capture device and an output device running on different clocks stay in
/// step without the buffer draining or overflowing.
#[derive(Debug, Clone)]
pub struct DriftController {
    target: f64,
    smoothed: f64,
}

impl DriftController {
    pub fn new(target_frames: usize) -> Self {
        let target = target_frames.max(1) as f64;
        Self {
            target,
            smoothed: target,
        }
    }

    pub fn update(&mut self, fill_frames: usize) -> f64 {
        self.smoothed += (fill_frames as f64 - self.smoothed) * SMOOTHING;
        let error = (self.smoothed - self.target) / self.target;
        1.0 + (error * GAIN).clamp(-MAX_DRIFT, MAX_DRIFT)
    }
}

pub fn mic_ring(in_rate: u32) -> (Producer<f32>, MicInput) {
    let capacity = (in_rate as usize / 2) * CHANNELS;
    let (producer, consumer) = RingBuffer::new(capacity);
    (producer, MicInput::new(consumer, in_rate))
}

/// Reads captured stereo frames from a ring and resamples them to the engine
/// rate with linear interpolation, compensating for clock drift.
pub struct MicInput {
    consumer: Consumer<f32>,
    nominal_step: f64,
    target_frames: usize,
    drift: DriftController,
    primed: bool,
    frac: f64,
    prev: [f32; 2],
    cur: [f32; 2],
}

impl MicInput {
    pub fn new(consumer: Consumer<f32>, in_rate: u32) -> Self {
        let target_frames = (in_rate * TARGET_LATENCY_MS / 1000) as usize;
        Self {
            consumer,
            nominal_step: in_rate as f64 / ENGINE_RATE as f64,
            target_frames,
            drift: DriftController::new(target_frames),
            primed: false,
            frac: 1.0,
            prev: [0.0; 2],
            cur: [0.0; 2],
        }
    }

    pub fn buffered_frames(&self) -> usize {
        self.consumer.slots() / CHANNELS
    }

    fn capacity_frames(&self) -> usize {
        self.consumer.buffer().capacity() / CHANNELS
    }

    fn pop_frame(&mut self) -> Option<[f32; 2]> {
        if self.consumer.slots() < CHANNELS {
            return None;
        }
        let l = self.consumer.pop().ok()?;
        let r = self.consumer.pop().ok()?;
        Some([l, r])
    }

    fn skip_frames(&mut self, frames: usize) {
        for _ in 0..frames {
            if self.pop_frame().is_none() {
                break;
            }
        }
    }

    fn reset(&mut self) {
        self.primed = false;
        self.frac = 1.0;
        self.prev = [0.0; 2];
        self.cur = [0.0; 2];
    }

    /// Adds `gain`-scaled mic audio into `out` (interleaved stereo).
    pub fn read_into(&mut self, out: &mut [f32], gain: f32) {
        let fill = self.buffered_frames();
        if !self.primed {
            if fill < self.target_frames {
                return;
            }
            self.primed = true;
        }
        if fill > self.capacity_frames() * 3 / 4 {
            self.skip_frames(fill - self.target_frames);
        }
        let step = self.nominal_step * self.drift.update(self.buffered_frames());
        for frame in out.as_chunks_mut::<CHANNELS>().0 {
            while self.frac >= 1.0 {
                match self.pop_frame() {
                    Some(next) => {
                        self.prev = self.cur;
                        self.cur = next;
                        self.frac -= 1.0;
                    }
                    None => {
                        self.reset();
                        return;
                    }
                }
            }
            let t = self.frac as f32;
            for (ch, sample) in frame.iter_mut().enumerate() {
                *sample += (self.prev[ch] + (self.cur[ch] - self.prev[ch]) * t) * gain;
            }
            self.frac += step;
        }
    }
}

#[cfg(test)]
#[path = "mic_tests.rs"]
mod tests;
