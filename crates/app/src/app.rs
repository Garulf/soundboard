use crate::hotkeys::bindings_from;
use crate::remote_api::RemoteManager;
use crate::tray::{Tray, TrayEvent};
use crate::ui::editor::{Editor, EditorResult};
use crate::ui::grid::{self, GridAction, GridContext};
use crate::ui::settings::SettingsWindow;
use crate::ui::tabs::{TabAction, TabBar};
use crate::ui::{bottom_bar, theme};
use egui::{Context, RichText, Ui, ViewportCommand};
use soundboard_core::{Command, ControllerHandle, SoundId, TabId, sound_key};
use soundboard_platform::hotkey::HotkeyProvider;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

const PLAYING_REPAINT: Duration = Duration::from_millis(33);

pub struct SoundboardApp {
    controller: ControllerHandle,
    hotkeys: Box<dyn HotkeyProvider>,
    hotkey_revision: u64,
    remote: RemoteManager,
    tray: Option<Tray>,
    quitting: bool,
    active_tab: Option<TabId>,
    search: String,
    tab_bar: TabBar,
    editor: Option<Editor>,
    settings: SettingsWindow,
}

impl SoundboardApp {
    pub fn new(
        ctx: &Context,
        controller: ControllerHandle,
        hotkeys: Box<dyn HotkeyProvider>,
        tray: Option<Tray>,
    ) -> Self {
        theme::apply(ctx);
        Self {
            remote: RemoteManager::new(controller.clone()),
            controller,
            hotkeys,
            hotkey_revision: 0,
            tray,
            quitting: false,
            active_tab: None,
            search: String::new(),
            tab_bar: TabBar::default(),
            editor: None,
            settings: SettingsWindow::default(),
        }
    }

    fn send_all(&self, commands: impl IntoIterator<Item = Command>) {
        for command in commands {
            self.controller.send(command);
        }
    }

    fn show_window(ctx: &Context) {
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
    }

    fn handle_tray(&mut self, ctx: &Context) {
        let events = self.tray.as_ref().map(Tray::poll).unwrap_or_default();
        for event in events {
            match event {
                TrayEvent::Show => Self::show_window(ctx),
                TrayEvent::StopAll => self.controller.send(Command::StopAll),
                TrayEvent::Quit => {
                    self.quitting = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }
    }

    fn handle_close(&mut self, ctx: &Context, close_to_tray: bool) {
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested && close_to_tray && self.tray.is_some() && !self.quitting {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        }
    }

    fn import_dropped_files(&self, ctx: &Context, tab: TabId) {
        let paths: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        if !paths.is_empty() {
            self.controller.send(Command::Import { paths, tab });
        }
    }

    fn pick_files(&self, tab: TabId) {
        let sender = self.controller.sender();
        std::thread::spawn(move || {
            let picked = rfd::FileDialog::new()
                .set_title("Add sounds")
                .add_filter(
                    "Audio",
                    &[
                        "wav", "mp3", "flac", "ogg", "oga", "m4a", "aac", "mp4", "aif", "aiff",
                        "caf", "mkv", "webm",
                    ],
                )
                .pick_files();
            if let Some(paths) = picked {
                let _ = sender.send(Command::Import { paths, tab });
            }
        });
    }

    fn top_bar(&mut self, ui: &mut Ui, snapshot: &soundboard_core::Snapshot) {
        ui.horizontal(|ui| {
            let actions = self.tab_bar.show(ui, &snapshot.library, self.active_tab);
            for action in actions {
                match action {
                    TabAction::Select(tab) => self.active_tab = tab,
                    TabAction::Command(command) => self.controller.send(command),
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Settings").clicked() {
                    self.settings.open = !self.settings.open;
                }
                let target = self.import_tab(snapshot);
                if ui.button("Add sounds").clicked() {
                    self.pick_files(target);
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Search")
                        .desired_width(160.0),
                );
            });
        });
    }

    fn import_tab(&self, snapshot: &soundboard_core::Snapshot) -> TabId {
        self.active_tab
            .filter(|t| snapshot.library.tab(*t).is_some())
            .unwrap_or(snapshot.library.tabs[0].id)
    }

    fn notices(&self, ui: &mut Ui, notices: &[String]) {
        for (index, notice) in notices.iter().enumerate() {
            egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .inner_margin(8.0)
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(notice);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Dismiss").clicked() {
                                self.controller.send(Command::DismissNotice(index));
                            }
                        });
                    });
                });
        }
    }
}

impl eframe::App for SoundboardApp {
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let snapshot = self.controller.snapshot();
        if snapshot.hotkey_revision != self.hotkey_revision {
            self.hotkey_revision = snapshot.hotkey_revision;
            self.hotkeys.bind(bindings_from(&snapshot.library));
        }
        self.remote.sync(&snapshot.library.settings.remote);
        self.handle_tray(ctx);
        self.handle_close(ctx, snapshot.library.settings.close_to_tray);
        if !self.controller.meters().playing().is_empty() {
            ctx.request_repaint_after(PLAYING_REPAINT);
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let snapshot = self.controller.snapshot();
        if self
            .active_tab
            .is_some_and(|t| snapshot.library.tab(t).is_none())
        {
            self.active_tab = None;
        }
        let playing = self.controller.meters().playing();
        let progress: HashMap<SoundId, f32> = snapshot
            .library
            .sounds
            .iter()
            .filter_map(|s| {
                let key = sound_key(s.id);
                playing
                    .iter()
                    .filter(|(k, _)| *k == key)
                    .map(|(_, p)| *p)
                    .reduce(f32::max)
                    .map(|p| (s.id, p))
            })
            .collect();
        let triggers = self.hotkeys.triggers();

        egui::Panel::top("tabs").show(ui, |ui| {
            ui.add_space(4.0);
            self.top_bar(ui, &snapshot);
            ui.add_space(4.0);
        });
        egui::Panel::bottom("controls").show(ui, |ui| {
            ui.add_space(4.0);
            let commands = bottom_bar::show(ui, &snapshot, self.controller.meters());
            self.send_all(commands);
            ui.add_space(4.0);
        });
        egui::CentralPanel::default().show(ui, |ui| {
            self.notices(ui, &snapshot.notices);
            let sounds = grid::visible_sounds(&snapshot.library, self.active_tab, &self.search);
            if sounds.is_empty() {
                ui.centered_and_justified(|ui| {
                    let text = if self.search.trim().is_empty() {
                        "Drop audio files here, or click \"Add sounds\"."
                    } else {
                        "No sounds match your search."
                    };
                    ui.label(RichText::new(text).size(16.0).weak());
                });
            } else {
                let grid_ctx = GridContext {
                    snapshot: &snapshot,
                    progress: &progress,
                    triggers: &triggers,
                };
                let reorderable = self.search.trim().is_empty();
                for action in grid::show(ui, &sounds, &grid_ctx, reorderable) {
                    match action {
                        GridAction::Command(command) => self.controller.send(command),
                        GridAction::Edit(id) => {
                            if let Some(sound) = snapshot.library.sound(id) {
                                self.editor = Some(Editor::new(sound.clone()));
                            }
                        }
                    }
                }
            }
        });
        self.import_dropped_files(&ctx, self.import_tab(&snapshot));

        let mut commands = Vec::new();
        if let Some(editor) = &mut self.editor
            && let EditorResult::Closed = editor.show(&ctx, &snapshot, &mut commands)
        {
            self.editor = None;
        }
        if self.settings.open {
            self.settings.show(
                &ctx,
                &snapshot,
                &self.hotkeys.status(),
                &self.remote,
                &mut commands,
            );
        }
        self.send_all(commands);
    }
}
