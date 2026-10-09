use super::theme;
use egui::{
    Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};
use soundboard_core::{ClipStatus, Command, Library, Snapshot, Sound, SoundId, TabId};
use std::collections::HashMap;

pub const BUTTON_SIZE: Vec2 = vec2(150.0, 76.0);
const GAP: f32 = 8.0;

/// Sounds to show: the search query spans every tab; otherwise the selected
/// tab, or every tab in order for the "All" view (`None`).
pub fn visible_sounds<'a>(library: &'a Library, tab: Option<TabId>, query: &str) -> Vec<&'a Sound> {
    let query = query.trim().to_lowercase();
    let in_order = library.tabs.iter().flat_map(|t| library.sounds_in(t.id));
    if !query.is_empty() {
        return in_order
            .filter(|s| s.name.to_lowercase().contains(&query))
            .collect();
    }
    match tab {
        Some(tab) => library.sounds_in(tab),
        None => in_order.collect(),
    }
}

pub enum GridAction {
    Command(Command),
    Edit(SoundId),
}

pub struct GridContext<'a> {
    pub snapshot: &'a Snapshot,
    pub progress: &'a HashMap<SoundId, f32>,
    pub triggers: &'a HashMap<String, String>,
}

fn status_of(snapshot: &Snapshot, id: SoundId) -> Option<&ClipStatus> {
    snapshot.clips.get(&id)
}

fn sound_button(ui: &mut Ui, sound: &Sound, ctx: &GridContext<'_>) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(BUTTON_SIZE, Sense::click_and_drag());
    let status = status_of(ctx.snapshot, sound.id);
    let progress = ctx.progress.get(&sound.id).copied();
    let base = Color32::from_rgb(sound.color[0], sound.color[1], sound.color[2]);
    let fill = match (status, response.hovered()) {
        (Some(ClipStatus::Failed(_)), _) => theme::FAILED,
        (_, true) => theme::lighten(base, 0.12),
        _ => base,
    };
    let painter = ui.painter_at(rect);
    let radius = CornerRadius::same(10);
    painter.rect_filled(rect, radius, fill);
    if let Some(progress) = progress {
        let filled = Rect::from_min_size(rect.min, vec2(rect.width() * progress, rect.height()));
        painter.rect_filled(filled, radius, Color32::from_white_alpha(40));
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(2.0, Color32::WHITE),
            StrokeKind::Inside,
        );
    }

    let text_color = theme::text_on(fill);
    let galley = ui.painter().layout(
        sound.name.clone(),
        FontId::proportional(15.0),
        text_color,
        rect.width() - 16.0,
    );
    let text_pos = pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0 - 4.0,
    );
    painter.galley(text_pos, galley, text_color);

    let key = super::hotkey_label(sound, ctx.triggers);
    if let Some(key) = key {
        painter.text(
            pos2(rect.max.x - 8.0, rect.max.y - 6.0),
            Align2::RIGHT_BOTTOM,
            key,
            FontId::monospace(11.0),
            text_color.gamma_multiply(0.8),
        );
    }
    if matches!(status, Some(ClipStatus::Loading)) {
        let spinner =
            Rect::from_center_size(pos2(rect.min.x + 14.0, rect.max.y - 14.0), vec2(14.0, 14.0));
        ui.put(spinner, egui::Spinner::new().size(14.0).color(text_color));
    }
    match status {
        Some(ClipStatus::Failed(err)) => response.on_hover_text(format!("Could not load: {err}")),
        Some(ClipStatus::Loading) => response.on_hover_text("Loading..."),
        _ => response,
    }
}

fn context_menu(
    response: &egui::Response,
    sound: &Sound,
    library: &Library,
    actions: &mut Vec<GridAction>,
) {
    response.context_menu(|ui| {
        if ui.button("Edit...").clicked() {
            actions.push(GridAction::Edit(sound.id));
            ui.close();
        }
        if ui.button("Stop").clicked() {
            actions.push(GridAction::Command(Command::Stop(sound.id)));
            ui.close();
        }
        ui.menu_button("Move to tab", |ui| {
            for tab in &library.tabs {
                let enabled = tab.id != sound.tab;
                if ui
                    .add_enabled(enabled, egui::Button::new(&tab.name))
                    .clicked()
                {
                    actions.push(GridAction::Command(Command::MoveSound {
                        sound: sound.id,
                        tab: tab.id,
                        index: usize::MAX,
                    }));
                    ui.close();
                }
            }
        });
        ui.separator();
        if ui.button("Delete").clicked() {
            actions.push(GridAction::Command(Command::DeleteSound(sound.id)));
            ui.close();
        }
    });
}

/// Draws the sound grid and returns what the user asked for.
pub fn show(
    ui: &mut Ui,
    sounds: &[&Sound],
    ctx: &GridContext<'_>,
    reorderable: bool,
) -> Vec<GridAction> {
    let mut actions = Vec::new();
    let library = &ctx.snapshot.library;
    let columns = ((ui.available_width() + GAP) / (BUTTON_SIZE.x + GAP))
        .floor()
        .max(1.0) as usize;
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(GAP, GAP);
            for row in sounds.chunks(columns) {
                ui.horizontal(|ui| {
                    for sound in row {
                        let response = ui
                            .push_id(sound.id, |ui| sound_button(ui, sound, ctx))
                            .inner;
                        if response.clicked() {
                            actions.push(GridAction::Command(Command::Play(sound.id)));
                        }
                        if response.double_clicked() {
                            actions.push(GridAction::Edit(sound.id));
                        }
                        if reorderable && response.drag_started() {
                            response.dnd_set_drag_payload(sound.id);
                        }
                        if reorderable
                            && let Some(dragged) = response.dnd_release_payload::<SoundId>()
                            && *dragged != sound.id
                        {
                            let index = library
                                .tab(sound.tab)
                                .and_then(|t| t.order.iter().position(|id| *id == sound.id))
                                .unwrap_or(usize::MAX);
                            actions.push(GridAction::Command(Command::MoveSound {
                                sound: *dragged,
                                tab: sound.tab,
                                index,
                            }));
                        }
                        if reorderable && response.dnd_hover_payload::<SoundId>().is_some() {
                            ui.painter().rect_stroke(
                                response.rect.expand(2.0),
                                CornerRadius::same(12),
                                Stroke::new(2.0, ui.visuals().selection.stroke.color),
                                StrokeKind::Outside,
                            );
                        }
                        context_menu(&response, sound, library, &mut actions);
                    }
                });
            }
        });
    actions
}

#[cfg(test)]
#[path = "grid_tests.rs"]
mod tests;
