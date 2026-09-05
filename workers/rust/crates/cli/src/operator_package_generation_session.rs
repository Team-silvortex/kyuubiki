use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_GENERATIONS_DIRECTORY: &str = "agent-runtime-generations";
const SESSIONS_DIRECTORY: &str = "sessions";
const SESSION_GENERATIONS_DIRECTORY: &str = "generations";
const SESSION_MARKER_FILE: &str = "kyuubiki-agent-session.json";
const SESSION_LEASE_FILE: &str = "kyuubiki-agent-session.lease";
const SESSION_MARKER_SCHEMA: &str = "kyuubiki.agent-operator-generation-session/v1";
const MAX_MARKER_BYTES: u64 = 16 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct GenerationJanitorReport {
    pub removed_stale_session_count: usize,
    pub retained_active_session_count: usize,
    pub retained_invalid_session_count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct SessionMarker {
    schema_version: String,
    session_id: String,
    owner_pid: u32,
    cache_store_root: String,
}

#[derive(Debug)]
pub(crate) struct OperatorPackageGenerationSession {
    session_root: PathBuf,
    sessions_root: PathBuf,
    generations_root: PathBuf,
    marker: SessionMarker,
    lease_file: Mutex<Option<File>>,
    janitor_report: GenerationJanitorReport,
}

impl OperatorPackageGenerationSession {
    pub(crate) fn open(cache_store_root: &Path) -> Result<Arc<Self>, String> {
        let cache_store_root = canonical_directory(cache_store_root, "operator package cache")?;
        let sessions_root = prepare_sessions_root(&cache_store_root)?;
        let mut janitor_report = reap_stale_sessions(&sessions_root, &cache_store_root)?;
        janitor_report.retained_invalid_session_count +=
            count_unleased_cache_entries(&sessions_root)?;
        let session_id = next_session_id()?;
        let session_root = sessions_root.join(&session_id);
        fs::create_dir(&session_root).map_err(|error| {
            format!(
                "failed to create operator package session {}: {error}",
                session_root.display()
            )
        })?;

        let result = create_session(
            cache_store_root,
            sessions_root,
            session_root.clone(),
            session_id,
            janitor_report,
        );
        if result.is_err() {
            let _ = fs::remove_dir_all(session_root);
        }
        result.map(Arc::new)
    }

    pub(crate) fn session_id(&self) -> &str {
        &self.marker.session_id
    }

    pub(crate) fn generations_root(&self) -> &Path {
        &self.generations_root
    }

    pub(crate) fn janitor_report(&self) -> GenerationJanitorReport {
        self.janitor_report
    }
}

impl Drop for OperatorPackageGenerationSession {
    fn drop(&mut self) {
        let lease = self
            .lease_file
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(lease) = lease {
            let _ = FileExt::unlock(&lease);
            drop(lease);
        }
        if session_identity_matches(&self.session_root, &self.marker).unwrap_or(false)
            && directory_is_empty(&self.generations_root).unwrap_or(false)
        {
            let _ = fs::remove_dir_all(&self.session_root);
            let _ = prune_empty_directory(&self.sessions_root);
            if let Some(parent) = self.sessions_root.parent() {
                let _ = prune_empty_directory(parent);
            }
        }
    }
}

fn create_session(
    cache_store_root: PathBuf,
    sessions_root: PathBuf,
    session_root: PathBuf,
    session_id: String,
    janitor_report: GenerationJanitorReport,
) -> Result<OperatorPackageGenerationSession, String> {
    let marker = SessionMarker {
        schema_version: SESSION_MARKER_SCHEMA.to_string(),
        session_id,
        owner_pid: std::process::id(),
        cache_store_root: cache_store_root.display().to_string(),
    };
    write_marker(&session_root, &marker)?;
    let lease_path = session_root.join(SESSION_LEASE_FILE);
    let mut lease = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&lease_path)
        .map_err(|error| format!("failed to create {}: {error}", lease_path.display()))?;
    writeln!(lease, "session_id={}", marker.session_id)
        .and_then(|_| lease.sync_all())
        .map_err(|error| format!("failed to persist {}: {error}", lease_path.display()))?;
    lease
        .try_lock_exclusive()
        .map_err(|error| format!("failed to lock {}: {error}", lease_path.display()))?;

    let generations_root = session_root.join(SESSION_GENERATIONS_DIRECTORY);
    fs::create_dir(&generations_root).map_err(|error| {
        format!(
            "failed to create session generations root {}: {error}",
            generations_root.display()
        )
    })?;
    let generations_root = canonical_directory(&generations_root, "session generations root")?;
    Ok(OperatorPackageGenerationSession {
        session_root,
        sessions_root,
        generations_root,
        marker,
        lease_file: Mutex::new(Some(lease)),
        janitor_report,
    })
}

fn prepare_sessions_root(cache_store_root: &Path) -> Result<PathBuf, String> {
    let cache_generations_root = cache_store_root.join(CACHE_GENERATIONS_DIRECTORY);
    create_managed_directory(&cache_generations_root, "operator package generations root")?;
    let cache_generations_root =
        canonical_directory(&cache_generations_root, "operator package generations root")?;
    if cache_generations_root.parent() != Some(cache_store_root) {
        return Err("operator package generations root escaped its cache store".to_string());
    }
    let sessions_root = cache_generations_root.join(SESSIONS_DIRECTORY);
    create_managed_directory(&sessions_root, "operator package sessions root")?;
    canonical_directory(&sessions_root, "operator package sessions root")
}

fn reap_stale_sessions(
    sessions_root: &Path,
    cache_store_root: &Path,
) -> Result<GenerationJanitorReport, String> {
    let mut entries = fs::read_dir(sessions_root)
        .map_err(|error| format!("failed to read {}: {error}", sessions_root.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut report = GenerationJanitorReport::default();
    for entry in entries {
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", entry.path().display()))?;
        if file_type.is_symlink() || !file_type.is_dir() {
            report.retained_invalid_session_count += 1;
            continue;
        }
        match try_reap_session(&entry.path(), cache_store_root) {
            Ok(SessionReapOutcome::Removed) => report.removed_stale_session_count += 1,
            Ok(SessionReapOutcome::Active) => report.retained_active_session_count += 1,
            Err(_) => report.retained_invalid_session_count += 1,
        }
    }
    Ok(report)
}

fn count_unleased_cache_entries(sessions_root: &Path) -> Result<usize, String> {
    let cache_generations_root = sessions_root
        .parent()
        .ok_or_else(|| "operator package sessions root has no cache parent".to_string())?;
    let mut count = 0_usize;
    for entry in fs::read_dir(cache_generations_root).map_err(|error| {
        format!(
            "failed to read {}: {error}",
            cache_generations_root.display()
        )
    })? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.file_name() != SESSIONS_DIRECTORY {
            count += 1;
        }
    }
    Ok(count)
}

enum SessionReapOutcome {
    Removed,
    Active,
}

fn try_reap_session(
    session_root: &Path,
    cache_store_root: &Path,
) -> Result<SessionReapOutcome, String> {
    let marker = read_marker(session_root)?;
    if marker.cache_store_root != cache_store_root.display().to_string()
        || session_root.file_name().and_then(|value| value.to_str())
            != Some(marker.session_id.as_str())
    {
        return Err("operator package session marker identity mismatch".to_string());
    }
    let lease_path = session_root.join(SESSION_LEASE_FILE);
    reject_symlink(&lease_path, "operator package session lease")?;
    let lease = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lease_path)
        .map_err(|error| format!("failed to open {}: {error}", lease_path.display()))?;
    match lease.try_lock_exclusive() {
        Ok(()) => {
            FileExt::unlock(&lease)
                .map_err(|error| format!("failed to unlock {}: {error}", lease_path.display()))?;
            drop(lease);
            fs::remove_dir_all(session_root).map_err(|error| {
                format!(
                    "failed to remove stale operator package session {}: {error}",
                    session_root.display()
                )
            })?;
            Ok(SessionReapOutcome::Removed)
        }
        Err(error) if is_lock_contention(&error) => Ok(SessionReapOutcome::Active),
        Err(error) => Err(format!(
            "failed to inspect operator package session lease {}: {error}",
            lease_path.display()
        )),
    }
}

fn is_lock_contention(error: &std::io::Error) -> bool {
    error.kind() == ErrorKind::WouldBlock
        || error
            .raw_os_error()
            .is_some_and(|code| fs2::lock_contended_error().raw_os_error() == Some(code))
}

fn session_identity_matches(root: &Path, expected: &SessionMarker) -> Result<bool, String> {
    Ok(read_marker(root)? == *expected
        && root.file_name().and_then(|value| value.to_str()) == Some(expected.session_id.as_str()))
}

fn write_marker(root: &Path, marker: &SessionMarker) -> Result<(), String> {
    let path = root.join(SESSION_MARKER_FILE);
    let payload = serde_json::to_vec_pretty(marker).map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    file.write_all(&payload)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("failed to persist {}: {error}", path.display()))
}

fn read_marker(root: &Path) -> Result<SessionMarker, String> {
    let path = root.join(SESSION_MARKER_FILE);
    reject_symlink(&path, "operator package session marker")?;
    let metadata = path
        .metadata()
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_MARKER_BYTES {
        return Err("operator package session marker has an invalid size".to_string());
    }
    let payload =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let marker: SessionMarker = serde_json::from_slice(&payload)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    if marker.schema_version != SESSION_MARKER_SCHEMA {
        return Err("unsupported operator package session marker schema".to_string());
    }
    Ok(marker)
}

fn next_session_id() -> Result<String, String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(format!("session-{}-{nonce}-{counter}", std::process::id()))
}

fn create_managed_directory(path: &Path, label: &str) -> Result<(), String> {
    reject_symlink(path, label)?;
    fs::create_dir_all(path)
        .map_err(|error| format!("failed to create {label} {}: {error}", path.display()))
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf, String> {
    reject_symlink(path, label)?;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))?;
    if !canonical.is_dir() {
        return Err(format!("{label} must be a directory"));
    }
    Ok(canonical)
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), String> {
    if path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(format!("{label} must not be a symlink: {}", path.display()));
    }
    Ok(())
}

fn directory_is_empty(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(true);
    }
    Ok(fs::read_dir(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?
        .next()
        .is_none())
}

fn prune_empty_directory(path: &Path) -> Result<(), String> {
    if directory_is_empty(path)? {
        fs::remove_dir(path)
            .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_reaps_an_unlocked_stale_session() {
        let root = temporary_root("stale-session");
        fs::create_dir_all(root.join("packages")).expect("create cache root");
        let cache_root = root.canonicalize().expect("canonical cache root");
        let sessions_root = prepare_sessions_root(&cache_root).expect("prepare sessions root");
        let stale = create_stale_fixture(&sessions_root, &cache_root);

        let session = OperatorPackageGenerationSession::open(&cache_root).expect("open session");
        assert_eq!(session.janitor_report().removed_stale_session_count, 1);
        assert!(!stale.exists());
        drop(session);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn new_session_preserves_a_locked_peer_session() {
        let root = temporary_root("active-session");
        fs::create_dir_all(root.join("packages")).expect("create cache root");
        let first = OperatorPackageGenerationSession::open(&root).expect("open first session");
        let first_root = first.session_root.clone();
        let second = OperatorPackageGenerationSession::open(&root).expect("open second session");
        assert_eq!(second.janitor_report().retained_active_session_count, 1);
        assert!(first_root.exists());
        drop(second);
        drop(first);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn lock_contention_classification_preserves_real_io_failures() {
        assert!(is_lock_contention(&std::io::Error::from(
            ErrorKind::WouldBlock
        )));
        assert!(is_lock_contention(&fs2::lock_contended_error()));
        assert!(!is_lock_contention(&std::io::Error::from(
            ErrorKind::PermissionDenied
        )));
    }

    #[test]
    fn new_session_retains_a_malformed_peer_fail_closed() {
        let root = temporary_root("invalid-session");
        fs::create_dir_all(root.join("packages")).expect("create cache root");
        let cache_root = root.canonicalize().expect("canonical cache root");
        let sessions_root = prepare_sessions_root(&cache_root).expect("prepare sessions root");
        let invalid = sessions_root.join("session-invalid-fixture");
        let unleased = sessions_root
            .parent()
            .expect("cache generations root")
            .join("legacy-unleased-generation");
        fs::create_dir(&invalid).expect("create invalid session");
        fs::create_dir(&unleased).expect("create unleased generation");
        fs::write(invalid.join(SESSION_MARKER_FILE), b"{}").expect("write invalid marker");
        fs::write(invalid.join(SESSION_LEASE_FILE), b"invalid\n").expect("write invalid lease");

        let session = OperatorPackageGenerationSession::open(&cache_root).expect("open session");
        assert_eq!(session.janitor_report().retained_invalid_session_count, 2);
        assert!(invalid.exists());
        assert!(unleased.exists());
        drop(session);
        let _ = fs::remove_dir_all(root);
    }

    fn create_stale_fixture(sessions_root: &Path, cache_root: &Path) -> PathBuf {
        let session_id = "session-stale-fixture";
        let root = sessions_root.join(session_id);
        fs::create_dir(&root).expect("create stale session");
        write_marker(
            &root,
            &SessionMarker {
                schema_version: SESSION_MARKER_SCHEMA.to_string(),
                session_id: session_id.to_string(),
                owner_pid: 1,
                cache_store_root: cache_root.display().to_string(),
            },
        )
        .expect("write stale marker");
        fs::write(root.join(SESSION_LEASE_FILE), b"stale\n").expect("write stale lease");
        fs::create_dir(root.join(SESSION_GENERATIONS_DIRECTORY)).expect("create stale generations");
        root
    }

    fn temporary_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("kyuubiki-generation-session-{label}-{nonce}"))
    }
}
