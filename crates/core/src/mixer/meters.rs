use super::{Bus, VOICE_CAPACITY};
use crate::ids::SoundId;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Stable non-zero key for a sound, so meters can be read without locks.
pub fn sound_key(id: SoundId) -> u64 {
    (id.0.as_u128() as u64) | 1
}

struct Slot {
    key: AtomicU64,
    progress: AtomicU32,
}

/// Lock-free state the audio thread publishes for the UI: per-bus peaks and
/// the progress of each active voice.
pub struct Meters {
    peaks: [AtomicU32; 2],
    slots: [Slot; VOICE_CAPACITY],
}

impl Default for Meters {
    fn default() -> Self {
        Self {
            peaks: [AtomicU32::new(0), AtomicU32::new(0)],
            slots: std::array::from_fn(|_| Slot {
                key: AtomicU64::new(0),
                progress: AtomicU32::new(0),
            }),
        }
    }
}

impl Meters {
    pub fn peak(&self, bus: Bus) -> f32 {
        f32::from_bits(self.peaks[bus as usize].load(Ordering::Relaxed))
    }

    pub fn playing(&self) -> Vec<(u64, f32)> {
        self.slots
            .iter()
            .filter_map(|slot| {
                let key = slot.key.load(Ordering::Relaxed);
                (key != 0).then(|| (key, f32::from_bits(slot.progress.load(Ordering::Relaxed))))
            })
            .collect()
    }

    pub fn progress_of(&self, id: SoundId) -> Option<f32> {
        let key = sound_key(id);
        self.playing()
            .into_iter()
            .filter(|(k, _)| *k == key)
            .map(|(_, p)| p)
            .reduce(f32::max)
    }

    pub(super) fn set_peak(&self, bus: Bus, value: f32) {
        self.peaks[bus as usize].store(value.to_bits(), Ordering::Relaxed);
    }

    pub(super) fn publish(&self, active: impl Iterator<Item = (u64, f32)>) {
        let mut used = 0;
        for (slot, (key, progress)) in self.slots.iter().zip(active) {
            slot.key.store(key, Ordering::Relaxed);
            slot.progress.store(progress.to_bits(), Ordering::Relaxed);
            used += 1;
        }
        for slot in &self.slots[used..] {
            slot.key.store(0, Ordering::Relaxed);
        }
    }
}
