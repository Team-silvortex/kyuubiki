use super::*;
use std::{cell::RefCell, os::unix::fs::PermissionsExt};

thread_local! { static FAULT: RefCell<Option<(&'static str, PathBuf)>> = const { RefCell::new(None) }; }

pub(super) fn checkpoint(stage: &str) {
    FAULT.with_borrow(|fault| {
        if let Some((expected, path)) = fault
            && *expected == stage
        {
            fs::set_permissions(path, fs::Permissions::from_mode(0o500)).unwrap();
        }
    });
}

struct DeniedDirectory {
    path: PathBuf,
    permissions: fs::Permissions,
    previous: Option<(&'static str, PathBuf)>,
}

impl DeniedDirectory {
    fn at(path: &Path, phase: &'static str) -> Self {
        let original = fs::metadata(path).unwrap().permissions();
        let previous = FAULT.replace(Some((phase, path.to_path_buf())));
        Self {
            path: path.to_path_buf(),
            permissions: original,
            previous,
        }
    }

    fn now(path: &Path, mode: u32) -> Self {
        let guard = Self::at(path, "never");
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
        guard
    }
}

impl Drop for DeniedDirectory {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.path, self.permissions.clone());
        FAULT.replace(self.previous.take());
    }
}

#[test]
#[ignore = "requires real non-root Unix permission enforcement; changes only owned fixture directories"]
fn real_permission_failures_preserve_records_and_allow_explicit_retry() {
    assert_ne!(
        unsafe { libc::geteuid() },
        0,
        "run this qualification as a non-root account"
    );
    failed_inspection();
    for phase in [
        "intent-staging-synced",
        "intent-published",
        "generations-created",
    ] {
        publication_or_commit_failure(phase);
    }
    incomplete_cleanup_failure();
}

fn failed_inspection() {
    let fixture = Fixture::new("permission-inspection");
    let catalog = CatalogLease::acquire(&fixture.cache).unwrap();
    let denied = DeniedDirectory::now(&fixture.sessions, 0o000);
    let error = begin(&fixture.sessions, &marker(&fixture), &catalog).unwrap_err();
    assert!(error.contains("inspect"), "{error}");
    assert!(!fixture.intents().exists());
    drop(denied);
}

fn publication_or_commit_failure(phase: &'static str) {
    let fixture = Fixture::new(phase);
    let peer = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    let peer_marker = fs::read(peer.session_root.join(SESSION_MARKER_FILE)).unwrap();
    let denied = DeniedDirectory::at(&fixture.intents(), phase);
    let error = OperatorPackageGenerationSession::open(&fixture.cache).unwrap_err();
    let expected = match phase {
        "intent-staging-synced" => "cannot publish initialization record",
        "intent-published" => "cannot retire initialization staging",
        _ => "failed to commit operator session initialization",
    };
    assert!(error.contains(expected), "{error}");
    let entries: Vec<_> = fs::read_dir(fixture.intents())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(entries.len(), usize::from(phase == "intent-published") + 1);
    let snapshots: Vec<_> = entries.iter().map(|p| (p, fs::read(p).unwrap())).collect();
    let catalog = CatalogLease::acquire(&fixture.cache).unwrap();
    let blocked = recover(&fixture.sessions, &fixture.cache, &catalog).unwrap();
    assert_eq!(blocked.report.removed_initialization_staging_count, 0);
    assert_eq!(blocked.report.removed_stale_session_count, 0);
    assert!(blocked.report.retained_invalid_session_count >= 1);
    for (path, bytes) in snapshots {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    assert_eq!(
        fs::read(peer.session_root.join(SESSION_MARKER_FILE)).unwrap(),
        peer_marker
    );
    drop(catalog);
    drop(denied);
    let replacement = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert_eq!(
        replacement.janitor_report().retained_invalid_session_count,
        0
    );
    assert_eq!(
        replacement
            .janitor_report()
            .removed_initialization_staging_count,
        usize::from(phase != "generations-created")
    );
    assert_eq!(
        replacement.janitor_report().removed_stale_session_count,
        usize::from(phase != "intent-staging-synced")
    );
    assert_eq!(fs::read_dir(fixture.intents()).unwrap().count(), 0);
    assert!(peer.session_root.exists());
    let report = replacement.janitor_report();
    replacement.refresh_stale_sessions().unwrap();
    assert_eq!(replacement.janitor_report(), report);
}

fn incomplete_cleanup_failure() {
    let fixture = Fixture::new("permission-cleanup");
    let stale = fixture.incomplete();
    let marker_before = fs::read(stale.join(SESSION_MARKER_FILE)).unwrap();
    let denied = DeniedDirectory::now(&stale, 0o500);
    let healthy = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert!(stale.exists());
    assert_eq!(healthy.janitor_report().retained_invalid_session_count, 1);
    assert_eq!(healthy.janitor_report().removed_stale_session_count, 0);
    assert_eq!(
        fs::read(stale.join(SESSION_MARKER_FILE)).unwrap(),
        marker_before
    );
    drop(denied);
    healthy.refresh_stale_sessions().unwrap();
    assert!(!stale.exists());
    assert_eq!(healthy.janitor_report().removed_stale_session_count, 1);
    assert_eq!(healthy.janitor_report().retained_invalid_session_count, 0);
}
