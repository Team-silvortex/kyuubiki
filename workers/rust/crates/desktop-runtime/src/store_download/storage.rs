use super::{Result, types::*};
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(super) fn regular(path: &Path, directory: bool) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta)
            if !meta.file_type().is_symlink()
                && if directory {
                    meta.is_dir()
                } else {
                    meta.is_file()
                } =>
        {
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err("unsafe_download_storage"),
    }
}
fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000);
    }
    options
}
pub(super) fn directory(path: &Path) -> Result<()> {
    regular(path, true)?;
    if !path.exists() {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(path)
            .map_err(|_| "download_storage_unavailable")?;
        if let Some(parent) = path.parent() {
            sync_dir(parent)?;
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = fs::metadata(path).map_err(|_| "download_storage_unavailable")?;
        if meta.mode() & 0o077 != 0 || meta.uid() != unsafe { libc::geteuid() } {
            return Err("unsafe_download_storage");
        }
    }
    Ok(())
}
pub(super) struct Store {
    pub root: PathBuf,
    _lock: File,
}
impl Store {
    pub fn open(root: PathBuf) -> Result<Self> {
        directory(&root)?;
        let lock = root.join("writer.lock");
        regular(&lock, false)?;
        let lock = options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock)
            .map_err(|_| "download_storage_unavailable")?;
        if !lock
            .metadata()
            .map_err(|_| "download_storage_unavailable")?
            .is_file()
        {
            return Err("unsafe_download_storage");
        }
        lock.try_lock_exclusive().map_err(|_| "download_busy")?;
        Ok(Self { root, _lock: lock })
    }
    pub fn path(&self, id: &str) -> Result<PathBuf> {
        if !digest(id) {
            return Err("invalid_download_id");
        }
        let path = self.root.join(id);
        regular(&path, true)?;
        Ok(path)
    }
    pub fn read(&self, id: &str) -> Result<Record> {
        read(&self.root, id)
    }
    pub fn save(&self, record: &Record) -> Result<()> {
        record.validate()?;
        let path = self.path(&record.id)?;
        directory(&path)?;
        let target = path.join("record.json");
        regular(&target, false)?;
        let staging = path.join("record.next");
        regular(&staging, false)?;
        if staging.exists() {
            fs::remove_file(&staging).map_err(|_| "download_storage_unavailable")?;
        }
        let bytes = serde_json::to_vec(record).map_err(|_| "download_record_invalid")?;
        if bytes.len() > 262144 {
            return Err("download_record_invalid");
        }
        let mut file = options()
            .write(true)
            .create_new(true)
            .open(&staging)
            .map_err(|_| "download_storage_unavailable")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "download_storage_unavailable")?;
        drop(file);
        fs::rename(&staging, &target).map_err(|_| "download_storage_unavailable")?;
        sync_dir(&path)?;
        sync_dir(&self.root)
    }
    pub fn reset_partial(&self, id: &str) -> Result<PathBuf> {
        let parent = self.path(id)?;
        // A failed receipt commit may leave the prior staged files; do not keep two copies on retry.
        for name in ["partial", "files"] {
            let path = parent.join(name);
            regular(&path, true)?;
            if path.exists() {
                fs::remove_dir_all(path).map_err(|_| "download_storage_unavailable")?;
            }
        }
        let path = parent.join("partial");
        directory(&path)?;
        Ok(path)
    }
    pub fn clear_partial(&self, id: &str) -> Result<()> {
        let path = self.path(id)?.join("partial");
        regular(&path, true)?;
        if path.exists() {
            fs::remove_dir_all(path).map_err(|_| "download_storage_unavailable")?;
        }
        Ok(())
    }
    pub fn create_file(&self, base: &Path, relative: &str) -> Result<File> {
        let mut path = base.to_path_buf();
        let parts: Vec<_> = relative.split('/').collect();
        for (index, part) in parts.iter().enumerate() {
            if !segment(part) {
                return Err("artifact_mismatch");
            }
            path.push(part);
            if index + 1 != parts.len() {
                directory(&path)?;
            }
        }
        options()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| "download_storage_unavailable")
    }
    pub fn promote(&self, record: &Record) -> Result<()> {
        let parent = self.path(&record.id)?;
        let target = parent.join("files");
        regular(&target, true)?;
        // Recovery after rename-before-receipt crash replaces only this operation's owned files.
        if target.exists() {
            fs::remove_dir_all(&target).map_err(|_| "download_storage_unavailable")?;
        }
        fs::rename(parent.join("partial"), target).map_err(|_| "download_storage_unavailable")?;
        sync_dir(&parent)
    }
    pub fn remove(&self, record: &Record) -> Result<()> {
        if record.attempted && record.outcome.is_none() {
            return Err("authorization_outcome_unknown");
        }
        fs::remove_dir_all(self.path(&record.id)?).map_err(|_| "download_storage_unavailable")?;
        sync_dir(&self.root)
    }
}
pub(super) fn sync_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| "download_storage_unavailable")?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
pub(super) fn read(root: &Path, id: &str) -> Result<Record> {
    if !digest(id) {
        return Err("invalid_download_id");
    }
    regular(root, true)?;
    let parent = root.join(id);
    regular(&parent, true)?;
    let path = parent.join("record.json");
    regular(&path, false)?;
    let file = options()
        .read(true)
        .open(path)
        .map_err(|_| "download_record_unavailable")?;
    if !file
        .metadata()
        .map_err(|_| "download_storage_unavailable")?
        .is_file()
    {
        return Err("unsafe_download_storage");
    }
    let mut bytes = Vec::new();
    file.take(262145)
        .read_to_end(&mut bytes)
        .map_err(|_| "download_record_unavailable")?;
    if bytes.len() > 262144 {
        return Err("download_record_invalid");
    }
    let record: Record = serde_json::from_slice(&bytes).map_err(|_| "download_record_invalid")?;
    record.validate()?;
    if record.id != id {
        return Err("download_record_invalid");
    }
    Ok(record)
}
pub(super) fn list(root: &Path) -> Result<Vec<Record>> {
    regular(root, true)?;
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    for entry in fs::read_dir(root).map_err(|_| "download_storage_unavailable")? {
        let entry = entry.map_err(|_| "download_storage_unavailable")?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "unsafe_download_storage")?;
        if name == "writer.lock" {
            continue;
        }
        if records.len() >= 4 {
            return Err("download_capacity_reached");
        }
        records.push(read(root, &name)?);
    }
    records.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(records)
}
