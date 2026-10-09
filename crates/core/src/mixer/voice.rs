use super::FADE_FRAMES;
use crate::clip::{CHANNELS, Clip};
use crate::ids::SoundId;
use crate::model::Route;
use std::sync::Arc;

pub(super) struct Voice {
    pub sound: SoundId,
    pub clip: Arc<Clip>,
    pub pos: usize,
    pub start: usize,
    pub end: usize,
    pub gain: f32,
    pub route: Route,
    pub looping: bool,
    pub fade_left: Option<usize>,
    pub age: u64,
    pub finished: bool,
}

impl Voice {
    pub fn is_active(&self) -> bool {
        self.fade_left.is_none() && !self.finished
    }

    pub fn fade_out(&mut self) {
        if self.fade_left.is_none() {
            self.fade_left = Some(FADE_FRAMES);
        }
    }

    pub fn progress(&self) -> f32 {
        (self.pos - self.start) as f32 / (self.end - self.start) as f32
    }

    pub fn mix(&mut self, mic: &mut [f32], monitor: &mut [f32]) {
        let to_mic = self.route.to_mic();
        let to_monitor = self.route.to_monitor();
        let frames = mic.len() / CHANNELS;
        for i in 0..frames {
            let fade = match self.fade_left {
                Some(0) => {
                    self.finished = true;
                    return;
                }
                Some(left) => {
                    self.fade_left = Some(left - 1);
                    left as f32 / FADE_FRAMES as f32
                }
                None => 1.0,
            };
            let gain = self.gain * fade;
            for ch in 0..CHANNELS {
                let s = self.clip.samples[self.pos * CHANNELS + ch] * gain;
                if to_mic {
                    mic[i * CHANNELS + ch] += s;
                }
                if to_monitor {
                    monitor[i * CHANNELS + ch] += s;
                }
            }
            self.pos += 1;
            if self.pos >= self.end {
                if self.looping {
                    self.pos = self.start;
                } else {
                    self.finished = true;
                    return;
                }
            }
        }
    }
}
