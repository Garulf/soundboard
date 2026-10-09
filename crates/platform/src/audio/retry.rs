use std::time::Duration;

pub const RETRY_DELAY: Duration = Duration::from_secs(2);

/// Whether an audio backend should tear down and reopen its streams. A
/// stream that errored is retried after a delay; a setup that is missing a
/// device is retried when the device list changes, so working streams are
/// not interrupted on a timer.
pub fn should_rebuild(
    stream_failed: bool,
    setup_incomplete: bool,
    since_last_attempt: Duration,
    devices_changed: bool,
) -> bool {
    (stream_failed && since_last_attempt >= RETRY_DELAY) || (setup_incomplete && devices_changed)
}

#[cfg(test)]
#[path = "retry_tests.rs"]
mod tests;
