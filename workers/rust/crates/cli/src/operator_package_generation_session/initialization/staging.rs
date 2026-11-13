//! Content-bound staging names permit prefix recovery, not authentication of local writers.
use super::*;
use sha2::{Digest, Sha256};

const SUFFIX: &str = ".v1.pending";

pub(super) fn is_staging(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(SUFFIX))
}

fn staging_name(id: &str, bytes: &[u8]) -> String {
    format!("{id}.{:x}{SUFFIX}", Sha256::digest(bytes))
}

fn owner_pid(id: &str) -> Result<u32, String> {
    let parts: Vec<_> = id.split('-').collect();
    if parts.len() != 4 || parts[0] != "session" {
        return Err("invalid initialization session ID".into());
    }
    let pid = parts[1].parse::<u32>().map_err(|error| error.to_string())?;
    let time = parts[2]
        .parse::<u128>()
        .map_err(|error| error.to_string())?;
    let sequence = parts[3].parse::<u64>().map_err(|error| error.to_string())?;
    if pid == 0 || id != format!("session-{pid}-{time}-{sequence}") {
        return Err("noncanonical initialization session ID".into());
    }
    Ok(pid)
}

fn expected_bytes(id: &str, cache: &Path) -> Result<Vec<u8>, String> {
    let intent = Intent {
        schema_version: SCHEMA.into(),
        session: SessionMarker {
            schema_version: SESSION_MARKER_SCHEMA.into(),
            session_id: id.into(),
            owner_pid: owner_pid(id)?,
            cache_store_root: cache.display().to_string(),
        },
    };
    let bytes = serde_json::to_vec_pretty(&intent).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_MARKER_BYTES {
        return Err("initialization record exceeds limit".into());
    }
    Ok(bytes)
}

pub(super) fn publish(
    directory: &Path,
    marker: &SessionMarker,
    bytes: &[u8],
    root: &Path,
) -> Result<PathBuf, String> {
    if bytes != expected_bytes(&marker.session_id, Path::new(&marker.cache_store_root))? {
        return Err("initialization staging identity is not reconstructible".into());
    }
    let destination = directory.join(format!("{}.json", marker.session_id));
    if !missing(&destination)? {
        return Err("session initialization record already exists".into());
    }
    let pending = directory.join(staging_name(&marker.session_id, bytes));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)
        .map_err(|error| format!("cannot create initialization staging: {error}"))?;
    checkpoint("intent-staging-created", root);
    #[cfg(test)]
    partial_checkpoint("intent-partial", root, &mut file, bytes)?;
    file.write_all(bytes)
        .and_then(|_| {
            #[cfg(test)]
            super::io_failures::before_staging_sync()?;
            file.sync_all()
        })
        .map_err(|error| format!("cannot persist initialization staging: {error}"))?;
    drop(file);
    checkpoint("intent-staging-synced", root);
    // Same-directory hard linking publishes complete bytes without replacing an existing name.
    publish_complete(&pending, &destination)?;
    checkpoint("intent-published", root);
    fs::remove_file(&pending)
        .map_err(|error| format!("cannot retire initialization staging: {error}"))?;
    Ok(destination)
}

fn publish_complete(pending: &Path, destination: &Path) -> Result<(), String> {
    fs::hard_link(pending, destination).map_err(|error| {
        format!("cannot publish initialization record without replacement: {error}")
    })
}

pub(super) fn associated_id(path: &Path) -> Option<&str> {
    let name = path.file_name()?.to_str()?.strip_suffix(SUFFIX)?;
    let (id, _) = name.rsplit_once('.')?;
    valid_session_id(id).then_some(id)
}

pub(super) fn recover(entries: &[fs::DirEntry], sessions: &Path, cache: &Path) -> Recovery {
    let mut recovery = Recovery::default();
    for entry in entries.iter().filter(|entry| is_staging(&entry.path())) {
        let path = entry.path();
        if reap_staging(&path, sessions, cache).is_ok() {
            recovery.report.removed_initialization_staging_count += 1;
        } else {
            recovery.report.retained_invalid_session_count += 1;
            if let Some(id) = associated_id(&path) {
                recovery.protected.insert(sessions.join(id));
            }
        }
    }
    recovery
}

fn reap_staging(path: &Path, sessions: &Path, cache: &Path) -> Result<(), String> {
    let id = associated_id(path).ok_or("unknown initialization staging name")?;
    let expected = expected_bytes(id, cache)?;
    if path.file_name().and_then(|name| name.to_str()) != Some(&staging_name(id, &expected)) {
        return Err("initialization staging commitment mismatch".into());
    }
    let bytes = read_regular(path)?;
    if !expected.starts_with(&bytes) {
        return Err("initialization staging prefix mismatch".into());
    }
    let destination = path
        .parent()
        .ok_or("initialization staging has no parent")?
        .join(format!("{id}.json"));
    if missing(&destination)? {
        if !missing(&sessions.join(id))? {
            return Err("unpublished staging has an unexpected session".into());
        }
    } else if bytes != expected || read_regular(&destination)? != expected {
        return Err("initialization staging conflicts with a published record".into());
    }
    // Only this staging name is removed. No session or final record is deleted here.
    fs::remove_file(path).map_err(|error| error.to_string())?;
    checkpoint("intent-staging-removed", &sessions.join(id));
    Ok(())
}

#[cfg(test)]
mod tests;
