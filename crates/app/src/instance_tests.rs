use super::*;

#[test]
fn second_lock_in_the_same_directory_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let first = acquire(dir.path()).unwrap();
    assert!(first.is_some());
    assert!(acquire(dir.path()).unwrap().is_none());
    drop(first);
    assert!(acquire(dir.path()).unwrap().is_some());
}
