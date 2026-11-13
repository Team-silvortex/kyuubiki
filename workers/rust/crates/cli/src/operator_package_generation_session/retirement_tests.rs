use super::*;
use std::cell::RefCell;

thread_local! { static FAILURE: RefCell<Option<(&'static str, PathBuf)>> = const { RefCell::new(None) }; }

const STAGES: &[&str] = &[
    "record-published",
    "payload-entry-removed",
    "payload-removed",
    "lease-removed",
    "marker-removed",
    "directory-removed",
];

pub(super) fn checkpoint(_stage: &str, _root: &Path) -> Result<(), String> {
    if FAILURE.with_borrow(|fault| {
        fault
            .as_ref()
            .is_some_and(|(stage, root)| *stage == _stage && root == _root)
    }) {
        return Err(format!("injected retirement failure: {_stage}"));
    }
    #[cfg(unix)]
    {
        permissions::checkpoint(_stage, _root);
        crash::checkpoint(_stage, _root);
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
    cache: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-retirement-{label}-{}",
            next_session_id().unwrap()
        ));
        fs::create_dir_all(root.join("cache/packages")).unwrap();
        let cache = root.join("cache").canonicalize().unwrap();
        Self { root, cache }
    }

    fn stale(&self) -> PathBuf {
        let session = OperatorPackageGenerationSession::open(&self.cache).unwrap();
        let root = session.session_root.clone();
        fs::create_dir(session.generations_root.join("payload")).unwrap();
        fs::write(
            session.generations_root.join("payload/remaining"),
            b"recover me",
        )
        .unwrap();
        fs::write(session.generations_root.join("first-removable"), b"first").unwrap();
        drop(session);
        root
    }

    fn record(&self, root: &Path) -> PathBuf {
        self.cache
            .join(CACHE_GENERATIONS_DIRECTORY)
            .join(retirement::DIRECTORY)
            .join(format!(
                "{}.json",
                root.file_name().unwrap().to_str().unwrap()
            ))
    }

    fn recorded(&self) -> PathBuf {
        let stale = self.stale();
        let failure = Failure::at("record-published", &stale);
        let catalog = CatalogLease::acquire(&self.cache).unwrap();
        assert!(try_reap_session(&stale, &self.cache, &catalog).is_err());
        drop(failure);
        assert!(self.record(&stale).is_file());
        stale
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[cfg(unix)]
#[test]
#[ignore = "requires non-root Unix directory permission enforcement"]
fn denied_payload_cleanup_preserves_identity_and_retries_without_replay() {
    use std::os::unix::fs::PermissionsExt;
    assert_ne!(unsafe { libc::geteuid() }, 0);
    struct Restore(PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
        }
    }
    let fixture = Fixture::new("permission");
    let stale = fixture.stale();
    let protected = stale.join("generations/payload");
    let restore = Restore(protected.clone());
    let original = fs::read(stale.join(SESSION_MARKER_FILE)).unwrap();
    fs::set_permissions(&protected, fs::Permissions::from_mode(0o500)).unwrap();
    let replacement = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert_eq!(replacement.janitor_report().removed_stale_session_count, 0);
    assert_eq!(
        replacement.janitor_report().retained_invalid_session_count,
        1
    );
    assert_eq!(
        fs::read(protected.join("remaining")).unwrap(),
        b"recover me"
    );
    assert_eq!(fs::read(stale.join(SESSION_MARKER_FILE)).unwrap(), original);
    assert!(!stale.join("generations/first-removable").exists());
    assert!(stale.join(SESSION_LEASE_FILE).is_file());
    drop(restore);
    replacement.refresh_stale_sessions().unwrap();
    assert!(!stale.exists());
    assert_eq!(replacement.janitor_report().removed_stale_session_count, 1);
    assert_eq!(
        replacement.janitor_report().retained_invalid_session_count,
        0
    );
    replacement.refresh_stale_sessions().unwrap();
    assert_eq!(replacement.janitor_report().removed_stale_session_count, 1);
}

#[test]
fn retirement_failures_at_each_boundary_resume_without_double_count_or_replay() {
    for stage in STAGES {
        let fixture = Fixture::new(stage);
        let stale = fixture.stale();
        let original = fs::read(stale.join(SESSION_MARKER_FILE)).unwrap();
        let failure = Failure::at(stage, &stale);
        let replacement = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert_eq!(
            replacement.janitor_report().removed_stale_session_count,
            0,
            "{stage}"
        );
        assert_eq!(
            replacement.janitor_report().retained_invalid_session_count,
            1,
            "{stage}"
        );
        assert_eq!(fs::read(fixture.record(&stale)).unwrap(), original);
        assert!(replacement.session_root.exists());
        drop(failure);
        replacement.refresh_stale_sessions().unwrap();
        assert!(!stale.exists(), "{stage}");
        assert!(!fixture.record(&stale).exists(), "{stage}");
        let report = replacement.janitor_report();
        assert_eq!(report.removed_stale_session_count, 1, "{stage}");
        assert_eq!(report.retained_invalid_session_count, 0, "{stage}");
        replacement.refresh_stale_sessions().unwrap();
        assert_eq!(replacement.janitor_report(), report, "{stage}");
    }
}

#[cfg(unix)]
mod crash;
#[cfg(unix)]
mod permissions;
mod safety;
