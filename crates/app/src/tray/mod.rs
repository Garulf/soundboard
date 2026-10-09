#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

use std::sync::mpsc::{Receiver, Sender, channel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    Show,
    StopAll,
    Quit,
}

/// System tray icon. Events are queued and the UI is woken to handle them.
pub struct Tray {
    rx: Receiver<TrayEvent>,
    _platform: Box<dyn std::any::Any>,
}

impl Tray {
    pub fn poll(&self) -> Vec<TrayEvent> {
        self.rx.try_iter().collect()
    }
}

fn notify(tx: &Sender<TrayEvent>, ctx: &egui::Context, event: TrayEvent) {
    let _ = tx.send(event);
    ctx.request_repaint();
}

pub fn create(ctx: &egui::Context) -> Option<Tray> {
    let (tx, rx) = channel();
    let platform = platform_tray(tx, ctx.clone())?;
    Some(Tray {
        rx,
        _platform: platform,
    })
}

#[cfg(target_os = "linux")]
fn platform_tray(tx: Sender<TrayEvent>, ctx: egui::Context) -> Option<Box<dyn std::any::Any>> {
    linux::spawn(tx, ctx)
}

#[cfg(windows)]
fn platform_tray(tx: Sender<TrayEvent>, ctx: egui::Context) -> Option<Box<dyn std::any::Any>> {
    windows::spawn(tx, ctx)
}

#[cfg(not(any(target_os = "linux", windows)))]
fn platform_tray(_tx: Sender<TrayEvent>, _ctx: egui::Context) -> Option<Box<dyn std::any::Any>> {
    None
}
