use crate::clip::{CHANNELS, ENGINE_RATE};
pub use crate::mixer::Bus;
use crate::mixer::Mixer;
use std::sync::{Arc, Mutex, PoisonError};

pub const PUMP_BLOCK_FRAMES: usize = 256;
pub const MAX_QUEUED_FRAMES: usize = 8192;
pub const RATE_ADAPTER_BLOCK: usize = 256;

/// Fixed-capacity sample FIFO that never allocates after construction and
/// drops its oldest samples when overfilled.
struct SampleQueue {
    buf: Vec<f32>,
    head: usize,
    len: usize,
}

impl SampleQueue {
    fn new(capacity: usize) -> Self {
        Self {
            buf: vec![0.0; capacity],
            head: 0,
            len: 0,
        }
    }

    fn push(&mut self, samples: &[f32]) {
        let cap = self.buf.len();
        for &s in samples {
            if self.len == cap {
                self.head = (self.head + 1) % cap;
                self.len -= 1;
            }
            let tail = (self.head + self.len) % cap;
            self.buf[tail] = s;
            self.len += 1;
        }
    }

    fn drop_oldest(&mut self, samples: usize) {
        let n = samples.min(self.len);
        self.head = (self.head + n) % self.buf.len();
        self.len -= n;
    }

    fn pop_into(&mut self, out: &mut [f32]) {
        let cap = self.buf.len();
        for slot in out.iter_mut() {
            if self.len == 0 {
                *slot = 0.0;
                continue;
            }
            *slot = self.buf[self.head];
            self.head = (self.head + 1) % cap;
            self.len -= 1;
        }
    }
}

struct PumpInner {
    mixer: Mixer,
    queues: [SampleQueue; 2],
    scratch: [Vec<f32>; 2],
}

/// Lets two independent device callbacks (virtual mic and monitor) pull
/// their bus from a single mixer. Whichever callback needs audio first
/// renders a block for both buses; the other bus is queued for its callback.
pub struct BusPump {
    inner: Mutex<PumpInner>,
}

impl BusPump {
    pub fn new(mixer: Mixer) -> Arc<Self> {
        let samples = MAX_QUEUED_FRAMES * CHANNELS;
        Arc::new(Self {
            inner: Mutex::new(PumpInner {
                mixer,
                queues: [SampleQueue::new(samples), SampleQueue::new(samples)],
                scratch: [
                    vec![0.0; PUMP_BLOCK_FRAMES * CHANNELS],
                    vec![0.0; PUMP_BLOCK_FRAMES * CHANNELS],
                ],
            }),
        })
    }

    pub fn pull(&self, bus: Bus, out: &mut [f32]) {
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let inner = &mut *guard;
        let want = out.len().min(MAX_QUEUED_FRAMES * CHANNELS);
        while inner.queues[bus as usize].len < want {
            let [mic, monitor] = &mut inner.scratch;
            inner.mixer.render(mic, monitor);
            inner.queues[Bus::Mic as usize].push(mic);
            inner.queues[Bus::Monitor as usize].push(monitor);
        }
        let queue = &mut inner.queues[bus as usize];
        queue.pop_into(out);
        let limit = want + 2 * PUMP_BLOCK_FRAMES * CHANNELS;
        if queue.len > limit {
            queue.drop_oldest(queue.len - want);
        }
    }

    pub fn queued_frames(&self, bus: Bus) -> usize {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        guard.queues[bus as usize].len / CHANNELS
    }
}

/// Converts engine-rate audio to a device running at another rate using
/// linear interpolation. Allocation-free after construction.
pub struct RateAdapter {
    step: f64,
    frac: f64,
    prev: [f32; 2],
    cur: [f32; 2],
    block: Vec<f32>,
    block_pos: usize,
}

impl RateAdapter {
    pub fn new(device_rate: u32) -> Self {
        Self {
            step: ENGINE_RATE as f64 / device_rate.max(1) as f64,
            frac: 1.0,
            prev: [0.0; 2],
            cur: [0.0; 2],
            block: vec![0.0; RATE_ADAPTER_BLOCK * CHANNELS],
            block_pos: RATE_ADAPTER_BLOCK,
        }
    }

    pub fn is_passthrough(&self) -> bool {
        self.step == 1.0
    }

    fn next_frame(&mut self, src: &mut impl FnMut(&mut [f32])) -> [f32; 2] {
        if self.block_pos == RATE_ADAPTER_BLOCK {
            src(&mut self.block);
            self.block_pos = 0;
        }
        let i = self.block_pos * CHANNELS;
        self.block_pos += 1;
        [self.block[i], self.block[i + 1]]
    }

    pub fn pull(&mut self, out: &mut [f32], mut src: impl FnMut(&mut [f32])) {
        if self.is_passthrough() {
            src(out);
            return;
        }
        for frame in out.as_chunks_mut::<CHANNELS>().0 {
            while self.frac >= 1.0 {
                self.prev = self.cur;
                self.cur = self.next_frame(&mut src);
                self.frac -= 1.0;
            }
            let t = self.frac as f32;
            for (ch, sample) in frame.iter_mut().enumerate() {
                *sample = self.prev[ch] + (self.cur[ch] - self.prev[ch]) * t;
            }
            self.frac += self.step;
        }
    }
}

#[cfg(test)]
#[path = "pump_tests.rs"]
mod tests;
