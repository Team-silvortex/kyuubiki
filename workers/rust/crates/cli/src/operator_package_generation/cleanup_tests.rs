use super::*;
use std::cell::RefCell;

thread_local! { static FAILURE: RefCell<Option<(&'static str, PathBuf)>> = const { RefCell::new(None) }; }

const STAGES: &[&str] = &[
    "record-published",
    "payload-entry-removed",
    "payload-removed",
    "marker-removed",
    "directory-removed",
];

pub(super) fn checkpoint(stage: &str, root: &Path) -> Result<(), String> {
    if FAILURE.with_borrow(|fault| {
        fault
            .as_ref()
            .is_some_and(|(selected, target)| *selected == stage && target == root)
    }) {
        return Err(format!("injected generation cleanup failure: {stage}"));
    }
    #[cfg(unix)]
    {
        crash::checkpoint(stage, root);
        permissions::checkpoint(stage, root);
    }
    Ok(())
}

struct Failure(Option<(&'static str, PathBuf)>);
impl Failure {
    fn at(stage: &'static str, root: &Path) -> Self {
        Self(FAILURE.replace(Some((stage, root.to_path_buf()))))
    }
}
impl Drop for Failure {
    fn drop(&mut self) {
        FAILURE.replace(self.0.take());
    }
}

struct Fixture {
    root: PathBuf,
    session: Arc<OperatorPackageGenerationSession>,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-generation-cleanup-{label}-{}",
            next_generation_id().unwrap()
        ));
        fs::create_dir_all(root.join("packages")).unwrap();
        let session = OperatorPackageGenerationSession::open(&root).unwrap();
        Self { root, session }
    }

    fn prepared(&self) -> PreparedOperatorPackageGeneration {
        prepare_fixture(&self.root, self.session.clone())
    }
}

fn prepare_fixture(
    root: &Path,
    session: Arc<OperatorPackageGenerationSession>,
) -> PreparedOperatorPackageGeneration {
    let prepared =
        prepare_operator_package_generation(session, &root.join("packages"), "operator.next")
            .unwrap();
    fs::write(prepared.store_root().join("first-removable"), b"first").unwrap();
    fs::create_dir(prepared.packages_root().join("locked")).unwrap();
    fs::write(prepared.packages_root().join("locked/remaining"), b"keep").unwrap();
    prepared
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn record(generation: &OwnedOperatorPackageGeneration) -> PathBuf {
    generation.generations_root.join(format!(
        ".kyuubiki-retired-{}.json",
        generation.marker.generation_id
    ))
}

#[test]
fn generation_cleanup_retries_each_record_payload_and_metadata_boundary() {
    for stage in STAGES {
        let fixture = Fixture::new(stage);
        let generation = fixture.prepared().commit();
        let root = &generation.generation_root;
        let marker = fs::read(root.join(GENERATION_MARKER_FILE)).unwrap();
        let failure = Failure::at(stage, root);
        assert!(
            remove_owned_operator_package_generation(&generation).is_err(),
            "{stage}"
        );
        assert_eq!(fs::read(record(&generation)).unwrap(), marker, "{stage}");
        if *stage == "payload-entry-removed" {
            assert!(!root.join("first-removable").exists());
            assert_eq!(
                fs::read(root.join("packages/locked/remaining")).unwrap(),
                b"keep"
            );
        }
        drop(failure);
        remove_owned_operator_package_generation(&generation).unwrap();
        assert!(!root.exists(), "{stage}");
        assert!(!record(&generation).exists(), "{stage}");
        remove_owned_operator_package_generation(&generation).unwrap();
    }
}

#[test]
fn abandoned_partial_generation_is_reclaimed_after_its_session_closes() {
    let fixture = Fixture::new("abandoned");
    let session = OperatorPackageGenerationSession::open(&fixture.root).unwrap();
    let prepared = prepare_operator_package_generation(
        session.clone(),
        &fixture.root.join("packages"),
        "operator.next",
    )
    .unwrap();
    fs::write(prepared.store_root().join("first-removable"), b"first").unwrap();
    fs::write(prepared.packages_root().join("remaining"), b"keep").unwrap();
    let root = prepared.store_root().to_path_buf();
    let journal = record(&prepared.owned);
    let failure = Failure::at("payload-entry-removed", &root);
    drop(prepared);
    assert!(journal.is_file());
    assert!(!root.join("first-removable").exists());
    fixture.session.refresh_stale_sessions().unwrap();
    assert!(root.exists(), "live session was reclaimed");
    let blocked = fixture
        .root
        .join("agent-runtime-generations/initializations")
        .join(format!("{}.json", session.session_id()));
    fs::write(&blocked, b"defer queued cleanup until close").unwrap();
    let weak = Arc::downgrade(&session);
    drop(failure);
    drop(session);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while weak.upgrade().is_some() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    fs::remove_file(blocked).unwrap();
    while root.exists() {
        assert!(std::time::Instant::now() < deadline);
        fixture.session.refresh_stale_sessions().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!root.exists());
    assert!(!journal.exists());
    assert_eq!(
        fixture.session.janitor_report().removed_stale_session_count,
        1
    );
    fixture.session.refresh_stale_sessions().unwrap();
    assert_eq!(
        fixture.session.janitor_report().removed_stale_session_count,
        1
    );
}

#[test]
fn abandoned_partial_generation_is_retried_before_next_preparation() {
    for stage in STAGES {
        let fixture = Fixture::new(stage);
        let active = fixture.prepared().commit();
        let prepared = fixture.prepared();
        let root = prepared.store_root().to_path_buf();
        let journal = record(&prepared.owned);
        let failure = Failure::at(stage, &root);
        drop(prepared);
        assert!(journal.is_file());
        drop(failure);
        let next = fixture.prepared();
        assert!(!root.exists(), "abandoned payload still exists at {stage}");
        assert!(
            !journal.exists(),
            "abandoned record still exists at {stage}"
        );
        assert!(next.packages_root().is_dir());
        assert_eq!(
            fs::read(active.generation_root.join("packages/locked/remaining")).unwrap(),
            b"keep"
        );
    }
}

#[cfg(unix)]
#[test]
#[ignore = "requires non-root Unix directory permission enforcement"]
fn partial_generation_cleanup_preserves_identity_and_retries_after_permission_repair() {
    use std::os::unix::fs::PermissionsExt;
    assert_ne!(unsafe { libc::geteuid() }, 0);
    struct Restore(PathBuf, fs::Permissions);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, self.1.clone());
        }
    }
    let fixture = Fixture::new("permission");
    let generation = fixture.prepared().commit();
    let root = &generation.generation_root;
    let marker = fs::read(root.join(GENERATION_MARKER_FILE)).unwrap();
    let locked = root.join("packages/locked");
    let restore = Restore(locked.clone(), fs::metadata(&locked).unwrap().permissions());
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o500)).unwrap();
    assert!(remove_owned_operator_package_generation(&generation).is_err());
    assert_eq!(fs::read(root.join(GENERATION_MARKER_FILE)).unwrap(), marker);
    assert!(
        record(&generation).is_file(),
        "partial cleanup lost its recovery identity"
    );
    assert_eq!(fs::read(locked.join("remaining")).unwrap(), b"keep");
    assert!(!root.join("first-removable").exists());
    drop(restore);
    remove_owned_operator_package_generation(&generation).unwrap();
    assert!(!root.exists());
    assert!(!record(&generation).exists());
    remove_owned_operator_package_generation(&generation).unwrap();
}

#[cfg(unix)]
mod crash;
#[cfg(unix)]
mod permissions;
mod safety;
