use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

pub(super) struct ManagedStateRoot {
    path: PathBuf,
    retained: bool,
}

impl ManagedStateRoot {
    pub(super) fn create(root: &Path, path: &Path) -> RunnerResult<Self> {
        ensure_state_root_scope(root, path)?;
        prepare_empty_root(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            retained: false,
        })
    }

    pub(super) fn retain(&mut self) {
        self.retained = true;
    }
}

impl Drop for ManagedStateRoot {
    fn drop(&mut self) {
        if !self.retained {
            let _ = remove_state_root_durable(&self.path);
        }
    }
}

pub(super) fn ensure_state_root_scope(root: &Path, state_root: &Path) -> RunnerResult<()> {
    let approved_root = root.join("tmp");
    fs::create_dir_all(&approved_root)
        .map_err(|error| format!("failed to create qualification tmp root: {error}"))?;
    let relative = state_root
        .strip_prefix(&approved_root)
        .map_err(|_| "host power-loss state root must be inside the repository tmp directory")?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(
            "host power-loss state root must name a child of the repository tmp directory".into(),
        );
    }
    let canonical_approved = fs::canonicalize(&approved_root)
        .map_err(|error| format!("failed to resolve qualification tmp root: {error}"))?;
    let existing = state_root
        .ancestors()
        .find(|candidate| candidate.exists())
        .ok_or("host power-loss state root has no existing ancestor")?;
    let canonical_existing = fs::canonicalize(existing)
        .map_err(|error| format!("failed to resolve state-root ancestor: {error}"))?;
    if !canonical_existing.starts_with(&canonical_approved) {
        return Err("host power-loss state root resolves outside repository tmp".into());
    }
    Ok(())
}

pub(super) fn remove_state_root_durable(path: &Path) -> RunnerResult<()> {
    let parent = path.parent().ok_or("state root has no parent")?;
    fs::remove_dir_all(path).map_err(|error| error.to_string())?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("failed to sync state-root deletion: {error}"))
}

fn prepare_empty_root(path: &Path) -> RunnerResult<()> {
    if path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err("host power-loss state root must not be a symlink".into());
    }
    if path.exists()
        && fs::read_dir(path)
            .map_err(|error| format!("failed to read state root: {error}"))?
            .next()
            .is_some()
    {
        return Err("host power-loss state root must be empty".into());
    }
    fs::create_dir_all(path).map_err(|error| format!("failed to create state root: {error}"))
}
