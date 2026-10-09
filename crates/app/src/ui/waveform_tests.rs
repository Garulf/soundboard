use super::*;

#[test]
fn ms_maps_linearly_onto_the_rect() {
    assert_eq!(ms_to_x(0, 1000, 10.0, 200.0), 10.0);
    assert_eq!(ms_to_x(500, 1000, 10.0, 200.0), 110.0);
    assert_eq!(ms_to_x(1000, 1000, 10.0, 200.0), 210.0);
}

#[test]
fn x_maps_back_to_ms_and_clamps() {
    assert_eq!(x_to_ms(110.0, 1000, 10.0, 200.0), 500);
    assert_eq!(x_to_ms(-50.0, 1000, 10.0, 200.0), 0);
    assert_eq!(x_to_ms(999.0, 1000, 10.0, 200.0), 1000);
}

#[test]
fn zero_duration_is_safe() {
    assert_eq!(ms_to_x(10, 0, 10.0, 200.0), 10.0);
    assert_eq!(x_to_ms(100.0, 0, 10.0, 200.0), 0);
}
