use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const FILE: &str = "desktop-direct-execution.json";
const LIMIT: usize = 64;
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirectExecutionIntent {
    pub scope: String,
    pub endpoint: String,
    pub study_kind: String,
    pub execution_id: String,
    pub created_at: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u8,
    pending: Vec<DirectExecutionIntent>,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum DirectExecutionJournalRequest {
    List {},
    Reserve { intent: DirectExecutionIntent },
    Remove { intent: DirectExecutionIntent },
}

fn hash(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
}

impl DirectExecutionIntent {
    fn validate(&self) -> Result<(), String> {
        let endpoint = crate::direct_mesh_rpc::parse_endpoint(&self.endpoint);
        if !hash(&self.scope, 64)
            || !hash(&self.execution_id, 32)
            || !endpoint.is_ok_and(|value| value.endpoint == self.endpoint)
            || self.endpoint.len() > 255
            || crate::direct_mesh_gateway::method_for_study(&self.study_kind).is_none()
            || self.created_at > 8_640_000_000_000_000
        {
            return Err("direct:journal_corrupt".into());
        }
        Ok(())
    }
}

pub fn direct_execution_journal(
    request: DirectExecutionJournalRequest,
) -> Result<serde_json::Value, String> {
    Store::open(super::desktop_preferences_dir()?)?.handle(request)
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
        _ => Err("direct:journal_unsafe_path".into()),
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
        fs::create_dir_all(&root).map_err(|_| "direct:journal_unavailable")?;
        let lock = root.join("desktop-direct-execution.lock");
        regular(&lock, false)?;
        let file = private_options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock)
            .map_err(|_| "direct:journal_unavailable")?;
        if !file
            .metadata()
            .map_err(|_| "direct:journal_unavailable")?
            .is_file()
        {
            return Err("direct:journal_unsafe_path".into());
        }
        file.try_lock_exclusive()
            .map_err(|_| "direct:journal_busy")?;
        Ok(Self { root, _lock: file })
    }

    fn read(&self) -> Result<Vec<DirectExecutionIntent>, String> {
        let path = self.root.join(FILE);
        regular(&path, false)?;
        let file = match private_options().read(true).open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(_) => return Err("direct:journal_unavailable".into()),
        };
        if !file
            .metadata()
            .map_err(|_| "direct:journal_unavailable")?
            .is_file()
        {
            return Err("direct:journal_unsafe_path".into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "direct:journal_unavailable")?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("direct:journal_corrupt".into());
        }
        let journal: Journal =
            serde_json::from_slice(&bytes).map_err(|_| "direct:journal_corrupt")?;
        if journal.version != 1 || journal.pending.len() > LIMIT {
            return Err("direct:journal_corrupt".into());
        }
        let mut keys = HashSet::new();
        let mut ids = HashSet::new();
        for entry in &journal.pending {
            entry.validate()?;
            if !keys.insert((&entry.scope, &entry.endpoint)) || !ids.insert(&entry.execution_id) {
                return Err("direct:journal_corrupt".into());
            }
        }
        Ok(journal.pending)
    }

    fn handle(&self, request: DirectExecutionJournalRequest) -> Result<serde_json::Value, String> {
        let mut entries = self.read()?;
        match request {
            DirectExecutionJournalRequest::List {} => Ok(serde_json::json!(entries)),
            DirectExecutionJournalRequest::Reserve { intent } => {
                intent.validate()?;
                if let Some(existing) = entries
                    .iter()
                    .find(|e| e.scope == intent.scope && e.endpoint == intent.endpoint)
                {
                    return Ok(serde_json::json!({"intent":existing,"is_new":false}));
                }
                if entries
                    .iter()
                    .any(|e| e.execution_id == intent.execution_id)
                {
                    return Err("direct:journal_corrupt".into());
                }
                if entries.len() >= LIMIT {
                    return Err("direct:journal_full".into());
                }
                entries.push(intent.clone());
                self.write(entries)?;
                Ok(serde_json::json!({"intent":intent,"is_new":true}))
            }
            DirectExecutionJournalRequest::Remove { intent } => {
                intent.validate()?;
                let count = entries.len();
                entries.retain(|e| e != &intent);
                if count != entries.len() {
                    self.write(entries)?;
                }
                Ok(serde_json::Value::Null)
            }
        }
    }

    fn write(&self, pending: Vec<DirectExecutionIntent>) -> Result<(), String> {
        let path = self.root.join(FILE);
        let staging = self.root.join("desktop-direct-execution.pending");
        regular(&path, false)?;
        regular(&staging, false)?;
        match fs::remove_file(&staging) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err("direct:journal_unavailable".into()),
        }
        let bytes = serde_json::to_vec(&Journal {
            version: 1,
            pending,
        })
        .map_err(|_| "direct:journal_corrupt")?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("direct:journal_full".into());
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
        result.map_err(|_| "direct:journal_unavailable".into())
    }
}

#[cfg(test)]
#[path = "direct_execution_journal_tests.rs"]
mod tests;
