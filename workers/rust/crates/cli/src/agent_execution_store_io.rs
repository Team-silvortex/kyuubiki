use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

pub(super) fn open_private(path: &Path, create_new: bool) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    if create_new {
        options.create_new(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("execution store entry is not a regular file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o077 != 0 || metadata.nlink() != 1 {
            return Err(
                "execution store files require private permissions and a single link".into(),
            );
        }
    }
    Ok(file)
}

pub(super) fn prepare_root(root: &Path) -> Result<(), String> {
    if !root.is_absolute() {
        return Err(
            "execution store directory must be an absolute deployment-configured path".into(),
        );
    }
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(root) {
        Ok(()) => sync_directory(root.parent().ok_or("execution store has no parent")?)?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => {
            return Err(format!(
                "create execution store (parent must already exist): {e}"
            ));
        }
    }
    let metadata = fs::symlink_metadata(root).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("execution store directory must not be a symlink".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err("execution store directory requires owner-only permissions (0700)".into());
        }
    }
    Ok(())
}

pub(super) fn read<T: DeserializeOwned>(path: &Path, limit: usize) -> Result<Option<T>, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err("symlink in execution store".into());
        }
        Ok(_) => (),
    }
    let file = open_private(path, false)?;
    let mut bytes = Vec::new();
    file.take((limit + 66) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() < 65 || bytes.len() > limit + 65 || bytes[64] != b'\n' {
        return Err("execution record is truncated or exceeds its byte limit".into());
    }
    let digest = format!("{:x}", Sha256::digest(&bytes[65..]));
    if bytes[..64] != *digest.as_bytes() {
        return Err("execution record checksum mismatch".into());
    }
    serde_json::from_slice(&bytes[65..])
        .map(Some)
        .map_err(|e| e.to_string())
}

pub(super) fn write<T: Serialize + DeserializeOwned>(
    root: &Path,
    name: &str,
    value: &T,
    limit: usize,
) -> Result<u64, String> {
    let prepared = prepare(value, limit).map_err(|error| error.to_string())?;
    commit(root, name, prepared)
}

pub(super) struct PreparedRecord(Vec<u8>);

#[derive(Debug)]
pub(super) enum PreparationError {
    ByteLimit,
    Encoding(String),
    RestartDecode(String),
}

impl PreparationError {
    pub(super) fn reason(&self) -> &'static str {
        match self {
            Self::ByteLimit => "byte_limit",
            Self::Encoding(_) => "serialization",
            Self::RestartDecode(_) => "restart_decode",
        }
    }
}

impl std::fmt::Display for PreparationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ByteLimit => formatter.write_str("execution receipt exceeds byte limit"),
            Self::Encoding(message) | Self::RestartDecode(message) => formatter.write_str(message),
        }
    }
}

pub(super) fn prepare<T: Serialize + DeserializeOwned>(
    value: &T,
    limit: usize,
) -> Result<PreparedRecord, PreparationError> {
    let mut bytes = LimitedBuffer {
        bytes: Vec::new(),
        limit,
        exceeded: false,
    };
    if let Err(error) = serde_json::to_writer(&mut bytes, value) {
        return Err(if bytes.exceeded {
            PreparationError::ByteLimit
        } else {
            PreparationError::Encoding(error.to_string())
        });
    }
    // Do not acknowledge a payload which the restart reader cannot decode (for
    // example a deeply nested external-operator result beyond the parser limit).
    serde_json::from_slice::<T>(&bytes.bytes)
        .map_err(|error| PreparationError::RestartDecode(error.to_string()))?;
    Ok(PreparedRecord(bytes.bytes))
}

pub(super) fn commit(root: &Path, name: &str, prepared: PreparedRecord) -> Result<u64, String> {
    let bytes = prepared.0;
    let digest = format!("{:x}\n", Sha256::digest(&bytes));
    let path = root.join(name);
    if fs::symlink_metadata(&path).is_ok() {
        return Err("refusing to overwrite an immutable execution record".into());
    }
    let pending = root.join(format!("{name}.next"));
    let outcome = (|| {
        let mut file = open_private(&pending, true)?;
        file.write_all(digest.as_bytes())
            .and_then(|_| file.write_all(&bytes))
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&pending, &path).map_err(|e| e.to_string())?;
        sync_directory(root)?;
        Ok((bytes.len() + digest.len()) as u64)
    })();
    if outcome.is_err() {
        let _ = fs::remove_file(pending);
    }
    outcome
}

#[cfg(unix)]
pub(super) fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())
}

#[cfg(not(unix))]
pub(super) fn sync_directory(_path: &Path) -> Result<(), String> {
    // File sync and process-crash recovery only; the descriptor exposes this boundary.
    Ok(())
}

struct LimitedBuffer {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl Write for LimitedBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            self.exceeded = true;
            return Err(std::io::Error::other(
                "execution receipt exceeds byte limit",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
