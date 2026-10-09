use super::*;
use std::time::Duration;

#[test]
fn healthy_setup_never_rebuilds() {
    assert!(!should_rebuild(false, false, Duration::from_secs(60), true));
}

#[test]
fn stream_errors_rebuild_after_the_retry_delay() {
    assert!(!should_rebuild(
        true,
        false,
        Duration::from_millis(500),
        false
    ));
    assert!(should_rebuild(true, false, RETRY_DELAY, false));
}

#[test]
fn incomplete_setup_rebuilds_when_devices_change() {
    assert!(!should_rebuild(false, true, Duration::from_secs(60), false));
    assert!(should_rebuild(false, true, Duration::ZERO, true));
}
