//! Library model, decoding, mixer and controller.

pub mod clip;
pub mod ids;
pub mod loudness;
pub mod mic;
pub mod mixer;
pub mod model;
pub mod store;

pub use ids::{SoundId, TabId};
pub use model::*;
pub use store::{LibraryStore, LoadOutcome, StoreError};
