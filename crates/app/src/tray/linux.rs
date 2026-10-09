use super::{TrayEvent, notify};
use crate::icon::{icon_rgba, rgba_to_argb};
use ksni::blocking::TrayMethods;
use ksni::menu::StandardItem;
use std::sync::mpsc::Sender;

struct SoundboardTray {
    tx: Sender<TrayEvent>,
    ctx: egui::Context,
}

impl SoundboardTray {
    fn item(label: &str, event: TrayEvent) -> ksni::MenuItem<Self> {
        StandardItem {
            label: label.into(),
            activate: Box::new(move |tray: &mut Self| notify(&tray.tx, &tray.ctx, event)),
            ..Default::default()
        }
        .into()
    }
}

impl ksni::Tray for SoundboardTray {
    fn id(&self) -> String {
        "soundboard".into()
    }

    fn title(&self) -> String {
        "Soundboard".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        [32, 64]
            .into_iter()
            .map(|size| ksni::Icon {
                width: size,
                height: size,
                data: rgba_to_argb(&icon_rgba(size as u32)),
            })
            .collect()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        notify(&self.tx, &self.ctx, TrayEvent::Show);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            Self::item("Show Soundboard", TrayEvent::Show),
            Self::item("Stop all sounds", TrayEvent::StopAll),
            ksni::MenuItem::Separator,
            Self::item("Quit", TrayEvent::Quit),
        ]
    }
}

pub fn spawn(tx: Sender<TrayEvent>, ctx: egui::Context) -> Option<Box<dyn std::any::Any>> {
    match (SoundboardTray { tx, ctx }).spawn() {
        Ok(handle) => Some(Box::new(handle)),
        Err(e) => {
            tracing::warn!("tray icon unavailable: {e}");
            None
        }
    }
}
