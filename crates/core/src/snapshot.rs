use crate::audio::{BackendStatus, DeviceList};
use crate::clip::Clip;
use crate::ids::SoundId;
use crate::model::Library;
use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

#[derive(Debug, Clone)]
pub enum ClipStatus {
    Loading,
    Ready(Arc<Clip>),
    Failed(String),
}

/// Immutable view of controller state published for the UI and remote.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub revision: u64,
    pub library: Library,
    pub clips: HashMap<SoundId, ClipStatus>,
    pub backend: BackendStatus,
    pub devices: DeviceList,
    pub notices: Vec<String>,
    pub hotkey_revision: u64,
}

#[derive(Clone, Default)]
pub struct SharedSnapshot(Arc<RwLock<Arc<Snapshot>>>);

impl SharedSnapshot {
    pub fn get(&self) -> Arc<Snapshot> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn set(&self, snapshot: Snapshot) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = Arc::new(snapshot);
    }
}
