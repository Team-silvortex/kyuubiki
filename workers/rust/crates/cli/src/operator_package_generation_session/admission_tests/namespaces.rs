use super::*;

struct ReplacedDirectory {
    path: PathBuf,
    saved: PathBuf,
}
impl ReplacedDirectory {
    fn move_aside(path: PathBuf) -> Self {
        let saved = path.with_extension("saved");
        fs::rename(&path, &saved).unwrap();
        Self { path, saved }
    }
}
impl Drop for ReplacedDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::rename(&self.saved, &self.path);
    }
}

fn exercise_replaced_directory(kind: &str, replace: impl FnOnce(&Path, &Path)) {
    let fixture = Fixture::new();
    let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    let root = owner.session_root.clone();
    let generation = owner.generations_root.join("generation");
    fs::create_dir(&generation).unwrap();
    let path = fixture.cache.join(CACHE_GENERATIONS_DIRECTORY).join(kind);
    let restored = ReplacedDirectory::move_aside(path.clone());
    replace(&path, &fixture.root);
    assert!(
        acquire_download_session_lease(&generation).is_err(),
        "download accepted unsafe {kind}"
    );
    assert!(owner.refresh_stale_sessions().is_err());
    fs::remove_dir(generation).unwrap();
    drop(owner);
    assert!(
        root.join(SESSION_MARKER_FILE).exists(),
        "close bypassed unsafe {kind}"
    );
    drop(restored);
    let healthy = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert!(!root.exists());
    assert_eq!(healthy.janitor_report().removed_stale_session_count, 1);
    if fixture.root.join("outside").exists() {
        assert_eq!(
            fs::read(fixture.root.join("outside/keep")).unwrap(),
            b"keep"
        );
    }
}

#[test]
fn lifecycle_record_namespace_files_block_admission_and_close_cleanup() {
    for directory in [initialization::DIRECTORY, retirement::DIRECTORY] {
        exercise_replaced_directory(directory, |path, _| {
            fs::write(path, b"keep namespace").unwrap()
        });
    }
}

#[cfg(unix)]
#[test]
fn lifecycle_record_namespace_symlinks_cannot_hide_admission_barriers() {
    for directory in [initialization::DIRECTORY, retirement::DIRECTORY] {
        exercise_replaced_directory(directory, |path, root| {
            let outside = root.join("outside");
            fs::create_dir(&outside).unwrap();
            fs::write(outside.join("keep"), b"keep").unwrap();
            std::os::unix::fs::symlink(&outside, path).unwrap();
        });
    }
}

#[cfg(unix)]
#[test]
#[ignore = "requires real non-root Unix directory permission enforcement"]
fn unreadable_initialization_records_retain_owner_until_explicit_retry() {
    use std::os::unix::fs::PermissionsExt;
    assert_ne!(unsafe { libc::geteuid() }, 0);
    struct Restore(PathBuf, fs::Permissions);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, self.1.clone());
        }
    }
    let fixture = Fixture::new();
    let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    let root = owner.session_root.clone();
    let generation = owner.generations_root.join("generation");
    fs::create_dir(&generation).unwrap();
    let directory = fixture
        .cache
        .join(CACHE_GENERATIONS_DIRECTORY)
        .join(initialization::DIRECTORY);
    let restore = Restore(
        directory.clone(),
        fs::metadata(&directory).unwrap().permissions(),
    );
    // Lookup succeeds, but enumeration needed to rule out pending staging is denied.
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o100)).unwrap();
    assert!(acquire_download_session_lease(&generation).is_err());
    assert!(owner.refresh_stale_sessions().is_err());
    fs::remove_dir(generation).unwrap();
    drop(owner);
    assert!(root.join(SESSION_MARKER_FILE).is_file());
    drop(restore);
    let healthy = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert!(!root.exists());
    assert_eq!(healthy.janitor_report().removed_stale_session_count, 1);
}
