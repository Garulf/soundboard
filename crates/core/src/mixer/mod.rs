mod limiter;
mod meters;
mod voice;

pub use limiter::LIMITER_CEILING;
pub use meters::{Meters, sound_key};

use crate::clip::Clip;
use crate::ids::SoundId;
use crate::mic::MicInput;
use crate::model::{PlayMode, Route};
use limiter::Limiter;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::Arc;
use voice::Voice;

pub const MAX_VOICES: usize = 32;
pub const VOICE_CAPACITY: usize = 48;
pub const FADE_FRAMES: usize = 480;
const MESSAGE_CAPACITY: usize = 256;
const GARBAGE_CAPACITY: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bus {
    Mic = 0,
    Monitor = 1,
}

#[derive(Clone)]
pub struct PlayRequest {
    pub sound: SoundId,
    pub clip: Arc<Clip>,
    pub start_frame: usize,
    pub end_frame: usize,
    pub gain: f32,
    pub route: Route,
    pub mode: PlayMode,
    pub looping: bool,
}

pub enum MixerMsg {
    Play(PlayRequest),
    Stop(SoundId),
    StopAll,
    BusGain(Bus, f32),
    AttachMic(Box<MicInput>),
    DetachMic,
    MicGain(f32),
}

/// Values the audio thread hands back so they are freed off the real-time path.
pub enum Garbage {
    Clip(Arc<Clip>),
    Mic(Box<MicInput>),
}

pub struct MixerHandle {
    tx: Producer<MixerMsg>,
    garbage: Consumer<Garbage>,
}

impl MixerHandle {
    pub fn send(&mut self, msg: MixerMsg) -> bool {
        self.tx.push(msg).is_ok()
    }

    pub fn collect_garbage(&mut self) {
        while self.garbage.pop().is_ok() {}
    }
}

pub struct Mixer {
    rx: Consumer<MixerMsg>,
    garbage: Producer<Garbage>,
    voices: Vec<Voice>,
    next_age: u64,
    bus_gain: [f32; 2],
    mic: Option<Box<MicInput>>,
    mic_gain: f32,
    limiter: Limiter,
    meters: Arc<Meters>,
}

pub fn mixer(meters: Arc<Meters>) -> (MixerHandle, Mixer) {
    let (tx, rx) = RingBuffer::new(MESSAGE_CAPACITY);
    let (garbage_tx, garbage_rx) = RingBuffer::new(GARBAGE_CAPACITY);
    (
        MixerHandle {
            tx,
            garbage: garbage_rx,
        },
        Mixer {
            rx,
            garbage: garbage_tx,
            voices: Vec::with_capacity(VOICE_CAPACITY),
            next_age: 0,
            bus_gain: [1.0, 1.0],
            mic: None,
            mic_gain: 1.0,
            limiter: Limiter::default(),
            meters,
        },
    )
}

impl Mixer {
    fn discard(&mut self, item: Garbage) {
        // If the ring is full the value is dropped here; that only costs a
        // deallocation on this thread in a pathological case.
        let _ = self.garbage.push(item);
    }

    fn handle(&mut self, msg: MixerMsg) {
        match msg {
            MixerMsg::Play(req) => self.play(req),
            MixerMsg::Stop(id) => self
                .voices
                .iter_mut()
                .filter(|v| v.sound == id)
                .for_each(Voice::fade_out),
            MixerMsg::StopAll => self.voices.iter_mut().for_each(Voice::fade_out),
            MixerMsg::BusGain(bus, gain) => self.bus_gain[bus as usize] = gain,
            MixerMsg::AttachMic(mic) => {
                if let Some(old) = self.mic.replace(mic) {
                    self.discard(Garbage::Mic(old));
                }
            }
            MixerMsg::DetachMic => {
                if let Some(old) = self.mic.take() {
                    self.discard(Garbage::Mic(old));
                }
            }
            MixerMsg::MicGain(gain) => self.mic_gain = gain,
        }
    }

    fn play(&mut self, req: PlayRequest) {
        let end = req.end_frame.min(req.clip.frames());
        if req.start_frame >= end {
            self.discard(Garbage::Clip(req.clip));
            return;
        }
        match req.mode {
            PlayMode::Overlap => {}
            PlayMode::Restart => self
                .voices
                .iter_mut()
                .filter(|v| v.sound == req.sound)
                .for_each(Voice::fade_out),
            PlayMode::Toggle => {
                let mut was_playing = false;
                for voice in self.voices.iter_mut().filter(|v| v.sound == req.sound) {
                    was_playing |= voice.is_active();
                    voice.fade_out();
                }
                if was_playing {
                    self.discard(Garbage::Clip(req.clip));
                    return;
                }
            }
            PlayMode::Exclusive => self.voices.iter_mut().for_each(Voice::fade_out),
        }
        self.make_room();
        self.next_age += 1;
        self.voices.push(Voice {
            sound: req.sound,
            clip: req.clip,
            pos: req.start_frame,
            start: req.start_frame,
            end,
            gain: req.gain,
            route: req.route,
            looping: req.looping,
            fade_left: None,
            age: self.next_age,
            finished: false,
        });
    }

    fn make_room(&mut self) {
        if self.voices.iter().filter(|v| v.is_active()).count() >= MAX_VOICES
            && let Some(oldest) = self
                .voices
                .iter_mut()
                .filter(|v| v.is_active())
                .min_by_key(|v| v.age)
        {
            oldest.fade_out();
        }
        if self.voices.len() >= VOICE_CAPACITY
            && let Some(index) = (0..self.voices.len()).min_by_key(|&i| self.voices[i].age)
        {
            let voice = self.voices.swap_remove(index);
            self.discard(Garbage::Clip(voice.clip));
        }
    }

    /// Renders one block into both buses (interleaved stereo, equal length).
    pub fn render(&mut self, mic: &mut [f32], monitor: &mut [f32]) {
        debug_assert_eq!(mic.len(), monitor.len());
        mic.fill(0.0);
        monitor.fill(0.0);
        while let Ok(msg) = self.rx.pop() {
            self.handle(msg);
        }

        for voice in &mut self.voices {
            voice.mix(mic, monitor);
        }
        let mut i = 0;
        while i < self.voices.len() {
            if self.voices[i].finished {
                let voice = self.voices.swap_remove(i);
                self.discard(Garbage::Clip(voice.clip));
            } else {
                i += 1;
            }
        }

        if let Some(input) = self.mic.as_mut() {
            input.read_into(mic, self.mic_gain);
        }
        apply_gain(mic, self.bus_gain[Bus::Mic as usize]);
        apply_gain(monitor, self.bus_gain[Bus::Monitor as usize]);
        self.limiter.process(mic);
        for s in monitor.iter_mut() {
            *s = s.clamp(-1.0, 1.0);
        }

        self.meters.set_peak(Bus::Mic, peak(mic));
        self.meters.set_peak(Bus::Monitor, peak(monitor));
        self.meters.publish(
            self.voices
                .iter()
                .filter(|v| v.is_active())
                .map(|v| (sound_key(v.sound), v.progress())),
        );
    }
}

fn apply_gain(buf: &mut [f32], gain: f32) {
    if gain != 1.0 {
        buf.iter_mut().for_each(|s| *s *= gain);
    }
}

fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0f32, |p, s| p.max(s.abs()))
}

#[cfg(test)]
mod tests;
