use crate::model::Settings;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BackendStatus {
    #[default]
    Starting,
    Ok,
    CableMissing,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceList {
    pub outputs: Vec<DeviceInfo>,
    pub inputs: Vec<DeviceInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AudioConfig {
    pub monitor_device: Option<String>,
    pub mic_device: Option<String>,
    pub cable_device: Option<String>,
    pub passthrough: bool,
}

impl AudioConfig {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            monitor_device: settings.monitor_device.clone(),
            mic_device: settings.mic_device.clone(),
            cable_device: settings.cable_device.clone(),
            passthrough: settings.passthrough,
        }
    }
}

/// Implemented by platform audio backends; the controller tells the backend
/// which devices to use and whether to capture the real microphone.
pub trait AudioControl: Send {
    fn configure(&mut self, cfg: &AudioConfig);
}
