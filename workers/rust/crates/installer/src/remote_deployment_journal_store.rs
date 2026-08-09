use crate::{RemoteDeploymentJournal, RemoteDeploymentPlan, verify_remote_deployment_journal};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteDeploymentJournalPaths {
    pub journal: PathBuf,
    pub next: PathBuf,
    pub previous: PathBuf,
}

pub fn remote_deployment_journal_paths(path: &Path) -> RemoteDeploymentJournalPaths {
    RemoteDeploymentJournalPaths {
        journal: path.to_path_buf(),
        next: sidecar_path(path, ".next"),
        previous: sidecar_path(path, ".previous"),
    }
}

pub fn write_remote_deployment_journal_atomic(
    plan: &RemoteDeploymentPlan,
    journal: &RemoteDeploymentJournal,
    path: &Path,
) -> Result<(), String> {
    verify_remote_deployment_journal(plan, journal)?;
    let paths = remote_deployment_journal_paths(path);
    let parent = paths
        .journal
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create journal directory: {error}"))?;

    let mut payload = serde_json::to_vec_pretty(journal)
        .map_err(|error| format!("failed to encode remote deployment journal: {error}"))?;
    payload.push(b'\n');
    write_synced(&paths.next, &payload)?;
    remove_if_exists(&paths.previous)?;

    if paths.journal.exists() {
        fs::rename(&paths.journal, &paths.previous)
            .map_err(|error| format!("failed to stage previous journal: {error}"))?;
    }
    if let Err(error) = fs::rename(&paths.next, &paths.journal) {
        if !paths.journal.exists() && paths.previous.exists() {
            let _ = fs::rename(&paths.previous, &paths.journal);
        }
        return Err(format!(
            "failed to commit remote deployment journal: {error}"
        ));
    }
    sync_parent(parent)?;
    remove_if_exists(&paths.previous)?;
    Ok(())
}

pub fn read_remote_deployment_journal(
    plan: &RemoteDeploymentPlan,
    path: &Path,
) -> Result<RemoteDeploymentJournal, String> {
    let paths = remote_deployment_journal_paths(path);
    match read_verified(plan, &paths.journal) {
        Ok(journal) => {
            remove_if_exists(&paths.next)?;
            remove_if_exists(&paths.previous)?;
            Ok(journal)
        }
        Err(primary_error) => recover_previous(plan, &paths, primary_error),
    }
}

fn recover_previous(
    plan: &RemoteDeploymentPlan,
    paths: &RemoteDeploymentJournalPaths,
    primary_error: String,
) -> Result<RemoteDeploymentJournal, String> {
    let previous = read_verified(plan, &paths.previous).map_err(|previous_error| {
        format!(
            "remote deployment journal has no valid committed copy: primary={primary_error}; previous={previous_error}"
        )
    })?;
    remove_if_exists(&paths.journal)?;
    fs::rename(&paths.previous, &paths.journal)
        .map_err(|error| format!("failed to restore previous journal: {error}"))?;
    remove_if_exists(&paths.next)?;
    if let Some(parent) = paths.journal.parent() {
        sync_parent(parent)?;
    }
    Ok(previous)
}

fn read_verified(
    plan: &RemoteDeploymentPlan,
    path: &Path,
) -> Result<RemoteDeploymentJournal, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let journal = serde_json::from_slice::<RemoteDeploymentJournal>(&bytes)
        .map_err(|error| format!("failed to decode {}: {error}", path.display()))?;
    verify_remote_deployment_journal(plan, &journal)
        .map_err(|error| format!("failed to verify {}: {error}", path.display()))?;
    Ok(journal)
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("failed to sync {}: {error}", path.display()))
}

fn remove_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("failed to remove {}: {error}", path.display())),
    }
}

#[cfg(unix)]
fn sync_parent(parent: &Path) -> Result<(), String> {
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("failed to sync journal directory: {error}"))
}

#[cfg(not(unix))]
fn sync_parent(_parent: &Path) -> Result<(), String> {
    Ok(())
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value: OsString = path.as_os_str().to_owned();
    value.push(suffix);
    PathBuf::from(value)
}
