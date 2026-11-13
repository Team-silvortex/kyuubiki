use super::*;
use std::time::Duration;
#[cfg(unix)]
use std::{
    process::{Child, Command, Stdio},
    time::Instant,
};

#[cfg(unix)]
const CHILD: &str =
    "operator_package_generation_session::initialization::tests::initialization_crash_child";

pub(super) fn checkpoint(stage: &str, session: &Path) {
    if std::env::var("KYUUBIKI_SESSION_TEST_PHASE").as_deref() != Ok(stage) {
        return;
    }
    let ready = PathBuf::from(
        std::env::var_os("KYUUBIKI_SESSION_TEST_READY").expect("test readiness path"),
    );
    let pending = ready.with_extension("pending");
    fs::write(&pending, session.to_string_lossy().as_bytes()).unwrap();
    fs::rename(pending, ready).unwrap();
    loop {
        std::thread::park_timeout(Duration::from_secs(60));
    }
}

#[test]
#[ignore = "owned subprocess entrypoint, invoked by the initialization crash regression"]
fn initialization_crash_child() {
    let Some(cache) = std::env::var_os("KYUUBIKI_SESSION_TEST_CACHE") else {
        return;
    };
    let _session = OperatorPackageGenerationSession::open(Path::new(&cache)).unwrap();
    panic!("initialization did not reach the selected crash checkpoint");
}

#[cfg(unix)]
#[test]
fn killed_initializer_recovers_each_committed_creation_boundary() {
    for stage in [
        "intent-synced",
        "directory-created",
        "marker-created",
        "marker-partial",
        "marker-synced",
        "lease-created",
        "lease-partial",
        "lease-locked",
        "generations-created",
        "initialization-committed",
    ] {
        let fixture = Fixture::new(stage);
        let stale = kill_at_checkpoint(&fixture, stage);
        let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert!(!stale.exists(), "stale session survived {stage}");
        assert_eq!(
            session.janitor_report().removed_stale_session_count,
            1,
            "{stage}"
        );
        assert_eq!(
            session.janitor_report().retained_invalid_session_count,
            0,
            "{stage}"
        );
        assert_eq!(
            fs::read_dir(fixture.intents()).unwrap().count(),
            0,
            "{stage}"
        );
        session.refresh_stale_sessions().unwrap();
        assert_eq!(
            session.janitor_report().removed_stale_session_count,
            1,
            "no double count"
        );
        drop(session);
        let next = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert_eq!(next.janitor_report().removed_stale_session_count, 0);
    }
}

#[test]
fn legacy_torn_intent_remains_unproven_without_creating_a_session() {
    let fixture = Fixture::new("legacy-partial-intent");
    let stale = fixture.incomplete();
    fs::remove_dir_all(&stale).unwrap();
    assert!(!stale.exists());
    let record = fs::read_dir(fixture.intents())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let complete = fs::read(&record).unwrap();
    let original = complete[..complete.len() / 2].to_vec();
    fs::write(&record, &original).unwrap();
    assert!(!original.is_empty());
    let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert_eq!(session.janitor_report().retained_invalid_session_count, 1);
    assert_eq!(session.janitor_report().removed_stale_session_count, 0);
    assert_eq!(fs::read(record).unwrap(), original);
    assert!(!stale.exists());
}

#[cfg(unix)]
#[test]
fn killed_initializer_cleanup_resumes_without_recursive_deletion() {
    for stage in ["cleanup-file-removed", "cleanup-directory-removed"] {
        let fixture = Fixture::new(stage);
        let stale = fixture.incomplete();
        assert_eq!(kill_at_checkpoint(&fixture, stage), stale);
        let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert!(!stale.exists());
        assert_eq!(session.janitor_report().removed_stale_session_count, 1);
        assert_eq!(session.janitor_report().retained_invalid_session_count, 0);
    }
}

#[test]
fn incomplete_initialization_requires_exact_metadata_and_no_payload() {
    for case in [
        "foreign-marker",
        "foreign-lease",
        "unknown-file",
        "generation-payload",
        "large-marker",
    ] {
        let fixture = Fixture::new(case);
        let stale = fixture.incomplete();
        let kept = match case {
            "foreign-marker" => {
                let path = stale.join(SESSION_MARKER_FILE);
                fs::write(&path, b"{\"other\":true}").unwrap();
                path
            }
            "foreign-lease" => {
                let path = stale.join(SESSION_LEASE_FILE);
                fs::write(&path, b"session_id=foreign\n").unwrap();
                path
            }
            "unknown-file" => {
                let path = stale.join("user-notes.txt");
                fs::write(&path, b"keep").unwrap();
                path
            }
            "generation-payload" => {
                let path = stale.join(SESSION_GENERATIONS_DIRECTORY).join("payload");
                fs::write(&path, b"keep").unwrap();
                path
            }
            _ => {
                let path = stale.join(SESSION_MARKER_FILE);
                fs::write(&path, vec![b' '; MAX_MARKER_BYTES as usize + 1]).unwrap();
                path
            }
        };
        let original = fs::read(&kept).unwrap();
        let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert_eq!(
            session.janitor_report().retained_invalid_session_count,
            1,
            "{case}"
        );
        assert_eq!(
            session.janitor_report().removed_stale_session_count,
            0,
            "{case}"
        );
        assert_eq!(fs::read(kept).unwrap(), original, "{case}");
        assert!(stale.exists());
    }
}

#[test]
fn partial_marker_cleanup_requires_a_prior_matching_intent() {
    let fixture = Fixture::new("partial-marker-proof");
    let owned = fixture.incomplete();
    let marker = fs::read(owned.join(SESSION_MARKER_FILE)).unwrap();
    fs::write(owned.join(SESSION_MARKER_FILE), &marker[..marker.len() / 2]).unwrap();
    let unknown = fixture.sessions.join("session-unclaimed");
    fs::create_dir(&unknown).unwrap();
    fs::write(
        unknown.join(SESSION_MARKER_FILE),
        &marker[..marker.len() / 2],
    )
    .unwrap();
    let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert!(
        !owned.exists(),
        "proven partial initialization was not recovered"
    );
    assert!(unknown.exists(), "unproven lookalike must never be swept");
    assert_eq!(session.janitor_report().removed_stale_session_count, 1);
    assert_eq!(session.janitor_report().retained_invalid_session_count, 1);
}

#[test]
fn incomplete_initialization_preserves_lease_holder_then_retries() {
    let fixture = Fixture::new("held-lease");
    let stale = fixture.incomplete();
    let lease = File::open(stale.join(SESSION_LEASE_FILE)).unwrap();
    FileExt::try_lock_shared(&lease).unwrap();
    let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert!(stale.exists());
    assert_eq!(session.janitor_report().retained_active_session_count, 1);
    assert_eq!(session.janitor_report().retained_invalid_session_count, 0);
    FileExt::unlock(&lease).unwrap();
    drop(lease);
    session.refresh_stale_sessions().unwrap();
    assert!(!stale.exists());
    assert_eq!(session.janitor_report().removed_stale_session_count, 1);
}

#[test]
fn initialization_intent_rejects_foreign_identity_and_path_traversal() {
    for case in ["wrong-cache", "wrong-name", "traversal"] {
        let fixture = Fixture::new(case);
        let stale = fixture.incomplete();
        let record = fixture.intents().join(format!(
            "{}.json",
            stale.file_name().unwrap().to_str().unwrap()
        ));
        let mut intent: Intent = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
        match case {
            "wrong-cache" => intent.session.cache_store_root.push_str("/other"),
            "wrong-name" => intent.session.session_id = "session-other".into(),
            _ => intent.session.session_id = "session-../../outside".into(),
        }
        fs::write(&record, serde_json::to_vec(&intent).unwrap()).unwrap();
        // An incomplete marker cannot independently authorize the ordinary janitor either.
        fs::write(stale.join(SESSION_MARKER_FILE), b"{\"schema").unwrap();
        let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert!(stale.exists(), "{case}");
        assert!(record.exists(), "{case}");
        assert_eq!(session.janitor_report().removed_stale_session_count, 0);
    }
}

#[cfg(unix)]
#[test]
fn incomplete_initialization_preserves_symlinks_and_their_targets() {
    for entry in [
        SESSION_MARKER_FILE,
        SESSION_LEASE_FILE,
        SESSION_GENERATIONS_DIRECTORY,
        "root",
        "record",
    ] {
        let fixture = Fixture::new(entry);
        let stale = fixture.incomplete();
        let outside = fixture.root.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep"), b"keep").unwrap();
        let link = match entry {
            "root" => {
                fs::remove_dir_all(&stale).unwrap();
                stale.clone()
            }
            "record" => {
                let path = fixture.intents().join(format!(
                    "{}.json",
                    stale.file_name().unwrap().to_str().unwrap()
                ));
                fs::remove_file(&path).unwrap();
                path
            }
            SESSION_GENERATIONS_DIRECTORY => {
                let path = stale.join(entry);
                fs::remove_dir(&path).unwrap();
                path
            }
            _ => {
                let path = stale.join(entry);
                fs::remove_file(&path).unwrap();
                path
            }
        };
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert!(
            link.symlink_metadata().unwrap().file_type().is_symlink(),
            "{entry}"
        );
        assert_eq!(fs::read(outside.join("keep")).unwrap(), b"keep");
        assert_eq!(session.janitor_report().removed_stale_session_count, 0);
    }
}

#[test]
fn initialization_never_claims_an_existing_target() {
    let fixture = Fixture::new("existing");
    let stale = fixture.incomplete();
    let marker = read_marker(&stale).unwrap();
    let catalog = CatalogLease::acquire(&fixture.cache).unwrap();
    assert!(begin(&fixture.sessions, &marker, &catalog).is_err());
    assert!(stale.exists());
}

pub(super) struct Fixture {
    pub(super) root: PathBuf,
    pub(super) cache: PathBuf,
    pub(super) sessions: PathBuf,
}

impl Fixture {
    pub(super) fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-init-recovery-{label}-{}",
            next_session_id().unwrap()
        ));
        fs::create_dir_all(root.join("cache/packages")).unwrap();
        let cache = root.join("cache").canonicalize().unwrap();
        let sessions = prepare_sessions_root(&cache).unwrap();
        Self {
            root,
            cache,
            sessions,
        }
    }

    pub(super) fn intents(&self) -> PathBuf {
        self.sessions.parent().unwrap().join(DIRECTORY)
    }

    pub(super) fn incomplete(&self) -> PathBuf {
        let catalog = CatalogLease::acquire(&self.cache).unwrap();
        let marker = SessionMarker {
            schema_version: SESSION_MARKER_SCHEMA.into(),
            session_id: next_session_id().unwrap(),
            owner_pid: std::process::id(),
            cache_store_root: self.cache.display().to_string(),
        };
        begin(&self.sessions, &marker, &catalog).unwrap();
        let root = self.sessions.join(&marker.session_id);
        fs::create_dir(&root).unwrap();
        write_marker(&root, &marker).unwrap();
        fs::write(
            root.join(SESSION_LEASE_FILE),
            format!("session_id={}\n", marker.session_id),
        )
        .unwrap();
        fs::create_dir(root.join(SESSION_GENERATIONS_DIRECTORY)).unwrap();
        root
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[cfg(unix)]
struct OwnedChild(Child);
#[cfg(unix)]
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg(unix)]
pub(super) fn kill_at_checkpoint(fixture: &Fixture, stage: &str) -> PathBuf {
    use std::os::unix::process::ExitStatusExt;
    let ready = fixture.root.join("ready");
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD, "--ignored", "--nocapture"])
            .env("KYUUBIKI_SESSION_TEST_PHASE", stage)
            .env("KYUUBIKI_SESSION_TEST_CACHE", &fixture.cache)
            .env("KYUUBIKI_SESSION_TEST_READY", &ready)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.is_file() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited before {stage}"
        );
        assert!(Instant::now() < deadline, "timeout at {stage}");
        std::thread::sleep(Duration::from_millis(5));
    }
    child.0.kill().unwrap();
    assert_eq!(child.0.wait().unwrap().signal(), Some(libc::SIGKILL));
    PathBuf::from(fs::read_to_string(ready).unwrap())
}
