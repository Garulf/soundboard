use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;

pub struct InstanceLock(#[allow(dead_code)] File);

/// Takes the single-instance lock, or returns `None` if another copy of the
/// app holds it.
pub fn acquire(dir: &Path) -> io::Result<Option<InstanceLock>> {
    std::fs::create_dir_all(dir)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("soundboard.lock"))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(InstanceLock(file))),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(e)) => Err(e),
    }
}

#[cfg(test)]
#[path = "instance_tests.rs"]
mod tests;
