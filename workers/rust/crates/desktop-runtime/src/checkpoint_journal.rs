use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const FILE: &str = "desktop-checkpoint-recovery.json";
const MAX_ENTRIES: usize = 64;
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CheckpointIntent {
    pub key: String,
    pub scope: String,
    pub operation: String,
    pub parent_id: String,
    pub request_id: String,
    pub created_at: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u8,
    pending: Vec<CheckpointIntent>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointReservation {
    pub intent: CheckpointIntent,
    pub is_new: bool,
}

impl CheckpointIntent {
    fn validate(&self) -> Result<(), String> {
        let hash = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
        };
        if !hash(&self.key)
            || !hash(&self.scope)
            || !matches!(self.operation.as_str(), "model" | "version")
            || self.parent_id.is_empty()
            || self.parent_id.len() > 128
            || !valid_request_id(&self.request_id)
            || self.created_at > 8_640_000_000_000_000
        {
            return Err("checkpoint:journal_corrupt".into());
        }
        Ok(())
    }
}

fn valid_request_id(value: &str) -> bool {
    (16..=128).contains(&value.len())
        && value
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || v == b'-' || v == b'_')
}

pub fn list_checkpoint_intents() -> Result<Vec<CheckpointIntent>, String> {
    Store::open(super::desktop_preferences_dir()?)?.read()
}

pub fn reserve_checkpoint_intent(
    intent: CheckpointIntent,
) -> Result<CheckpointReservation, String> {
    intent.validate()?;
    Store::open(super::desktop_preferences_dir()?)?.reserve(intent)
}

pub fn remove_checkpoint_intent(key: String, request_id: String) -> Result<(), String> {
    if key.len() != 64 || !valid_request_id(&request_id) {
        return Err("checkpoint:invalid_recovery_key".into());
    }
    Store::open(super::desktop_preferences_dir()?)?.remove(&key, &request_id)
}

struct Store {
    root: PathBuf,
    _lock: File,
}

fn regular(path: &Path, directory: bool) -> Result<(), String> {
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
        _ => Err("checkpoint:journal_unsafe_path".into()),
    }
}

fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    options
}

impl Store {
    fn open(root: PathBuf) -> Result<Self, String> {
        regular(&root, true)?;
        fs::create_dir_all(&root).map_err(|_| "checkpoint:journal_unavailable")?;
        let lock_path = root.join("desktop-checkpoint-recovery.lock");
        regular(&lock_path, false)?;
        let file = private_options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|_| "checkpoint:journal_unavailable")?;
        if !file
            .metadata()
            .map_err(|_| "checkpoint:journal_unavailable")?
            .is_file()
        {
            return Err("checkpoint:journal_unsafe_path".into());
        }
        file.try_lock_exclusive()
            .map_err(|_| "checkpoint:journal_busy")?;
        Ok(Self { root, _lock: file })
    }

    fn read(&self) -> Result<Vec<CheckpointIntent>, String> {
        let path = self.root.join(FILE);
        regular(&path, false)?;
        let file = match private_options().read(true).open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err("checkpoint:journal_unavailable".into()),
        };
        if !file
            .metadata()
            .map_err(|_| "checkpoint:journal_unavailable")?
            .is_file()
        {
            return Err("checkpoint:journal_unsafe_path".into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "checkpoint:journal_unavailable")?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("checkpoint:journal_corrupt".into());
        }
        let journal: Journal =
            serde_json::from_slice(&bytes).map_err(|_| "checkpoint:journal_corrupt")?;
        if journal.version != 1 || journal.pending.len() > MAX_ENTRIES {
            return Err("checkpoint:journal_corrupt".into());
        }
        let mut keys = HashSet::new();
        for intent in &journal.pending {
            intent.validate()?;
            if !keys.insert(&intent.key) {
                return Err("checkpoint:journal_corrupt".into());
            }
        }
        Ok(journal.pending)
    }

    fn reserve(&self, intent: CheckpointIntent) -> Result<CheckpointReservation, String> {
        intent.validate()?;
        let mut pending = self.read()?;
        if let Some(existing) = pending.iter().find(|entry| entry.key == intent.key) {
            if existing.scope != intent.scope
                || existing.operation != intent.operation
                || existing.parent_id != intent.parent_id
            {
                return Err("checkpoint:journal_corrupt".into());
            }
            return Ok(CheckpointReservation {
                intent: existing.clone(),
                is_new: false,
            });
        }
        if pending.len() >= MAX_ENTRIES {
            return Err("checkpoint:recovery_capacity_reached".into());
        }
        pending.push(intent.clone());
        self.write(pending)?;
        Ok(CheckpointReservation {
            intent,
            is_new: true,
        })
    }

    fn remove(&self, key: &str, id: &str) -> Result<(), String> {
        let mut pending = self.read()?;
        let count = pending.len();
        pending.retain(|entry| entry.key != key || entry.request_id != id);
        if pending.len() != count {
            self.write(pending)?;
        }
        Ok(())
    }

    fn write(&self, pending: Vec<CheckpointIntent>) -> Result<(), String> {
        let path = self.root.join(FILE);
        regular(&path, false)?;
        // A single locked staging name bounds residue even if the process dies.
        let staging = self.root.join("desktop-checkpoint-recovery.pending");
        regular(&staging, false)?;
        match fs::remove_file(&staging) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err("checkpoint:journal_unavailable".into()),
        }
        let bytes = serde_json::to_vec(&Journal {
            version: 1,
            pending,
        })
        .map_err(|_| "checkpoint:journal_corrupt")?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("checkpoint:journal_corrupt".into());
        }
        let result = (|| {
            let mut file = private_options()
                .write(true)
                .create_new(true)
                .open(&staging)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&staging, path)?;
            #[cfg(unix)]
            File::open(&self.root)?.sync_all()?;
            Ok::<_, std::io::Error>(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(staging);
        }
        result.map_err(|_| "checkpoint:journal_unavailable".into())
    }
}

#[cfg(test)]
#[path = "checkpoint_journal_tests.rs"]
mod tests;
