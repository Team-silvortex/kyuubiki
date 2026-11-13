//! Stable lock inode: never unlink it while other Agent sessions may open the cache.
use super::{is_lock_contention, reject_symlink};
use fs2::FileExt;
use std::{
    fs::{File, OpenOptions},
    path::Path,
    time::{Duration, Instant},
};

pub(super) const LOCK_FILE: &str = "agent-runtime-generations.lock";
pub(super) const WAIT_MS: u64 = 2000;

pub(super) struct CatalogLease(File);

impl CatalogLease {
    pub(super) fn acquire(cache: &Path) -> Result<Self, String> {
        Self::acquire_until(cache, Instant::now() + Duration::from_millis(WAIT_MS))
    }

    fn acquire_until(cache: &Path, deadline: Instant) -> Result<Self, String> {
        let path = cache.join(LOCK_FILE);
        reject_symlink(&path, "operator session catalog lock")?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("cannot open operator session catalog lock: {error}"))?;
        if !file
            .metadata()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            return Err("operator session catalog lock must be a regular file".into());
        }
        loop {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(Self(file)),
                Err(error) if is_lock_contention(&error) => {
                    let now = Instant::now();
                    if now >= deadline {
                        return Err("operator session catalog busy; retry explicitly".into());
                    }
                    std::thread::sleep(Duration::from_millis(5).min(deadline - now));
                }
                Err(error) => return Err(format!("cannot lock operator session catalog: {error}")),
            }
        }
    }
}

impl Drop for CatalogLease {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_wait_is_bounded_and_never_replaces_the_lock_file() {
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-catalog-lock-{}-{}",
            std::process::id(),
            super::super::next_session_id().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        let owner = CatalogLease::acquire(&root).unwrap();
        assert!(
            CatalogLease::acquire_until(&root, Instant::now() + Duration::from_millis(10)).is_err()
        );
        drop(owner);
        assert!(root.join(LOCK_FILE).is_file());
        let next = CatalogLease::acquire(&root).unwrap();
        drop(next);
        std::fs::remove_dir_all(root).unwrap();
    }
}
