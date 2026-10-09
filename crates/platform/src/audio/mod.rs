pub mod cable;
#[cfg(windows)]
mod cpal_backend;
pub mod devices;
#[cfg(target_os = "linux")]
mod pipewire;
pub mod retry;

use soundboard_core::{AudioConfig, AudioControl, BackendStatus, BusPump, Command};
use std::sync::Arc;

pub type Reporter = Arc<dyn Fn(Command) + Send + Sync>;

struct NoAudio;

impl AudioControl for NoAudio {
    fn configure(&mut self, _cfg: &AudioConfig) {}
}

/// Starts the platform audio backend. On failure the error is reported as a
/// backend status and a do-nothing backend is returned so the app still runs.
pub fn start_audio(pump: Arc<BusPump>, report: Reporter) -> Box<dyn AudioControl> {
    match start_platform(pump, report.clone()) {
        Ok(backend) => backend,
        Err(e) => {
            tracing::error!("audio backend failed to start: {e}");
            report(Command::BackendStatus(BackendStatus::Error(e)));
            Box::new(NoAudio)
        }
    }
}

#[cfg(target_os = "linux")]
fn start_platform(pump: Arc<BusPump>, report: Reporter) -> Result<Box<dyn AudioControl>, String> {
    pipewire::start(pump, report).map(|b| Box::new(b) as Box<dyn AudioControl>)
}

#[cfg(windows)]
fn start_platform(pump: Arc<BusPump>, report: Reporter) -> Result<Box<dyn AudioControl>, String> {
    cpal_backend::start(pump, report).map(|b| Box::new(b) as Box<dyn AudioControl>)
}

#[cfg(not(any(target_os = "linux", windows)))]
fn start_platform(_pump: Arc<BusPump>, _report: Reporter) -> Result<Box<dyn AudioControl>, String> {
    Err("no audio backend for this platform".into())
}
