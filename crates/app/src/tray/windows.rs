use super::{TrayEvent, notify};
use crate::icon::icon_rgba;
use std::sync::mpsc::Sender;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, TrayIconBuilder, TrayIconEvent};

pub fn spawn(tx: Sender<TrayEvent>, ctx: egui::Context) -> Option<Box<dyn std::any::Any>> {
    let show = MenuItem::new("Show Soundboard", true, None);
    let stop = MenuItem::new("Stop all sounds", true, None);
    let quit = MenuItem::new("Quit", true, None);
    let menu = Menu::new();
    let appended = menu
        .append(&show)
        .and_then(|_| menu.append(&stop))
        .and_then(|_| menu.append(&PredefinedMenuItem::separator()))
        .and_then(|_| menu.append(&quit));
    if let Err(e) = appended {
        tracing::warn!("tray menu failed: {e}");
        return None;
    }
    let ids = [
        (show.id().clone(), TrayEvent::Show),
        (stop.id().clone(), TrayEvent::StopAll),
        (quit.id().clone(), TrayEvent::Quit),
    ];
    {
        let tx = tx.clone();
        let ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some((_, action)) = ids.iter().find(|(id, _)| *id == event.id) {
                notify(&tx, &ctx, *action);
            }
        }));
    }
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        } = event
        {
            notify(&tx, &ctx, TrayEvent::Show);
        }
    }));
    let icon = Icon::from_rgba(icon_rgba(32), 32, 32).ok()?;
    match TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Soundboard")
        .with_icon(icon)
        .build()
    {
        Ok(tray) => Some(Box::new(tray)),
        Err(e) => {
            tracing::warn!("tray icon unavailable: {e}");
            None
        }
    }
}
