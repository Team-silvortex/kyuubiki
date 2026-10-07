use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const OUTPUT_FAILURE: &str = "headless output failed:";
const BUFFER_BYTES: usize = 64 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn write_json_file<T: Serialize>(path: &str, value: &T) -> Result<PathBuf, String> {
    write_json_file_checked(path, value, || Ok(()))
}

pub(super) fn write_json_file_checked<T: Serialize>(
    path: &str,
    value: &T,
    validate: impl Fn() -> Result<(), String>,
) -> Result<PathBuf, String> {
    publish_checked_with(
        Path::new(path),
        |writer| serde_json::to_writer_pretty(writer, value).map_err(|error| error.to_string()),
        validate,
    )
}

pub(super) fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    let stdout = io::stdout();
    let mut writer = BufWriter::with_capacity(BUFFER_BYTES, stdout.lock());
    serde_json::to_writer_pretty(&mut writer, value)
        .map_err(|error| output_error("encode/write stdout", error))?;
    writer
        .write_all(b"\n")
        .and_then(|()| writer.flush())
        .map_err(|error| output_error("flush stdout", error))
}

#[cfg(test)]
fn publish_with(
    path: &Path,
    encode: impl FnOnce(&mut dyn Write) -> Result<(), String>,
) -> Result<PathBuf, String> {
    publish_checked_with(path, encode, || Ok(()))
}

fn publish_checked_with(
    path: &Path,
    encode: impl FnOnce(&mut dyn Write) -> Result<(), String>,
    validate: impl Fn() -> Result<(), String>,
) -> Result<PathBuf, String> {
    validate()?;
    if path.file_name().is_none() {
        return Err(output_error("select output file", path.display()));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| output_error("create output directory", error))?;
    check_destination(path)?;
    let mut pending = PendingFile::create(parent)?;
    {
        let mut writer = BufWriter::with_capacity(
            BUFFER_BYTES,
            pending.file.as_mut().expect("open pending output"),
        );
        encode(&mut writer)
            .map_err(|error| output_error(&format!("encode/write {}", path.display()), error))?;
        writer
            .flush()
            .map_err(|error| output_error("flush pending output", error))?;
    }
    pending
        .file
        .as_ref()
        .expect("open pending output")
        .sync_all()
        .map_err(|error| output_error("sync pending output", error))?;
    // Close before replacement and cleanup, including on platforms with open-file restrictions.
    drop(pending.file.take());
    validate()?;
    check_destination(path)?;
    fs::rename(&pending.path, path)
        .map_err(|error| output_error(&format!("publish {}", path.display()), error))?;
    pending.committed = true;
    Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
}

fn check_destination(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => Err(output_error(
            "select regular output file (symlinks and special files are unsupported)",
            path.display(),
        )),
        Ok(metadata) if metadata.permissions().readonly() => Err(output_error(
            "select writable output file (destination is read-only)",
            path.display(),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(output_error("inspect output file", error)),
    }
}

struct PendingFile {
    path: PathBuf,
    file: Option<File>,
    committed: bool,
}

impl PendingFile {
    fn create(parent: &Path) -> Result<Self, String> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for _ in 0..16 {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                ".kyuubiki-headless-output-{}-{stamp}-{sequence}.tmp",
                std::process::id()
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        path,
                        file: Some(file),
                        committed: false,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(output_error("create pending output", error)),
            }
        }
        Err(output_error(
            "create pending output",
            "name collisions exhausted",
        ))
    }
}

impl Drop for PendingFile {
    fn drop(&mut self) {
        drop(self.file.take());
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn output_error(operation: &str, error: impl std::fmt::Display) -> String {
    format!("{OUTPUT_FAILURE} {operation}: {error}")
}

#[cfg(test)]
#[path = "report_io_tests.rs"]
mod tests;
