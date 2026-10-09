use super::hotkey_capture::hotkey_button;
use super::waveform;
use egui::{Context, RichText};
use soundboard_core::{ClipStatus, Command, PlayMode, Route, Snapshot, Sound};

const PEAK_BUCKETS: usize = 300;

pub struct Editor {
    draft: Sound,
    peaks: Option<Vec<f32>>,
    confirm_delete: bool,
}

pub enum EditorResult {
    Open,
    Closed,
}

fn route_label(route: Option<Route>) -> &'static str {
    match route {
        None => "Default",
        Some(Route::Both) => "Voice chat and me",
        Some(Route::Mic) => "Voice chat only",
        Some(Route::Monitor) => "Only me",
    }
}

fn mode_label(mode: PlayMode) -> (&'static str, &'static str) {
    match mode {
        PlayMode::Overlap => ("Overlap", "Every press starts another copy"),
        PlayMode::Restart => ("Restart", "Pressing again restarts from the beginning"),
        PlayMode::Toggle => ("Toggle", "Pressing again stops it"),
        PlayMode::Exclusive => ("Exclusive", "Stops every other sound first"),
    }
}

impl Editor {
    pub fn new(sound: Sound) -> Self {
        Self {
            draft: sound,
            peaks: None,
            confirm_delete: false,
        }
    }

    pub fn show(
        &mut self,
        ctx: &Context,
        snapshot: &Snapshot,
        commands: &mut Vec<Command>,
    ) -> EditorResult {
        let mut open = true;
        let mut close = false;
        let library = &snapshot.library;
        if library.sound(self.draft.id).is_none() {
            return EditorResult::Closed;
        }
        egui::Window::new("Edit sound")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                egui::Grid::new("sound-fields")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut self.draft.name);
                        ui.end_row();

                        ui.label("Color");
                        ui.color_edit_button_srgb(&mut self.draft.color);
                        ui.end_row();

                        ui.label("Volume");
                        ui.add(
                            egui::Slider::new(&mut self.draft.volume, 0.0..=2.0)
                                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                        );
                        ui.end_row();

                        ui.label("Plays to");
                        egui::ComboBox::from_id_salt("route")
                            .selected_text(route_label(self.draft.route))
                            .show_ui(ui, |ui| {
                                for route in [
                                    None,
                                    Some(Route::Both),
                                    Some(Route::Mic),
                                    Some(Route::Monitor),
                                ] {
                                    ui.selectable_value(
                                        &mut self.draft.route,
                                        route,
                                        route_label(route),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("When pressed");
                        egui::ComboBox::from_id_salt("mode")
                            .selected_text(mode_label(self.draft.mode).0)
                            .show_ui(ui, |ui| {
                                for mode in [
                                    PlayMode::Overlap,
                                    PlayMode::Restart,
                                    PlayMode::Toggle,
                                    PlayMode::Exclusive,
                                ] {
                                    let (name, help) = mode_label(mode);
                                    ui.selectable_value(&mut self.draft.mode, mode, name)
                                        .on_hover_text(help);
                                }
                            });
                        ui.end_row();

                        ui.label("");
                        ui.checkbox(&mut self.draft.looping, "Loop until stopped");
                        ui.end_row();

                        ui.label("");
                        let lufs = self
                            .draft
                            .lufs
                            .map(|l| format!(" (measured {l:.1} LUFS)"))
                            .unwrap_or_default();
                        ui.checkbox(
                            &mut self.draft.normalize,
                            format!("Even out loudness{lufs}"),
                        );
                        ui.end_row();

                        ui.label("Tab");
                        let tab_name = library
                            .tab(self.draft.tab)
                            .map(|t| t.name.clone())
                            .unwrap_or_default();
                        egui::ComboBox::from_id_salt("tab")
                            .selected_text(tab_name)
                            .show_ui(ui, |ui| {
                                for tab in &library.tabs {
                                    ui.selectable_value(&mut self.draft.tab, tab.id, &tab.name);
                                }
                            });
                        ui.end_row();

                        ui.label("Hotkey");
                        if let Some(hotkey) =
                            hotkey_button(ui, ("sound-hotkey", self.draft.id), &self.draft.hotkey)
                        {
                            self.draft.hotkey = hotkey;
                        }
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.label(RichText::new("Trim").strong());
                match snapshot.clips.get(&self.draft.id) {
                    Some(ClipStatus::Ready(clip)) => {
                        let peaks = self.peaks.get_or_insert_with(|| clip.peaks(PEAK_BUCKETS));
                        waveform::trim_editor(
                            ui,
                            clip,
                            peaks,
                            &mut self.draft.trim_start_ms,
                            &mut self.draft.trim_end_ms,
                        );
                        let end = self.draft.trim_end_ms.unwrap_or(clip.duration_ms());
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "{:.2}s to {:.2}s of {:.2}s",
                                self.draft.trim_start_ms as f32 / 1000.0,
                                end as f32 / 1000.0,
                                clip.duration_ms() as f32 / 1000.0
                            ));
                            if ui.small_button("Reset").clicked() {
                                self.draft.trim_start_ms = 0;
                                self.draft.trim_end_ms = None;
                            }
                        });
                    }
                    Some(ClipStatus::Loading) => {
                        ui.spinner();
                    }
                    Some(ClipStatus::Failed(e)) => {
                        ui.colored_label(super::theme::ERROR, e);
                    }
                    None => {}
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button("Preview")
                        .on_hover_text("Play the trimmed sound only to you")
                        .clicked()
                    {
                        commands.push(Command::Preview {
                            sound: self.draft.id,
                            trim_start_ms: self.draft.trim_start_ms,
                            trim_end_ms: self.draft.trim_end_ms,
                        });
                    }
                    if ui.button("Stop").clicked() {
                        commands.push(Command::Stop(self.draft.id));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Save").strong()).clicked() {
                            commands.push(Command::UpdateSound(Box::new(self.draft.clone())));
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                        if self.confirm_delete {
                            if ui
                                .button(RichText::new("Really delete").color(super::theme::ERROR))
                                .clicked()
                            {
                                commands.push(Command::DeleteSound(self.draft.id));
                                close = true;
                            }
                        } else if ui.button("Delete").clicked() {
                            self.confirm_delete = true;
                        }
                    });
                });
            });
        if open && !close {
            EditorResult::Open
        } else {
            EditorResult::Closed
        }
    }
}
