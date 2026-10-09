use egui::{Color32, Context, Visuals};

pub const ACCENT: Color32 = Color32::from_rgb(229, 72, 77);
pub const FAILED: Color32 = Color32::from_rgb(90, 40, 44);
pub const OK: Color32 = Color32::from_rgb(70, 190, 110);
pub const WARN: Color32 = Color32::from_rgb(230, 170, 60);
pub const ERROR: Color32 = Color32::from_rgb(229, 72, 77);

pub fn apply(ctx: &Context) {
    let mut visuals = Visuals::dark();
    visuals.selection.bg_fill = ACCENT;
    visuals.hyperlink_color = ACCENT;
    visuals.panel_fill = Color32::from_rgb(22, 24, 29);
    visuals.window_fill = Color32::from_rgb(30, 33, 40);
    visuals.extreme_bg_color = Color32::from_rgb(16, 17, 21);
    ctx.set_visuals(visuals);
}

pub fn lighten(color: Color32, amount: f32) -> Color32 {
    let mix = |c: u8| (c as f32 + (255.0 - c as f32) * amount) as u8;
    Color32::from_rgb(mix(color.r()), mix(color.g()), mix(color.b()))
}

/// Black or white, whichever reads better on `fill`.
pub fn text_on(fill: Color32) -> Color32 {
    let luminance = 0.2126 * fill.r() as f32 + 0.7152 * fill.g() as f32 + 0.0722 * fill.b() as f32;
    if luminance > 150.0 {
        Color32::from_rgb(20, 20, 24)
    } else {
        Color32::WHITE
    }
}
