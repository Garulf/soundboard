use super::*;

#[test]
fn live_stream_states_mean_ready() {
    assert_eq!(stream_status(&StreamState::Paused), Some(BackendStatus::Ok));
    assert_eq!(
        stream_status(&StreamState::Streaming),
        Some(BackendStatus::Ok)
    );
}

#[test]
fn stream_errors_are_reported() {
    assert_eq!(
        stream_status(&StreamState::Error("boom".into())),
        Some(BackendStatus::Error("boom".into()))
    );
}

#[test]
fn transitional_states_report_nothing() {
    assert_eq!(stream_status(&StreamState::Connecting), None);
    assert_eq!(stream_status(&StreamState::Unconnected), None);
}
