use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

const PREFIX: &str = "kyuubiki.agent-execution-store/v1\n";
const LIMIT: u64 = 160;

// The caller holds the exclusive owner.lock lock. An incomplete nonempty identity
// is an error, never permission to replace an already published owner identity.
pub(super) fn load_or_create(lock: &mut File, root: &Path) -> Result<String, String> {
    if let Some(id) = read_identity(lock)? {
        Ok(id)
    } else {
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).map_err(|e| e.to_string())?;
        let id = nonce
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let payload = format!("{PREFIX}{id}\n");
        let data = format!("{payload}{:x}\n", Sha256::digest(payload.as_bytes()));
        lock.write_all(data.as_bytes())
            .and_then(|_| lock.sync_all())
            .map_err(|e| e.to_string())?;
        super::io::sync_directory(root)?;
        Ok(id)
    }
}

#[cfg(unix)]
pub(super) fn check_binding(lock: &File, root: &Path, expected: &str) -> Result<(), String> {
    let check = || {
        super::io::validate_root(root)?;
        let mut current = super::io::open_private(&root.join("owner.lock"), false)?;
        {
            use std::os::unix::fs::MetadataExt;
            let held = lock.metadata().map_err(|e| e.to_string())?;
            let path = current.metadata().map_err(|e| e.to_string())?;
            if (held.dev(), held.ino()) != (path.dev(), path.ino()) {
                return Err("owner.lock no longer names the locked file".into());
            }
        }
        if read_identity(&mut current)?.as_deref() != Some(expected) {
            return Err("owner identity is missing or changed".into());
        }
        Ok(())
    };
    check().map_err(|error: String| format!("execution store binding unavailable: {error}"))
}

#[cfg(not(unix))]
pub(super) fn check_binding(_lock: &File, root: &Path, _expected: &str) -> Result<(), String> {
    // Do not reopen/read a byte-range-locked Windows file as if it were Unix flock.
    // Native handle identity validation is not yet qualified on this platform.
    super::io::validate_root(root)
        .map_err(|error| format!("execution store binding unavailable: {error}"))
}

fn read_identity(file: &mut File) -> Result<Option<String>, String> {
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| "invalid execution store identity")?;
    let fields: Vec<_> = text.split('\n').collect();
    if bytes.len() > LIMIT as usize
        || fields.len() != 4
        || !text.starts_with(PREFIX)
        || kyuubiki_protocol::validate_durable_execution_id(fields[1]).is_err()
        || !fields[3].is_empty()
        || fields[2]
            != format!(
                "{:x}",
                Sha256::digest(format!("{PREFIX}{}\n", fields[1]).as_bytes())
            )
    {
        return Err("invalid execution store identity; refusing to replace owner identity".into());
    }
    Ok(Some(fields[1].into()))
}
