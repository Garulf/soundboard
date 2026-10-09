const CABLE_PATTERNS: &[&str] = &["cable input", "virtual speaker", "virtual audio driver"];

/// Index of the first output device that looks like a virtual audio cable's
/// playback side (what we write the mic bus into).
pub fn find_cable(output_names: &[&str]) -> Option<usize> {
    output_names.iter().position(|name| {
        let name = name.to_lowercase();
        !name.contains("cable output") && CABLE_PATTERNS.iter().any(|p| name.contains(p))
    })
}

/// Converts interleaved device frames with `channels` channels to stereo,
/// duplicating mono and keeping the first two channels of wider layouts.
pub fn push_stereo(input: &[f32], channels: usize, mut push: impl FnMut(f32)) {
    let channels = channels.max(1);
    for frame in input.chunks_exact(channels) {
        let left = frame[0];
        let right = if channels > 1 { frame[1] } else { left };
        push(left);
        push(right);
    }
}

/// Writes stereo frames into a device buffer with `channels` channels.
pub fn write_frames(out: &mut [f32], channels: usize, stereo: &[f32]) {
    let channels = channels.max(1);
    for (frame, lr) in out
        .chunks_exact_mut(channels)
        .zip(stereo.as_chunks::<2>().0)
    {
        if channels == 1 {
            frame[0] = (lr[0] + lr[1]) * 0.5;
            continue;
        }
        frame[0] = lr[0];
        frame[1] = lr[1];
        frame[2..].fill(0.0);
    }
}

#[cfg(test)]
#[path = "cable_tests.rs"]
mod tests;
