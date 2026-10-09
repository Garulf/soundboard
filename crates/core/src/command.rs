use crate::audio::{BackendStatus, DeviceList};
use crate::clip::Clip;
use crate::ids::{SoundId, TabId};
use crate::mic::MicInput;
use crate::model::{Settings, Sound};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct LoadedClip {
    pub clip: Arc<Clip>,
    pub lufs: Option<f64>,
}

pub enum Command {
    Play(SoundId),
    Preview {
        sound: SoundId,
        trim_start_ms: u64,
        trim_end_ms: Option<u64>,
    },
    Stop(SoundId),
    StopAll,
    TogglePassthrough,
    Import {
        paths: Vec<PathBuf>,
        tab: TabId,
    },
    UpdateSound(Box<Sound>),
    DeleteSound(SoundId),
    MoveSound {
        sound: SoundId,
        tab: TabId,
        index: usize,
    },
    AddTab(String),
    RenameTab(TabId, String),
    DeleteTab(TabId),
    MoveTab(TabId, usize),
    UpdateSettings(Box<Settings>),
    DismissNotice(usize),
    Notice(String),
    BackendStatus(BackendStatus),
    Devices(DeviceList),
    MicReady(Box<MicInput>),
    MicClosed,
    ClipLoaded {
        sound: SoundId,
        file: Option<String>,
        result: Result<LoadedClip, String>,
    },
    Shutdown,
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Play(id) => write!(f, "Play({id})"),
            Command::Preview { sound, .. } => write!(f, "Preview({sound})"),
            Command::Stop(id) => write!(f, "Stop({id})"),
            Command::StopAll => f.write_str("StopAll"),
            Command::TogglePassthrough => f.write_str("TogglePassthrough"),
            Command::Import { paths, .. } => write!(f, "Import({paths:?})"),
            Command::UpdateSound(s) => write!(f, "UpdateSound({})", s.id),
            Command::DeleteSound(id) => write!(f, "DeleteSound({id})"),
            Command::MoveSound { sound, .. } => write!(f, "MoveSound({sound})"),
            Command::AddTab(name) => write!(f, "AddTab({name})"),
            Command::RenameTab(id, _) => write!(f, "RenameTab({id})"),
            Command::DeleteTab(id) => write!(f, "DeleteTab({id})"),
            Command::MoveTab(id, i) => write!(f, "MoveTab({id}, {i})"),
            Command::UpdateSettings(_) => f.write_str("UpdateSettings"),
            Command::DismissNotice(i) => write!(f, "DismissNotice({i})"),
            Command::Notice(text) => write!(f, "Notice({text})"),
            Command::BackendStatus(s) => write!(f, "BackendStatus({s:?})"),
            Command::Devices(_) => f.write_str("Devices"),
            Command::MicReady(_) => f.write_str("MicReady"),
            Command::MicClosed => f.write_str("MicClosed"),
            Command::ClipLoaded { sound, result, .. } => {
                write!(f, "ClipLoaded({sound}, ok={})", result.is_ok())
            }
            Command::Shutdown => f.write_str("Shutdown"),
        }
    }
}
