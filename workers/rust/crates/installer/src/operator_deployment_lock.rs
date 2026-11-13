use fs2::FileExt;
use std::fs::File;

pub(super) struct DeploymentLock(File);

impl DeploymentLock {
    pub(super) fn acquire(file: File) -> Result<Self, String> {
        file.try_lock_exclusive().map_err(|_| "deployment_busy")?;
        Ok(Self(file))
    }
}

impl Drop for DeploymentLock {
    fn drop(&mut self) {
        // Closing alone can leave flock held by a duplicated/inherited description.
        let _ = FileExt::unlock(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "kyuubiki-deployment-lock-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn open(&self) -> File {
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(self.0.join("lock"))
                .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn deployment_lock_blocks_concurrent_upload_and_releases_on_error() {
        let f = Fixture::new();
        let operation = || -> Result<(), String> {
            let _guard = DeploymentLock::acquire(f.open())?;
            assert_eq!(
                DeploymentLock::acquire(f.open()).err().as_deref(),
                Some("deployment_busy")
            );
            Err("deployment_corrupt".into())
        };
        assert_eq!(operation().unwrap_err(), "deployment_corrupt");
        assert!(DeploymentLock::acquire(f.open()).is_ok());
    }
    #[cfg(unix)]
    #[test]
    fn completed_deployment_unlocks_while_inherited_description_survives() {
        let f = Fixture::new();
        let guard = DeploymentLock::acquire(f.open()).unwrap();
        let inherited = guard.0.try_clone().unwrap();
        drop(guard);
        assert!(
            DeploymentLock::acquire(f.open()).is_ok(),
            "a completed upload retained the lock through a duplicate descriptor"
        );
        drop(inherited);
    }
}
