//! Library model, decoding, mixer and controller.

pub mod audio;
pub mod clip;
pub mod command;
pub mod controller;
pub mod ids;
mod jobs;
pub mod loudness;
pub mod mic;
pub mod mixer;
pub mod model;
pub mod pump;
pub mod snapshot;
pub mod store;

pub use audio::{AudioConfig, AudioControl, BackendStatus, DeviceInfo, DeviceList};
pub use clip::{CHANNELS, Clip, ENGINE_RATE};
pub use command::{Command, LoadedClip};
pub use controller::{ControllerHandle, spawn_controller};
pub use ids::{SoundId, TabId};
pub use mic::{MicInput, mic_ring};
pub use mixer::{Bus, Meters, mixer, sound_key};
pub use model::*;
pub use pump::{BusPump, RateAdapter};
pub use snapshot::{ClipStatus, SharedSnapshot, Snapshot};
pub use store::{LibraryStore, LoadOutcome, StoreError};
