//! A retained identity outlives payload deletion and the final metadata unlink window.
use super::*;
use std::{collections::HashSet, io::Read};

pub(super) const DIRECTORY: &str = "retirements";

#[derive(Default)]
pub(super) struct Recovery {
    pub report: GenerationJanitorReport,
    pub protected: HashSet<PathBuf>,
}

fn directory(sessions: &Path) -> Result<PathBuf, String> {
    let parent = sessions.parent().ok_or("session catalog has no parent")?;
    let path = parent.join(DIRECTORY);
    create_managed_directory(&path, "session retirement records")?;
    let path = canonical_directory(&path, "session retirement records")?;
    if path.parent() != Some(parent) {
        return Err("session retirement records escaped their catalog".into());
    }
    Ok(path)
}

fn missing(path: &Path) -> Result<bool, String> {
    match path.symlink_metadata() {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.to_string()),
    }
}

fn regular_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = path.symlink_metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_MARKER_BYTES {
        return Err("retirement metadata must be a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|error| error.to_string())?
        .take(MAX_MARKER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_MARKER_BYTES {
        return Err("retirement metadata grew beyond its limit".into());
    }
    Ok(bytes)
}

pub(super) fn has_record(sessions: &Path, id: &str) -> Result<bool, String> {
    let Some(directory) = lifecycle::record_directory(sessions, DIRECTORY)? else {
        return Ok(false);
    };
    Ok(!missing(&directory.join(format!("{id}.json")))?)
}

fn valid_id(id: &str) -> bool {
    id.starts_with("session-")
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn identity(bytes: &[u8], id: &str, cache: &Path) -> Result<SessionMarker, String> {
    let marker: SessionMarker = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if !valid_id(id)
        || marker.session_id != id
        || marker.owner_pid == 0
        || marker.schema_version != SESSION_MARKER_SCHEMA
        || marker.cache_store_root != cache.display().to_string()
    {
        return Err("retirement identity mismatch".into());
    }
    Ok(marker)
}

struct Lease(File);
impl Drop for Lease {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}

fn lock(root: &Path) -> Result<Option<Lease>, String> {
    let path = root.join(SESSION_LEASE_FILE);
    regular_bytes(&path)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|error| error.to_string())?;
    match file.try_lock_exclusive() {
        Ok(()) => Ok(Some(Lease(file))),
        Err(error) if is_lock_contention(&error) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn inspect_root(root: &Path) -> Result<(), String> {
    if !root.symlink_metadata().map_err(|e| e.to_string())?.is_dir() {
        return Err("retirement target must be a real directory".into());
    }
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_name() == SESSION_GENERATIONS_DIRECTORY {
            if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                return Err("retirement payload must be a real directory".into());
            }
        } else if entry.file_name() != SESSION_MARKER_FILE
            && entry.file_name() != SESSION_LEASE_FILE
        {
            return Err("retirement target contains an unknown entry".into());
        }
    }
    Ok(())
}

pub(super) fn reap(
    root: &Path,
    cache: &Path,
    _catalog: &CatalogLease,
) -> Result<SessionReapOutcome, String> {
    inspect_root(root)?;
    let bytes = regular_bytes(&root.join(SESSION_MARKER_FILE))?;
    let id = root
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or("invalid session name")?;
    identity(&bytes, id, cache)?;
    let Some(lease) = lock(root)? else {
        return Ok(SessionReapOutcome::Active);
    };
    let records = directory(root.parent().ok_or("session has no catalog")?)?;
    let record = records.join(format!("{id}.json"));
    // Linking existing, synced identity avoids both partial record writes and replacement.
    fs::hard_link(root.join(SESSION_MARKER_FILE), &record).map_err(|error| {
        format!("cannot publish session retirement without replacement: {error}")
    })?;
    if regular_bytes(&record)? != bytes {
        return Err("session identity changed during retirement publication".into());
    }
    checkpoint("record-published", root)?;
    finish(root, &record, Some(lease))
}

pub(super) fn recover(
    sessions: &Path,
    cache: &Path,
    _catalog: &CatalogLease,
    blocked: &HashSet<PathBuf>,
) -> Result<Recovery, String> {
    let mut recovery = Recovery::default();
    for entry in fs::read_dir(directory(sessions)?).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let Some(id) = name
            .to_str()
            .and_then(|n| n.strip_suffix(".json"))
            .filter(|id| valid_id(id))
        else {
            recovery.report.retained_invalid_session_count += 1;
            continue;
        };
        let root = sessions.join(id);
        recovery.protected.insert(root.clone());
        let outcome = if blocked.contains(&root) {
            Err("retirement conflicts with incomplete initialization".into())
        } else {
            resume(&root, &entry.path(), id, cache)
        };
        match outcome {
            Ok(SessionReapOutcome::Removed) => recovery.report.removed_stale_session_count += 1,
            Ok(SessionReapOutcome::Active) => recovery.report.retained_active_session_count += 1,
            Err(_) => recovery.report.retained_invalid_session_count += 1,
        }
    }
    Ok(recovery)
}

fn resume(
    root: &Path,
    record: &Path,
    id: &str,
    cache: &Path,
) -> Result<SessionReapOutcome, String> {
    let bytes = regular_bytes(record)?;
    identity(&bytes, id, cache)?;
    if missing(root)? {
        fs::remove_file(record).map_err(|e| e.to_string())?;
        return Ok(SessionReapOutcome::Removed);
    }
    inspect_root(root)?;
    let marker_missing = missing(&root.join(SESSION_MARKER_FILE))?;
    let lease_missing = missing(&root.join(SESSION_LEASE_FILE))?;
    if (marker_missing || lease_missing) && !missing(&root.join(SESSION_GENERATIONS_DIRECTORY))? {
        return Err("retirement lost identity before payload removal".into());
    }
    if !marker_missing && regular_bytes(&root.join(SESSION_MARKER_FILE))? != bytes {
        return Err("retirement record conflicts with session marker".into());
    }
    let lease = if lease_missing {
        None
    } else {
        let Some(lease) = lock(root)? else {
            return Ok(SessionReapOutcome::Active);
        };
        Some(lease)
    };
    finish(root, record, lease)
}

fn finish(root: &Path, record: &Path, lease: Option<Lease>) -> Result<SessionReapOutcome, String> {
    let payload = root.join(SESSION_GENERATIONS_DIRECTORY);
    if !missing(&payload)? {
        // Only payload is recursive. Identity and its independent record survive any partial failure.
        let mut entries = fs::read_dir(&payload)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                fs::remove_dir_all(entry.path())
            } else {
                fs::remove_file(entry.path())
            }
            .map_err(|e| format!("cannot remove retired payload: {e}"))?;
            checkpoint("payload-entry-removed", root)?;
        }
        fs::remove_dir(&payload).map_err(|e| e.to_string())?;
    }
    checkpoint("payload-removed", root)?;
    drop(lease); // Windows requires the lease handle closed before unlink; catalog still held.
    for (name, stage) in [
        (SESSION_LEASE_FILE, "lease-removed"),
        (SESSION_MARKER_FILE, "marker-removed"),
    ] {
        let path = root.join(name);
        if !missing(&path)? {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        checkpoint(stage, root)?;
    }
    fs::remove_dir(root).map_err(|e| e.to_string())?;
    checkpoint("directory-removed", root)?;
    fs::remove_file(record).map_err(|e| e.to_string())?;
    Ok(SessionReapOutcome::Removed)
}

fn checkpoint(_stage: &str, _root: &Path) -> Result<(), String> {
    #[cfg(test)]
    super::retirement_tests::checkpoint(_stage, _root)?;
    Ok(())
}
