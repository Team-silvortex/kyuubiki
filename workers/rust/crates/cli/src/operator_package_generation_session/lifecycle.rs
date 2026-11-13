//! Shared entry checks; callers hold catalog coordination through their protected operation.
use super::*;

pub(super) fn record_directory(sessions: &Path, name: &str) -> Result<Option<PathBuf>, String> {
    let parent = sessions.parent().ok_or("session catalog has no parent")?;
    let path = parent.join(name);
    match path.symlink_metadata() {
        Ok(metadata) if metadata.is_dir() => Ok(Some(path)),
        Ok(_) => Err(format!("session {name} must be a real directory")),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("cannot inspect session {name}: {error}")),
    }
}

pub(super) fn ensure_ready(
    sessions: &Path,
    id: &str,
    _catalog: &CatalogLease,
) -> Result<(), String> {
    if initialization::has_pending_records(sessions, id)? {
        return Err(
            "operator session has unresolved initialization records; retry only after resolution"
                .into(),
        );
    }
    if retirement::has_record(sessions, id)? {
        return Err(
            "operator session is retiring; new preparation or attachment is forbidden".into(),
        );
    }
    Ok(())
}
