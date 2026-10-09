use super::theme;
use egui::{Color32, Sense, Ui, vec2};
use soundboard_core::{BackendStatus, Bus, Command, Meters, Settings, Snapshot};

fn meter(ui: &mut Ui, level: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(60.0, 8.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, ui.visuals().extreme_bg_color);
    let level = level.clamp(0.0, 1.0);
    let color = if level > 0.89 {
        theme::ERROR
    } else if level > 0.6 {
        theme::WARN
    } else {
        theme::OK
    };
    let mut filled = rect;
    filled.set_width(rect.width() * level);
    painter.rect_filled(filled, 3.0, color);
}

fn volume(ui: &mut Ui, label: &str, value: &mut f32, hover: &str) -> bool {
    ui.label(label).on_hover_text(hover);
    ui.add(egui::Slider::new(value, 0.0..=1.5).show_value(false))
        .on_hover_text(format!("{:.0}%", *value * 100.0))
        .changed()
}

pub fn status_text(status: &BackendStatus) -> (Color32, String) {
    match status {
        BackendStatus::Starting => (theme::WARN, "Starting audio...".into()),
        BackendStatus::Ok => (theme::OK, "Soundboard Mic ready".into()),
        BackendStatus::CableMissing => (
            theme::WARN,
            "No virtual cable found. Install VB-Cable to send sounds to voice chat.".into(),
        ),
        BackendStatus::Error(e) => (theme::ERROR, format!("Audio problem: {e}")),
    }
}

pub fn show(ui: &mut Ui, snapshot: &Snapshot, meters: &Meters) -> Vec<Command> {
    let mut commands = Vec::new();
    let mut settings: Settings = snapshot.library.settings.clone();
    ui.horizontal(|ui| {
        let stop = egui::Button::new(
            egui::RichText::new("Stop all")
                .strong()
                .color(Color32::WHITE),
        )
        .fill(theme::ACCENT);
        if ui.add(stop).clicked() {
            commands.push(Command::StopAll);
        }
        ui.separator();
        let mut changed = volume(
            ui,
            "Voice",
            &mut settings.mic_bus_volume,
            "Volume of sounds sent to the virtual mic",
        );
        meter(ui, meters.peak(Bus::Mic));
        ui.separator();
        changed |= volume(
            ui,
            "You",
            &mut settings.monitor_bus_volume,
            "Volume of sounds you hear",
        );
        meter(ui, meters.peak(Bus::Monitor));
        ui.separator();
        let mut passthrough = settings.passthrough;
        if ui
            .checkbox(&mut passthrough, "Mic")
            .on_hover_text("Mix your real microphone into the virtual mic")
            .changed()
        {
            commands.push(Command::TogglePassthrough);
        }
        ui.add_enabled_ui(settings.passthrough, |ui| {
            changed |= ui
                .add(egui::Slider::new(&mut settings.mic_volume, 0.0..=2.0).show_value(false))
                .on_hover_text(format!("Microphone {:.0}%", settings.mic_volume * 100.0))
                .changed();
        });
        if changed {
            commands.push(Command::UpdateSettings(Box::new(settings.clone())));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (color, text) = status_text(&snapshot.backend);
            let (rect, response) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 5.0, color);
            response.on_hover_text(&text);
            ui.add(egui::Label::new(egui::RichText::new(text).small().weak()).truncate());
        });
    });
    commands
}
