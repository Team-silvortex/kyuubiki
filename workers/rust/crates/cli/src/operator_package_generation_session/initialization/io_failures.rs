//! Test-only I/O failures; no production environment-variable controls.
use super::tests::Fixture;
use super::*;
use std::cell::Cell;

thread_local! { static FAIL_SYNC: Cell<bool> = const { Cell::new(false) }; }

pub(super) fn before_staging_sync() -> std::io::Result<()> {
    if FAIL_SYNC.get() {
        Err(std::io::Error::other(
            "injected initialization sync failure",
        ))
    } else {
        Ok(())
    }
}

struct SyncFailure(bool);
impl SyncFailure {
    fn new() -> Self {
        Self(FAIL_SYNC.replace(true))
    }
}
impl Drop for SyncFailure {
    fn drop(&mut self) {
        FAIL_SYNC.set(self.0);
    }
}

pub(super) fn checkpoint(_stage: &str) {
    #[cfg(unix)]
    permissions::checkpoint(_stage);
}

#[cfg(unix)]
fn marker(fixture: &Fixture) -> SessionMarker {
    SessionMarker {
        schema_version: SESSION_MARKER_SCHEMA.into(),
        session_id: next_session_id().unwrap(),
        owner_pid: std::process::id(),
        cache_store_root: fixture.cache.display().to_string(),
    }
}

// Non-Unix platforms may classify a non-directory parent as NotFound.
#[cfg(unix)]
#[test]
fn failed_target_inspection_never_creates_initialization_records() {
    let fixture = Fixture::new("target-inspection-io");
    fs::remove_dir(&fixture.sessions).unwrap();
    fs::write(&fixture.sessions, b"keep").unwrap();
    let catalog = CatalogLease::acquire(&fixture.cache).unwrap();
    assert!(
        begin(&fixture.sessions, &marker(&fixture), &catalog).is_err(),
        "an ENOTDIR/inspection failure must not be treated as a missing target"
    );
    assert!(
        !fixture.intents().exists(),
        "failed inspection created a recovery record"
    );
    assert_eq!(fs::read(&fixture.sessions).unwrap(), b"keep");
}

#[test]
fn staging_sync_error_never_publishes_or_creates_a_session() {
    let fixture = Fixture::new("sync-failure");
    let peer = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    let peer_path = peer.session_root.clone();
    let peer_marker = fs::read(peer_path.join(SESSION_MARKER_FILE)).unwrap();
    let failure = SyncFailure::new();
    let error = OperatorPackageGenerationSession::open(&fixture.cache).unwrap_err();
    assert!(
        error.contains("injected initialization sync failure"),
        "{error}"
    );
    let entries: Vec<_> = fs::read_dir(fixture.intents())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1);
    assert!(staging::is_staging(&entries[0]));
    assert!(!fs::read(&entries[0]).unwrap().is_empty());
    assert_eq!(fs::read_dir(&fixture.sessions).unwrap().count(), 1);
    assert_eq!(
        fs::read(peer_path.join(SESSION_MARKER_FILE)).unwrap(),
        peer_marker
    );
    drop(failure);
    let replacement = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert_eq!(
        replacement
            .janitor_report()
            .removed_initialization_staging_count,
        1
    );
    assert_eq!(
        replacement.janitor_report().retained_invalid_session_count,
        0
    );
    assert!(peer_path.exists());
    let report = replacement.janitor_report();
    replacement.refresh_stale_sessions().unwrap();
    assert_eq!(replacement.janitor_report(), report);
}

#[cfg(unix)]
mod permissions;
#[cfg(unix)]
mod write_limit;
