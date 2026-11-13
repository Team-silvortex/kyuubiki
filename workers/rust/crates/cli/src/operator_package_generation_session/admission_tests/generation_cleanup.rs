use super::*;
use crate::operator_package_generation::{
    PreparedOperatorPackageGeneration, prepare_operator_package_generation,
    remove_owned_operator_package_generation,
};

fn prepare(
    fixture: &Fixture,
    session: &Arc<OperatorPackageGenerationSession>,
) -> PreparedOperatorPackageGeneration {
    let prepared = prepare_operator_package_generation(
        session.clone(),
        &fixture.cache.join("packages"),
        "operator.next",
    )
    .unwrap();
    fs::write(prepared.packages_root().join("payload"), b"keep payload").unwrap();
    prepared
}

#[test]
fn retired_generation_cleanup_preserves_unresolved_session_records() {
    for kind in ["initialization", "staging", "retirement"] {
        let fixture = Fixture::new();
        let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        let prepared = prepare(&fixture, &owner);
        let root = prepared.store_root().to_path_buf();
        let generation = prepared.commit();
        let record = fixture.block(&owner, kind);
        assert!(
            remove_owned_operator_package_generation(&generation).is_err(),
            "generation cleanup bypassed {kind}"
        );
        assert_eq!(
            fs::read(root.join("packages/payload")).unwrap(),
            b"keep payload"
        );
        assert_eq!(fs::read(&record).unwrap(), b"unresolved record");

        let peer = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        let peer_prepared = prepare(&fixture, &peer);
        let peer_root = peer_prepared.store_root().to_path_buf();
        let peer_generation = peer_prepared.commit();
        remove_owned_operator_package_generation(&peer_generation).unwrap();
        assert!(
            !peer_root.exists(),
            "healthy peer cleanup was blocked by {kind}"
        );
        assert!(root.exists());

        fs::remove_file(record).unwrap();
        remove_owned_operator_package_generation(&generation).unwrap();
        assert!(!root.exists());
        remove_owned_operator_package_generation(&generation).unwrap();
    }
}

#[test]
fn abandoned_generation_keeps_ambiguous_session_for_restart_cleanup() {
    for kind in ["initialization", "staging", "retirement"] {
        let fixture = Fixture::new();
        let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        let prepared = prepare(&fixture, &owner);
        let root = prepared.store_root().to_path_buf();
        let session_root = owner.session_root.clone();
        let record = fixture.block(&owner, kind);
        drop(prepared);
        assert!(root.exists(), "abandoned generation bypassed {kind}");
        assert_eq!(
            fs::read(root.join("packages/payload")).unwrap(),
            b"keep payload"
        );
        let weak = Arc::downgrade(&owner);
        drop(owner);
        let peer = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert_eq!(peer.janitor_report().removed_stale_session_count, 0);
        assert!(root.exists());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while weak.upgrade().is_some() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        fs::remove_file(record).unwrap();
        while session_root.exists() {
            assert!(std::time::Instant::now() < deadline);
            peer.refresh_stale_sessions().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!session_root.exists());
        assert_eq!(peer.janitor_report().removed_stale_session_count, 1);
        peer.refresh_stale_sessions().unwrap();
        assert_eq!(peer.janitor_report().removed_stale_session_count, 1);
    }
}

#[test]
fn generation_cleanup_requires_unchanged_session_identity() {
    let fixture = Fixture::new();
    let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    let prepared = prepare(&fixture, &owner);
    let root = prepared.store_root().to_path_buf();
    let generation = prepared.commit();
    let path = owner.session_root.join(SESSION_MARKER_FILE);
    let original = fs::read(&path).unwrap();
    let mut changed = owner.marker.clone();
    changed.session_id.push_str("-foreign");
    fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(remove_owned_operator_package_generation(&generation).is_err());
    assert_eq!(
        fs::read(root.join("packages/payload")).unwrap(),
        b"keep payload"
    );
    fs::write(path, original).unwrap();
    remove_owned_operator_package_generation(&generation).unwrap();
    assert!(!root.exists());
}

#[test]
fn generation_cleanup_does_not_bypass_busy_catalog() {
    let fixture = Fixture::new();
    let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    let prepared = prepare(&fixture, &owner);
    let root = prepared.store_root().to_path_buf();
    let generation = prepared.commit();
    let catalog = CatalogLease::acquire(&fixture.cache).unwrap();
    let result = remove_owned_operator_package_generation(&generation);
    drop(catalog);
    let error = result.expect_err("cleanup must not bypass catalog coordination");
    assert!(error.contains("catalog busy"), "{error}");
    assert_eq!(
        fs::read(root.join("packages/payload")).unwrap(),
        b"keep payload"
    );
    remove_owned_operator_package_generation(&generation).unwrap();
    assert!(!root.exists());
}

#[test]
fn generation_cleanup_holds_catalog_until_the_operation_finishes() {
    let fixture = Fixture::new();
    let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    let probe = OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.cache.join(coordination::LOCK_FILE))
        .unwrap();
    let error = owner
        .with_generation_cleanup(|| {
            assert!(is_lock_contention(&probe.try_lock_exclusive().unwrap_err()));
            Err("cleanup failed after admission".into())
        })
        .unwrap_err();
    assert_eq!(error, "cleanup failed after admission");
    probe.try_lock_exclusive().unwrap();
    FileExt::unlock(&probe).unwrap();
    owner.with_generation_cleanup(|| Ok(())).unwrap();
}

fn replaced_namespace(replace: impl Fn(&Path, &Path)) {
    for directory in [initialization::DIRECTORY, retirement::DIRECTORY] {
        let fixture = Fixture::new();
        let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        let prepared = prepare(&fixture, &owner);
        let root = prepared.store_root().to_path_buf();
        let generation = prepared.commit();
        let path = fixture
            .cache
            .join(CACHE_GENERATIONS_DIRECTORY)
            .join(directory);
        let saved = path.with_extension("saved");
        fs::rename(&path, &saved).unwrap();
        replace(&path, &fixture.root);
        let result = remove_owned_operator_package_generation(&generation);
        fs::remove_file(&path).unwrap();
        fs::rename(saved, path).unwrap();
        assert!(result.is_err(), "cleanup accepted substituted {directory}");
        assert_eq!(
            fs::read(root.join("packages/payload")).unwrap(),
            b"keep payload"
        );
        remove_owned_operator_package_generation(&generation).unwrap();
        assert!(!root.exists());
        if fixture.root.join("outside").exists() {
            assert_eq!(
                fs::read(fixture.root.join("outside/keep")).unwrap(),
                b"outside"
            );
        }
    }
}

#[test]
fn generation_cleanup_rejects_file_record_namespaces() {
    replaced_namespace(|path, _| fs::write(path, b"not a record directory").unwrap());
}

#[cfg(unix)]
#[test]
fn generation_cleanup_rejects_symlink_record_namespaces() {
    replaced_namespace(|path, root| {
        let outside = root.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep"), b"outside").unwrap();
        std::os::unix::fs::symlink(outside, path).unwrap();
    });
}

#[cfg(unix)]
#[test]
#[ignore = "requires real non-root Unix directory permission enforcement"]
fn generation_cleanup_requires_readable_initialization_records() {
    use std::os::unix::fs::PermissionsExt;
    assert_ne!(unsafe { libc::geteuid() }, 0);
    struct Restore(PathBuf, fs::Permissions);
    impl Drop for Restore {
        fn drop(&mut self) {
            fs::set_permissions(&self.0, self.1.clone()).unwrap();
        }
    }
    let fixture = Fixture::new();
    let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    let prepared = prepare(&fixture, &owner);
    let root = prepared.store_root().to_path_buf();
    let generation = prepared.commit();
    let directory = fixture
        .cache
        .join(CACHE_GENERATIONS_DIRECTORY)
        .join(initialization::DIRECTORY);
    let restore = Restore(
        directory.clone(),
        fs::metadata(&directory).unwrap().permissions(),
    );
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o100)).unwrap();
    assert!(remove_owned_operator_package_generation(&generation).is_err());
    assert_eq!(
        fs::read(root.join("packages/payload")).unwrap(),
        b"keep payload"
    );
    drop(restore);
    remove_owned_operator_package_generation(&generation).unwrap();
    assert!(!root.exists());
}
