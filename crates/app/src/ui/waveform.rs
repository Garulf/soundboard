use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use soundboard_core::Clip;

pub fn ms_to_x(ms: u64, duration_ms: u64, min_x: f32, width: f32) -> f32 {
    if duration_ms == 0 {
        return min_x;
    }
    min_x + width * (ms.min(duration_ms) as f32 / duration_ms as f32)
}

pub fn x_to_ms(x: f32, duration_ms: u64, min_x: f32, width: f32) -> u64 {
    if duration_ms == 0 || width <= 0.0 {
        return 0;
    }
    let t = ((x - min_x) / width).clamp(0.0, 1.0);
    (t as f64 * duration_ms as f64).round() as u64
}

const HANDLE_GRAB: f32 = 8.0;

/// Draws the clip's waveform with draggable trim handles. Returns true if a
/// handle moved.
pub fn trim_editor(
    ui: &mut Ui,
    clip: &Clip,
    peaks: &[f32],
    trim_start_ms: &mut u64,
    trim_end_ms: &mut Option<u64>,
) -> bool {
    let duration = clip.duration_ms();
    let size = Vec2::new(ui.available_width(), 96.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    painter.rect_filled(rect, 4.0, visuals.extreme_bg_color);

    let end_ms = trim_end_ms.unwrap_or(duration).min(duration);
    let start_x = ms_to_x(*trim_start_ms, duration, rect.min.x, rect.width());
    let end_x = ms_to_x(end_ms, duration, rect.min.x, rect.width());

    let mid = rect.center().y;
    let half = rect.height() / 2.0 - 4.0;
    let bar_width = rect.width() / peaks.len().max(1) as f32;
    for (i, peak) in peaks.iter().enumerate() {
        let x = rect.min.x + (i as f32 + 0.5) * bar_width;
        let active = x >= start_x && x <= end_x;
        let color = if active {
            visuals.selection.bg_fill
        } else {
            visuals.weak_text_color()
        };
        let h = (peak.min(1.0) * half).max(0.5);
        painter.line_segment(
            [Pos2::new(x, mid - h), Pos2::new(x, mid + h)],
            Stroke::new(bar_width.max(1.0), color),
        );
    }
    let dim = Color32::from_black_alpha(110);
    painter.rect_filled(
        Rect::from_x_y_ranges(rect.min.x..=start_x, rect.y_range()),
        0.0,
        dim,
    );
    painter.rect_filled(
        Rect::from_x_y_ranges(end_x..=rect.max.x, rect.y_range()),
        0.0,
        dim,
    );
    for x in [start_x, end_x] {
        painter.line_segment(
            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
            Stroke::new(2.0, Color32::WHITE),
        );
    }
    painter.rect_stroke(
        rect,
        4.0,
        visuals.widgets.noninteractive.bg_stroke,
        StrokeKind::Inside,
    );

    let drag_id = response.id.with("handle");
    if response.drag_started()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let grab_start = (pos.x - start_x).abs() <= (pos.x - end_x).abs();
        ui.data_mut(|d| d.insert_temp(drag_id, grab_start));
    }
    let mut changed = false;
    if response.dragged()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let grab_start = ui
            .data(|d| d.get_temp::<bool>(drag_id))
            .unwrap_or((pos.x - start_x).abs() < HANDLE_GRAB);
        let ms = x_to_ms(pos.x, duration, rect.min.x, rect.width());
        if grab_start {
            *trim_start_ms = ms.min(end_ms.saturating_sub(10));
        } else {
            let end = ms.max(*trim_start_ms + 10).min(duration);
            *trim_end_ms = (end < duration).then_some(end);
        }
        changed = true;
    }
    changed
}

#[cfg(test)]
#[path = "waveform_tests.rs"]
mod tests;
