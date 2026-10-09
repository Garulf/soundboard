use super::*;

#[test]
fn icon_has_rgba_pixels_for_the_requested_size() {
    let rgba = icon_rgba(32);
    assert_eq!(rgba.len(), 32 * 32 * 4);
    assert!(rgba.chunks(4).any(|p| p[3] == 255));
    assert!(rgba.chunks(4).any(|p| p[3] == 0));
}

#[test]
fn argb_conversion_reorders_channels() {
    assert_eq!(rgba_to_argb(&[1, 2, 3, 4]), [4, 1, 2, 3]);
}
