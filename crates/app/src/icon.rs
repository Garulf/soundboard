const BACKGROUND: [u8; 3] = [229, 72, 77];
const BAR: [u8; 3] = [255, 255, 255];
const BAR_HEIGHTS: [f32; 5] = [0.35, 0.65, 0.9, 0.55, 0.3];

fn inside_rounded_square(x: f32, y: f32, radius: f32) -> bool {
    let cx = x.clamp(radius, 1.0 - radius);
    let cy = y.clamp(radius, 1.0 - radius);
    (x - cx).powi(2) + (y - cy).powi(2) <= radius * radius
}

fn on_bar(x: f32, y: f32) -> bool {
    let slots = BAR_HEIGHTS.len() as f32 * 2.0 + 1.0;
    let slot = (x - 0.12) / 0.76 * slots;
    if !(0.0..slots).contains(&slot) || (slot as usize).is_multiple_of(2) {
        return false;
    }
    let height = BAR_HEIGHTS[slot as usize / 2] * 0.62;
    (y - 0.5).abs() <= height / 2.0
}

/// The app icon (a rounded red square with level bars) as straight RGBA.
pub fn icon_rgba(size: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for py in 0..size {
        for px in 0..size {
            let x = (px as f32 + 0.5) / size as f32;
            let y = (py as f32 + 0.5) / size as f32;
            let pixel = if !inside_rounded_square(x, y, 0.22) {
                [0, 0, 0, 0]
            } else if on_bar(x, y) {
                [BAR[0], BAR[1], BAR[2], 255]
            } else {
                [BACKGROUND[0], BACKGROUND[1], BACKGROUND[2], 255]
            };
            rgba.extend_from_slice(&pixel);
        }
    }
    rgba
}

#[cfg(any(target_os = "linux", test))]
pub fn rgba_to_argb(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[3], p[0], p[1], p[2]])
        .collect()
}

#[cfg(test)]
#[path = "icon_tests.rs"]
mod tests;
