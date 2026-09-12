use kyuubiki_platform::process_instance_token;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::runtime_support::remove_file_if_present;

const SCHEMA: &str = "kyuubiki.runtime-process/v1";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProcessRecord {
    schema_version: String,
    pub pid: u32,
    pub instance: String,
    label: String,
    port: Option<u16>,
    owner_directory: PathBuf,
}

pub(crate) enum Ownership {
    Absent,
    Stale,
    Owned(ProcessRecord),
}

pub(crate) fn record_path(pid_path: &Path) -> PathBuf {
    pid_path.with_extension("process.json")
}

pub(crate) fn read_pid(path: &Path) -> Option<u32> {
    let mut value = String::new();
    fs::File::open(path)
        .ok()?
        .take(65)
        .read_to_string(&mut value)
        .ok()?;
    if value.len() > 64 {
        return None;
    }
    value
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|pid| *pid > 0 && *pid <= i32::MAX as u32)
}

pub(crate) fn observe(
    pid_path: &Path,
    label: &str,
    port: Option<u16>,
) -> Result<Ownership, String> {
    if !pid_path.exists() {
        return if record_path(pid_path).exists() {
            Err(format!(
                "{label}: process record has no PID file; repair the runtime state before retrying"
            ))
        } else {
            Ok(Ownership::Absent)
        };
    }
    let pid = read_pid(pid_path).ok_or_else(|| format!("{label}: invalid PID record"))?;
    let Some(instance) = identity(pid)? else {
        return Ok(Ownership::Stale);
    };
    let path = record_path(pid_path);
    let record = (|| {
        let mut bytes = Vec::new();
        fs::File::open(&path)?
            .take(16 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 16 * 1024 {
            return Err(std::io::Error::other("process record is too large"));
        }
        serde_json::from_slice::<ProcessRecord>(&bytes).map_err(std::io::Error::other)
    })()
    .map_err(|error| {
        format!("{label}: unverified live process {pid}; refusing to adopt or stop it: {error}")
    })?;
    let owner = pid_path
        .parent()
        .ok_or("PID record has no owner directory")?
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if record.schema_version != SCHEMA
        || record.pid != pid
        || record.instance != instance
        || record.label != label
        || record.port != port
        || record.owner_directory != owner
        || pid == std::process::id()
    {
        return Err(format!(
            "{label}: process ownership mismatch for PID {pid}; refusing to adopt or stop it"
        ));
    }
    Ok(Ownership::Owned(record))
}

pub(crate) fn identity(pid: u32) -> Result<Option<String>, String> {
    process_instance_token(pid).map_err(|error| format!("cannot verify process {pid}: {error}"))
}

pub(crate) fn persist(
    pid_path: &Path,
    pid: u32,
    label: &str,
    port: Option<u16>,
) -> Result<(), String> {
    let instance = identity(pid)?
        .ok_or_else(|| format!("{label}: process {pid} exited before registration"))?;
    let record = ProcessRecord {
        schema_version: SCHEMA.to_string(),
        pid,
        instance,
        label: label.to_string(),
        port,
        owner_directory: pid_path
            .parent()
            .ok_or("PID record has no directory")?
            .canonicalize()
            .map_err(|error| error.to_string())?,
    };
    let bytes = serde_json::to_vec_pretty(&record).map_err(|error| error.to_string())?;
    write_new(pid_path, format!("{pid}\n").as_bytes())?;
    if let Err(error) = write_new(&record_path(pid_path), &bytes) {
        let _ = remove_file_if_present(pid_path);
        return Err(error);
    }
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = remove_file_if_present(path);
        return Err(format!("failed to persist {}: {error}", path.display()));
    }
    Ok(())
}

pub(crate) fn clear(pid_path: &Path) -> Result<(), String> {
    remove_file_if_present(&record_path(pid_path))?;
    remove_file_if_present(pid_path)
}
