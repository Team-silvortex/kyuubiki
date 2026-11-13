//! Creation intent precedes the session directory; recovery never recursively deletes it.
use super::*;
use std::collections::HashSet;
use std::io::Read;

pub(super) const DIRECTORY: &str = "initializations";
const SCHEMA: &str = "kyuubiki.agent-session-initialization/v1";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    schema_version: String,
    session: SessionMarker,
}

#[derive(Default)]
pub(super) struct Recovery {
    pub report: GenerationJanitorReport,
    pub protected: HashSet<PathBuf>,
}

fn directory(sessions: &Path) -> Result<PathBuf, String> {
    let parent = sessions.parent().ok_or("session catalog has no parent")?;
    let path = parent.join(DIRECTORY);
    create_managed_directory(&path, "session initialization records")?;
    let path = canonical_directory(&path, "session initialization records")?;
    if path.parent() != Some(parent) {
        return Err("session initialization records escaped their catalog".into());
    }
    Ok(path)
}

pub(super) fn begin(
    sessions: &Path,
    marker: &SessionMarker,
    _catalog: &CatalogLease,
) -> Result<PathBuf, String> {
    let root = sessions.join(&marker.session_id);
    if root.symlink_metadata().is_ok() {
        return Err("operator session initialization target already exists".into());
    }
    let intent = Intent {
        schema_version: SCHEMA.into(),
        session: marker.clone(),
    };
    let path = directory(sessions)?.join(format!("{}.json", marker.session_id));
    let bytes = serde_json::to_vec_pretty(&intent).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_MARKER_BYTES {
        return Err("operator session initialization record exceeds its limit".into());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("cannot create session initialization record: {error}"))?;
    #[cfg(test)]
    partial_checkpoint("intent-partial", &root, &mut file, &bytes)?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("cannot persist session initialization record: {error}"))?;
    checkpoint("intent-synced", &root);
    Ok(path)
}

pub(super) fn recover(
    sessions: &Path,
    cache: &Path,
    _catalog: &CatalogLease,
) -> Result<Recovery, String> {
    let mut recovery = Recovery::default();
    for entry in fs::read_dir(directory(sessions)?).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if let Some(id) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.strip_suffix(".json"))
            && valid_session_id(id)
        {
            recovery.protected.insert(sessions.join(id));
        }
        let parsed = read_intent(&entry.path(), cache);
        let Ok(intent) = parsed else {
            recovery.report.retained_invalid_session_count += 1;
            continue;
        };
        let root = sessions.join(&intent.session.session_id);
        // Never let the ordinary recursive janitor bypass a rejected initialization record.
        recovery.protected.insert(root.clone());
        match reap_incomplete(&root, &intent.session) {
            Ok(SessionReapOutcome::Removed) => {
                if fs::remove_file(entry.path()).is_ok() {
                    recovery.report.removed_stale_session_count += 1;
                } else {
                    recovery.report.retained_invalid_session_count += 1;
                }
            }
            Ok(SessionReapOutcome::Active) => recovery.report.retained_active_session_count += 1,
            Err(_) => recovery.report.retained_invalid_session_count += 1,
        }
    }
    Ok(recovery)
}

fn read_intent(path: &Path, cache: &Path) -> Result<Intent, String> {
    let bytes = read_regular(path)?;
    let intent: Intent = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    let marker = &intent.session;
    let id = &marker.session_id;
    if intent.schema_version != SCHEMA
        || marker.schema_version != SESSION_MARKER_SCHEMA
        || marker.owner_pid == 0
        || marker.cache_store_root != cache.display().to_string()
        || !valid_session_id(id)
        || path.file_name().and_then(|name| name.to_str()) != Some(&format!("{id}.json"))
    {
        return Err("session initialization identity mismatch".into());
    }
    Ok(intent)
}

fn valid_session_id(id: &str) -> bool {
    id.starts_with("session-")
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn read_regular(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = path.symlink_metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_MARKER_BYTES {
        return Err("initialization entry must be a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|error| error.to_string())?
        .take(MAX_MARKER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_MARKER_BYTES {
        return Err("initialization entry grew beyond its limit".into());
    }
    Ok(bytes)
}

fn missing(path: &Path) -> Result<bool, String> {
    match path.symlink_metadata() {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.to_string()),
    }
}

fn reap_incomplete(root: &Path, expected: &SessionMarker) -> Result<SessionReapOutcome, String> {
    if missing(root)? {
        return Ok(SessionReapOutcome::Removed);
    }
    if !root
        .symlink_metadata()
        .map_err(|error| error.to_string())?
        .is_dir()
    {
        return Err("initialization target must be a real directory".into());
    }
    let lease_path = root.join(SESSION_LEASE_FILE);
    let lease = if missing(&lease_path)? {
        None
    } else {
        read_regular(&lease_path)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lease_path)
            .map_err(|error| error.to_string())?;
        match file.try_lock_exclusive() {
            Ok(()) => Some(file),
            Err(error) if is_lock_contention(&error) => return Ok(SessionReapOutcome::Active),
            Err(error) => return Err(error.to_string()),
        }
    };
    let marker_bytes = serde_json::to_vec_pretty(expected).map_err(|error| error.to_string())?;
    let lease_bytes = format!("session_id={}\n", expected.session_id);
    let mut files = Vec::new();
    let mut generations = None;
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if entry.file_name() == SESSION_GENERATIONS_DIRECTORY {
            if !entry
                .file_type()
                .map_err(|error| error.to_string())?
                .is_dir()
                || !directory_is_empty(&path)?
            {
                return Err("incomplete session contains generation payload or a symlink".into());
            }
            generations = Some(path);
        } else {
            let expected_bytes = if entry.file_name() == SESSION_MARKER_FILE {
                marker_bytes.as_slice()
            } else if entry.file_name() == SESSION_LEASE_FILE {
                lease_bytes.as_bytes()
            } else {
                return Err("incomplete session contains an unknown entry".into());
            };
            if !expected_bytes.starts_with(&read_regular(&path)?) {
                return Err(
                    "incomplete session metadata does not match its creation intent".into(),
                );
            }
            files.push(path);
        }
    }
    // Windows needs the lease handle closed before unlink; catalog lock excludes new attachers.
    drop(lease);
    for file in files {
        fs::remove_file(file).map_err(|error| error.to_string())?;
        checkpoint("cleanup-file-removed", root);
    }
    if let Some(generations) = generations {
        fs::remove_dir(generations).map_err(|error| error.to_string())?;
    }
    fs::remove_dir(root).map_err(|error| error.to_string())?;
    checkpoint("cleanup-directory-removed", root);
    Ok(SessionReapOutcome::Removed)
}

// Failure barriers are absent from production builds.
pub(super) fn checkpoint(_stage: &str, _root: &Path) {
    #[cfg(test)]
    tests::checkpoint(_stage, _root);
}

#[cfg(test)]
pub(super) fn partial_checkpoint(
    stage: &str,
    root: &Path,
    file: &mut File,
    bytes: &[u8],
) -> Result<(), String> {
    if std::env::var("KYUUBIKI_SESSION_TEST_PHASE").as_deref() == Ok(stage) {
        file.write_all(&bytes[..bytes.len() / 2])
            .and_then(|_| file.sync_all())
            .map_err(|error| error.to_string())?;
        checkpoint(stage, root);
        return Err("initialization test checkpoint unexpectedly returned".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
