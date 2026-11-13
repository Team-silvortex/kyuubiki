//! Keep generation identity outside the directory until its final unlink succeeds.
use super::*;
use std::io::ErrorKind;

fn missing(path: &Path) -> Result<bool, String> {
    match path.symlink_metadata() {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(true),
        Err(error) => Err(format!("cannot inspect generation cleanup path: {error}")),
    }
}

fn validate_identity(path: &Path, expected: &GenerationMarker) -> Result<(), String> {
    if read_marker_file(path)? != *expected {
        return Err("owned operator package generation marker mismatch".into());
    }
    Ok(())
}

pub(super) fn record_path(root: &Path) -> Result<PathBuf, String> {
    let parent = root.parent().ok_or("owned generation has no parent")?;
    let id = root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid generation name")?;
    Ok(parent.join(format!(".kyuubiki-retired-{id}.json")))
}

pub(super) fn ensure_download_ready(root: &Path) -> Result<(), String> {
    if !missing(&record_path(root)?)? {
        return Err("operator generation is retiring; downloader attachment is forbidden".into());
    }
    Ok(())
}

pub(super) fn remove(generation: &OwnedOperatorPackageGeneration) -> Result<(), String> {
    let root = &generation.generation_root;
    let parent = root.parent().ok_or("owned generation has no parent")?;
    if canonical_directory(parent, "operator package generations root")?
        != generation.generations_root
        || root.file_name().and_then(|name| name.to_str())
            != Some(generation.marker.generation_id.as_str())
    {
        return Err("owned operator package generation path identity mismatch".into());
    }
    let record = record_path(root)?;
    let recorded = !missing(&record)?;
    if recorded {
        validate_identity(&record, &generation.marker)?;
    }
    if missing(root)? {
        if recorded {
            fs::remove_file(&record).map_err(|error| error.to_string())?;
        }
        return Ok(());
    }
    if !root.symlink_metadata().map_err(|e| e.to_string())?.is_dir() {
        return Err("owned operator package generation must be a real directory".into());
    }
    let marker = root.join(GENERATION_MARKER_FILE);
    let marker_missing = missing(&marker)?;
    if !marker_missing {
        validate_identity(&marker, &generation.marker)?;
    } else if !recorded
        || fs::read_dir(root)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
    {
        return Err("generation lost identity before payload removal".into());
    }
    if !recorded {
        // No replacement, partial JSON write, or loss of identity during root deletion.
        fs::hard_link(&marker, &record)
            .map_err(|error| format!("cannot publish generation retirement: {error}"))?;
        validate_identity(&record, &generation.marker)?;
    }
    checkpoint("record-published", root)?;
    let mut entries = fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if entry.file_name() == GENERATION_MARKER_FILE {
            continue;
        }
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            fs::remove_dir_all(entry.path())
        } else {
            fs::remove_file(entry.path())
        }
        .map_err(|error| format!("cannot remove retired generation payload: {error}"))?;
        checkpoint("payload-entry-removed", root)?;
    }
    checkpoint("payload-removed", root)?;
    if !marker_missing {
        fs::remove_file(&marker).map_err(|error| error.to_string())?;
    }
    checkpoint("marker-removed", root)?;
    fs::remove_dir(root).map_err(|error| error.to_string())?;
    checkpoint("directory-removed", root)?;
    fs::remove_file(record).map_err(|error| error.to_string())
}

fn checkpoint(_stage: &str, _root: &Path) -> Result<(), String> {
    #[cfg(test)]
    super::cleanup_tests::checkpoint(_stage, _root)?;
    Ok(())
}
