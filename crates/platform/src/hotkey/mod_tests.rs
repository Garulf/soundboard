use super::*;

fn binding(id: &str, acc: &str) -> Binding {
    Binding {
        id: id.into(),
        description: id.into(),
        accelerator: acc.parse().unwrap(),
    }
}

#[test]
fn duplicates_after_the_first_are_split_off() {
    let (unique, failed) = split_duplicates(vec![
        binding("a", "Ctrl+1"),
        binding("b", "Ctrl+1"),
        binding("c", "Ctrl+2"),
    ]);
    let ids: Vec<_> = unique.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(ids, ["a", "c"]);
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].0, "b");
}

#[test]
fn a_rejected_bind_lists_each_binding_as_failed() {
    let status = bind_failure_status(
        "portal",
        &[binding("a", "Ctrl+1")],
        vec![("b".into(), "duplicate".into())],
        "cancelled by user",
    );
    match status {
        HotkeyStatus::Active { backend, failed } => {
            assert_eq!(backend, "portal");
            assert_eq!(failed.len(), 2);
            assert_eq!(failed[0], ("b".to_string(), "duplicate".to_string()));
            assert_eq!(failed[1].0, "a");
            assert!(failed[1].1.contains("cancelled by user"));
        }
        other => panic!("unexpected {other:?}"),
    }
}
