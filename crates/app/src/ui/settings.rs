use super::hotkey_capture::hotkey_button;
use super::qr::QrCache;
use crate::remote_api::{RemoteManager, generate_token, lan_ip, remote_url};
use egui::{Context, RichText, Ui, vec2};
use soundboard_core::{Command, DeviceInfo, Route, Settings, Snapshot};
use soundboard_platform::hotkey::HotkeyStatus;

#[derive(Default)]
pub struct SettingsWindow {
    pub open: bool,
    qr: QrCache,
}

fn device_combo(
    ui: &mut Ui,
    id: &str,
    default_label: &str,
    devices: &[DeviceInfo],
    selected: &mut Option<String>,
) -> bool {
    let current = selected
        .as_ref()
        .map(|id| {
            devices
                .iter()
                .find(|d| &d.id == id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| format!("{id} (not connected)"))
        })
        .unwrap_or_else(|| default_label.to_string());
    let before = selected.clone();
    egui::ComboBox::from_id_salt(id)
        .selected_text(current)
        .width(280.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(selected, None, default_label);
            for device in devices {
                ui.selectable_value(selected, Some(device.id.clone()), &device.name);
            }
        });
    *selected != before
}

fn hotkey_status_text(status: &HotkeyStatus) -> String {
    match status {
        HotkeyStatus::Starting => "Connecting to the hotkey service...".into(),
        HotkeyStatus::Active { backend, failed } if failed.is_empty() => {
            format!("Hotkeys active ({backend}).")
        }
        HotkeyStatus::Active { backend, failed } => {
            let list: Vec<String> = failed
                .iter()
                .map(|(id, why)| format!("{id}: {why}"))
                .collect();
            format!(
                "Hotkeys active ({backend}), but some could not be registered: {}",
                list.join("; ")
            )
        }
        HotkeyStatus::Unavailable(why) => why.clone(),
    }
}

impl SettingsWindow {
    pub fn show(
        &mut self,
        ctx: &Context,
        snapshot: &Snapshot,
        hotkeys: &HotkeyStatus,
        remote: &RemoteManager,
        commands: &mut Vec<Command>,
    ) {
        let mut settings: Settings = snapshot.library.settings.clone();
        let mut open = self.open;
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .default_width(460.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Audio");
                    egui::Grid::new("audio-settings").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                        ui.label("Monitor output");
                        device_combo(ui, "monitor", "System default", &snapshot.devices.outputs, &mut settings.monitor_device);
                        ui.end_row();
                        ui.label("Microphone");
                        device_combo(ui, "mic", "System default", &snapshot.devices.inputs, &mut settings.mic_device);
                        ui.end_row();
                        if cfg!(windows) {
                            ui.label("Virtual cable");
                            device_combo(ui, "cable", "Find automatically", &snapshot.devices.outputs, &mut settings.cable_device);
                            ui.end_row();
                        }
                        ui.label("New sounds play to");
                        egui::ComboBox::from_id_salt("default-route")
                            .selected_text(match settings.default_route {
                                Route::Both => "Voice chat and me",
                                Route::Mic => "Voice chat only",
                                Route::Monitor => "Only me",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut settings.default_route, Route::Both, "Voice chat and me");
                                ui.selectable_value(&mut settings.default_route, Route::Mic, "Voice chat only");
                                ui.selectable_value(&mut settings.default_route, Route::Monitor, "Only me");
                            });
                        ui.end_row();
                        ui.label("Loudness target");
                        ui.add(egui::Slider::new(&mut settings.normalize_target_lufs, -30.0..=-10.0).suffix(" LUFS"));
                        ui.end_row();
                    });
                    ui.label(RichText::new("In Discord or your game, choose \"Soundboard Mic\" (Linux) or \"CABLE Output\" (Windows) as the input device.").small().weak());

                    ui.add_space(12.0);
                    ui.heading("Hotkeys");
                    ui.label(hotkey_status_text(hotkeys));
                    egui::Grid::new("hotkey-settings").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                        ui.label("Stop all");
                        if let Some(key) = hotkey_button(ui, "stop-all-hotkey", &settings.stop_all_hotkey) {
                            settings.stop_all_hotkey = key;
                        }
                        ui.end_row();
                        ui.label("Toggle mic");
                        if let Some(key) = hotkey_button(ui, "passthrough-hotkey", &settings.passthrough_hotkey) {
                            settings.passthrough_hotkey = key;
                        }
                        ui.end_row();
                    });

                    ui.add_space(12.0);
                    ui.heading("Phone remote");
                    let remote_settings = &mut settings.remote;
                    if ui.checkbox(&mut remote_settings.enabled, "Allow playing sounds from other devices on this network").changed()
                        && remote_settings.enabled
                        && remote_settings.token.is_empty()
                    {
                        remote_settings.token = generate_token();
                    }
                    ui.horizontal(|ui| {
                        ui.label("Port");
                        ui.add(egui::DragValue::new(&mut remote_settings.port).range(1024..=65535));
                        if ui.button("New link").on_hover_text("Makes old links and QR codes stop working").clicked() {
                            remote_settings.token = generate_token();
                        }
                    });
                    if let Some(error) = &remote.error {
                        ui.colored_label(super::theme::ERROR, error);
                    }
                    if remote.is_running() {
                        let url = remote_url(&lan_ip().to_string(), remote_settings.port, &remote_settings.token);
                        ui.horizontal(|ui| {
                            if ui.button("Copy link").clicked() {
                                ui.ctx().copy_text(url.clone());
                            }
                            ui.label(RichText::new("Scan with your phone:").weak());
                        });
                        if let Some(texture) = self.qr.texture(ctx, &url) {
                            ui.image((texture.id(), vec2(180.0, 180.0)));
                        }
                    }

                    ui.add_space(12.0);
                    ui.heading("Window");
                    ui.checkbox(&mut settings.close_to_tray, "Closing the window keeps the soundboard running in the tray");
                    ui.checkbox(&mut settings.start_minimized, "Start hidden in the tray");
                });
            });
        self.open = open;
        if settings != snapshot.library.settings {
            commands.push(Command::UpdateSettings(Box::new(settings)));
        }
    }
}
