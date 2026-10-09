#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod hotkeys;
mod icon;
mod instance;
mod logging;
mod remote_api;
mod tray;
mod ui;

use anyhow::Context as _;
use directories::ProjectDirs;
use soundboard_core::{BusPump, Command, LibraryStore, Meters, mixer, spawn_controller};
use soundboard_platform::audio::{Reporter, start_audio};
use soundboard_platform::hotkey::create_hotkeys;
use std::sync::{Arc, OnceLock};

const ICON_SIZE: u32 = 64;

fn already_running() {
    tracing::warn!("another instance is already running");
    rfd::MessageDialog::new()
        .set_title("Soundboard")
        .set_description("Soundboard is already running. Look for it in your system tray.")
        .set_level(rfd::MessageLevel::Info)
        .show();
}

fn main() -> anyhow::Result<()> {
    let dirs =
        ProjectDirs::from("", "", "soundboard").context("could not find a home directory")?;
    let _log_guard = logging::init(&dirs.data_dir().join("logs"));
    tracing::info!("soundboard {} starting", env!("CARGO_PKG_VERSION"));

    let Some(_instance) = instance::acquire(dirs.config_dir())? else {
        already_running();
        return Ok(());
    };

    let store = LibraryStore::new(dirs.config_dir());
    let outcome = store.load().context("could not read the sound library")?;
    let start_hidden = outcome.library.settings.start_minimized;

    let meters = Arc::new(Meters::default());
    let (mixer_handle, mixer) = mixer(meters.clone());
    let pump = BusPump::new(mixer);

    let repaint: Arc<OnceLock<egui::Context>> = Arc::default();
    let on_change = {
        let repaint = repaint.clone();
        Box::new(move || {
            if let Some(ctx) = repaint.get() {
                ctx.request_repaint();
            }
        })
    };
    let (controller, controller_thread) = spawn_controller(
        store,
        outcome,
        mixer_handle,
        meters,
        move |tx| {
            let report: Reporter = Arc::new(move |cmd| {
                let _ = tx.send(cmd);
            });
            start_audio(pump, report)
        },
        on_change,
    );

    let icon = egui::IconData {
        rgba: icon::icon_rgba(ICON_SIZE),
        width: ICON_SIZE,
        height: ICON_SIZE,
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Soundboard")
            .with_app_id("soundboard")
            .with_inner_size([980.0, 640.0])
            .with_min_inner_size([520.0, 360.0])
            .with_icon(Arc::new(icon)),
        ..Default::default()
    };

    let app_controller = controller.clone();
    let result = eframe::run_native(
        "Soundboard",
        options,
        Box::new(move |cc| {
            let _ = repaint.set(cc.egui_ctx.clone());
            let hotkey_sender = app_controller.sender();
            let hotkeys = create_hotkeys(Arc::new(move |binding: &str| {
                if let Some(command) = hotkeys::command_for(binding) {
                    let _ = hotkey_sender.send(command);
                }
            }));
            let tray = tray::create(&cc.egui_ctx);
            if start_hidden && tray.is_some() {
                cc.egui_ctx
                    .send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
            Ok(Box::new(app::SoundboardApp::new(
                &cc.egui_ctx,
                app_controller,
                hotkeys,
                tray,
            )))
        }),
    );

    controller.send(Command::Shutdown);
    let _ = controller_thread.join();
    result.map_err(|e| anyhow::anyhow!("the window could not be created: {e}"))
}
