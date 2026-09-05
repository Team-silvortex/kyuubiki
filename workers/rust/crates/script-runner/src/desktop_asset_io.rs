use std::fs;
use std::path::Path;

use crate::RunnerResult;

#[derive(Default, Debug)]
pub(crate) struct SyncStats {
    pub checked: usize,
    pub written: usize,
    pub removed: usize,
}

pub(crate) fn write_changed(
    target: &Path,
    expected: &[u8],
    stats: &mut SyncStats,
) -> RunnerResult<()> {
    stats.checked += 1;
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.is_file() => {
            if metadata.len() == expected.len() as u64
                && fs::read(target)
                    .map_err(|error| format!("read {}: {error}", target.display()))?
                    == expected
            {
                return Ok(());
            }
        }
        Ok(_) => {
            return Err(format!(
                "asset target is not a regular file: {}",
                target.display()
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("inspect {}: {error}", target.display())),
    }
    let parent = target
        .parent()
        .ok_or_else(|| format!("asset has no parent: {}", target.display()))?;
    ensure_directory(parent)?;
    fs::write(target, expected).map_err(|error| format!("write {}: {error}", target.display()))?;
    stats.written += 1;
    Ok(())
}

pub(crate) fn copy_changed(
    source: &Path,
    target: &Path,
    stats: &mut SyncStats,
) -> RunnerResult<()> {
    if !fs::symlink_metadata(source)
        .map_err(|error| format!("inspect {}: {error}", source.display()))?
        .is_file()
    {
        return Err(format!(
            "asset source is not a regular file: {}",
            source.display()
        ));
    }
    let bytes = fs::read(source).map_err(|error| format!("read {}: {error}", source.display()))?;
    write_changed(target, &bytes, stats)
}

pub(crate) fn mirror_tree(source: &Path, target: &Path, stats: &mut SyncStats) -> RunnerResult<()> {
    if !fs::symlink_metadata(source)
        .map_err(|error| format!("inspect {}: {error}", source.display()))?
        .is_dir()
    {
        return Err(format!(
            "asset source is not a directory: {}",
            source.display()
        ));
    }
    ensure_directory(target)?;
    let mut expected = Vec::new();
    for entry in
        fs::read_dir(source).map_err(|error| format!("read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("read {} entry: {error}", source.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("inspect asset: {error}"))?;
        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            mirror_tree(&entry.path(), &destination, stats)?;
        } else if file_type.is_file() {
            copy_changed(&entry.path(), &destination, stats)?;
        } else {
            return Err(format!(
                "unsupported asset source: {}",
                entry.path().display()
            ));
        }
        expected.push(entry.file_name());
    }
    prune_entries(target, &expected, stats)
}

pub(crate) fn prune_entries(
    target: &Path,
    expected: &[std::ffi::OsString],
    stats: &mut SyncStats,
) -> RunnerResult<()> {
    for entry in
        fs::read_dir(target).map_err(|error| format!("read {}: {error}", target.display()))?
    {
        let entry = entry.map_err(|error| format!("read {} entry: {error}", target.display()))?;
        if !expected.contains(&entry.file_name()) {
            let file_type = entry
                .file_type()
                .map_err(|error| format!("inspect asset: {error}"))?;
            if file_type.is_dir() {
                fs::remove_dir_all(entry.path())
            } else if file_type.is_file() {
                fs::remove_file(entry.path())
            } else {
                return Err(format!(
                    "unsupported stale asset: {}",
                    entry.path().display()
                ));
            }
            .map_err(|error| format!("remove stale asset {}: {error}", entry.path().display()))?;
            stats.removed += 1;
        }
    }
    Ok(())
}

pub(crate) fn ensure_directory(path: &Path) -> RunnerResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(format!(
            "asset directory is not a real directory: {}",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                ensure_directory(parent)?;
            }
            fs::create_dir(path).map_err(|error| format!("create {}: {error}", path.display()))
        }
        Err(error) => Err(format!("inspect {}: {error}", path.display())),
    }
}
