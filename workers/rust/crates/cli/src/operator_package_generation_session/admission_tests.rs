use super::*;

struct Fixture {
    root: PathBuf,
    cache: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-session-admission-{}",
            next_session_id().unwrap()
        ));
        fs::create_dir_all(root.join("cache/packages")).unwrap();
        let cache = root.join("cache").canonicalize().unwrap();
        Self { root, cache }
    }

    fn block(&self, session: &OperatorPackageGenerationSession, kind: &str) -> PathBuf {
        let (directory, name) = match kind {
            "staging" => (
                initialization::DIRECTORY,
                format!("{}.bad.v1.pending", session.session_id()),
            ),
            "retirement" => (
                retirement::DIRECTORY,
                format!("{}.json", session.session_id()),
            ),
            _ => (
                initialization::DIRECTORY,
                format!("{}.json", session.session_id()),
            ),
        };
        let path = self
            .cache
            .join(CACHE_GENERATIONS_DIRECTORY)
            .join(directory)
            .join(name);
        fs::write(&path, b"unresolved record").unwrap();
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn normal_session_drop_preserves_unresolved_initialization_and_staging() {
    for kind in ["initialization", "staging", "retirement"] {
        let fixture = Fixture::new();
        let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        let root = owner.session_root.clone();
        let original = fs::read(root.join(SESSION_MARKER_FILE)).unwrap();
        let record = fixture.block(&owner, kind);
        drop(owner);
        assert!(root.exists(), "normal drop bypassed {kind} protection");
        assert_eq!(fs::read(root.join(SESSION_MARKER_FILE)).unwrap(), original);
        assert_eq!(fs::read(&record).unwrap(), b"unresolved record");
        let peer = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert_eq!(peer.janitor_report().removed_stale_session_count, 0);
        assert_eq!(peer.janitor_report().retained_invalid_session_count, 1);
        fs::remove_file(record).unwrap();
        peer.refresh_stale_sessions().unwrap();
        assert!(!root.exists());
        assert_eq!(peer.janitor_report().removed_stale_session_count, 1);
    }
}

#[test]
fn unresolved_lifecycle_records_block_new_downloader_attachment() {
    for kind in ["initialization", "staging", "retirement"] {
        let fixture = Fixture::new();
        let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        let generation = owner.generations_root.join("generation");
        fs::create_dir(&generation).unwrap();
        let record = fixture.block(&owner, kind);
        assert!(
            acquire_download_session_lease(&generation).is_err(),
            "download bypassed {kind}"
        );
        assert_eq!(fs::read(&record).unwrap(), b"unresolved record");
        fs::remove_file(record).unwrap();
        let lease = acquire_download_session_lease(&generation).unwrap();
        FileExt::unlock(&lease).unwrap();
    }
}

#[test]
fn unresolved_current_session_blocks_generation_preparation_but_not_healthy_peers() {
    for kind in ["initialization", "staging", "retirement"] {
        let fixture = Fixture::new();
        let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        let record = fixture.block(&owner, kind);
        assert!(
            owner.refresh_stale_sessions().is_err(),
            "preparation bypassed {kind}"
        );
        assert_eq!(owner.janitor_report().retained_invalid_session_count, 1);
        let peer = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        peer.refresh_stale_sessions().unwrap();
        assert!(owner.session_root.exists());
        fs::remove_file(record).unwrap();
        owner.refresh_stale_sessions().unwrap();
        assert_eq!(owner.janitor_report().retained_invalid_session_count, 0);
    }
}

#[test]
fn blocked_session_never_creates_a_generation_and_explicit_retry_works() {
    use crate::operator_package_generation::prepare_operator_package_generation;
    for kind in ["initialization", "staging", "retirement"] {
        let fixture = Fixture::new();
        let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        let record = fixture.block(&owner, kind);
        let prepare = || {
            prepare_operator_package_generation(
                owner.clone(),
                &fixture.cache.join("packages"),
                "operator.next",
            )
        };
        assert!(prepare().is_err(), "generation created through {kind}");
        assert_eq!(fs::read_dir(&owner.generations_root).unwrap().count(), 0);
        fs::remove_file(record).unwrap();
        let prepared = prepare().unwrap();
        assert!(prepared.packages_root().is_dir());
        drop(prepared);
        assert_eq!(fs::read_dir(&owner.generations_root).unwrap().count(), 0);
    }
}

#[test]
fn lifecycle_admission_never_revokes_existing_downloader_lease() {
    for kind in ["initialization", "staging", "retirement"] {
        let fixture = Fixture::new();
        let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        let root = owner.session_root.clone();
        let generation = owner.generations_root.join("downloading");
        fs::create_dir(&generation).unwrap();
        fs::write(generation.join("partial"), b"keep").unwrap();
        let existing = acquire_download_session_lease(&generation).unwrap();
        let record = fixture.block(&owner, kind);
        assert!(acquire_download_session_lease(&generation).is_err());
        drop(owner);
        let replacement = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert_eq!(replacement.janitor_report().removed_stale_session_count, 0);
        assert_eq!(fs::read(generation.join("partial")).unwrap(), b"keep");
        fs::remove_file(record).unwrap();
        replacement.refresh_stale_sessions().unwrap();
        assert_eq!(
            replacement.janitor_report().retained_active_session_count,
            1
        );
        assert!(root.exists());
        FileExt::unlock(&existing).unwrap();
        drop(existing);
        replacement.refresh_stale_sessions().unwrap();
        assert!(!root.exists());
        assert_eq!(replacement.janitor_report().removed_stale_session_count, 1);
        replacement.refresh_stale_sessions().unwrap();
        assert_eq!(replacement.janitor_report().removed_stale_session_count, 1);
    }
}

mod generation_cleanup;
mod namespaces;

#[test]
fn matching_but_uncommitted_initialization_blocks_admission_until_owner_closes() {
    let fixture = Fixture::new();
    let owner = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    let root = owner.session_root.clone();
    let generation = owner.generations_root.join("generation");
    fs::create_dir(&generation).unwrap();
    let path = fixture
        .cache
        .join(CACHE_GENERATIONS_DIRECTORY)
        .join(initialization::DIRECTORY)
        .join(format!("{}.json", owner.session_id()));
    fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version":"kyuubiki.agent-session-initialization/v1", "session":owner.marker,
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(acquire_download_session_lease(&generation).is_err());
    assert!(owner.refresh_stale_sessions().is_err());
    assert_eq!(owner.janitor_report().retained_active_session_count, 1);
    fs::remove_dir(generation).unwrap();
    drop(owner);
    assert!(root.join(SESSION_MARKER_FILE).is_file());
    assert!(path.is_file());
    let healthy = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert!(!root.exists());
    assert!(!path.exists());
    assert_eq!(healthy.janitor_report().removed_stale_session_count, 1);
    assert_eq!(healthy.janitor_report().retained_invalid_session_count, 0);
    healthy.refresh_stale_sessions().unwrap();
}
