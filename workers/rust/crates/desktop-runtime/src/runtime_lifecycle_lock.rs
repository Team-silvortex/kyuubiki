use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::Path;

pub(crate) struct RuntimeLifecycleLock(File);

impl RuntimeLifecycleLock {
    pub(crate) fn acquire(run: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(run)
            .map_err(|error| format!("failed to create runtime state: {error}"))?;
        let path = run.join("lifecycle.lock");
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let file = options
            .open(&path)
            .map_err(|error| format!("failed to open lifecycle lock: {error}"))?;
        file.try_lock_exclusive().map_err(|error| {
            format!("runtime lifecycle is busy or unavailable at {}; retry after the active start/stop/restart completes: {error}", path.display())
        })?;
        Ok(Self(file))
    }
}

impl Drop for RuntimeLifecycleLock {
    fn drop(&mut self) {
        // Keep the inode: unlinking a lock would let different callers lock different files.
        let _ = FileExt::unlock(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeLifecycleLock;

    #[test]
    fn lifecycle_mutations_are_exclusive_and_release_without_deleting_the_lock() {
        let run =
            std::env::temp_dir().join(format!("kyuubiki-lifecycle-lock-{}", std::process::id()));
        let lock = RuntimeLifecycleLock::acquire(&run).unwrap();
        assert!(
            RuntimeLifecycleLock::acquire(&run)
                .err()
                .expect("must be locked")
                .contains("lifecycle")
        );
        drop(lock);
        assert!(run.join("lifecycle.lock").is_file());
        drop(RuntimeLifecycleLock::acquire(&run).unwrap());
        std::fs::remove_dir_all(run).unwrap();
    }
}
