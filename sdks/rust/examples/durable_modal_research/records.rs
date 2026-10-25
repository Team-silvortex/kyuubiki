use serde_json::Value;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn retain(root: &Path, name: &str, value: &Value) -> Result<()> {
    retain_with(root, name, value, |file, bytes| {
        file.write_all(bytes)?;
        Ok(())
    })
}

// A test-local writer can fail or pause during an actual write. The shipped
// example exposes no fault-injection environment variable or SDK storage API.
pub(crate) fn retain_with(
    root: &Path,
    name: &str,
    value: &Value,
    write: impl FnOnce(&mut fs::File, &[u8]) -> Result<()>,
) -> Result<()> {
    retain_inner(root, name, value, write).map_err(|error| format!("record {name}: {error}").into())
}

fn retain_inner(
    root: &Path,
    name: &str,
    value: &Value,
    write: impl FnOnce(&mut fs::File, &[u8]) -> Result<()>,
) -> Result<()> {
    if !matches!(
        name,
        "input.json" | "submission-intent.json" | "admission.json" | "receipt.json"
    ) {
        return Err("unsupported fixed evidence filename".into());
    }
    if !fs::symlink_metadata(root)?.is_dir() {
        return Err("record container must be a non-symlink directory".into());
    }
    let destination = root.join(name);
    match fs::symlink_metadata(&destination) {
        Ok(_) => return Err("refusing to replace existing evidence".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let encoded = serde_json::to_vec_pretty(value)?;
    if encoded.len() as u64 > super::evidence::MAX_BYTES {
        return Err("exceeds the example's 8 MiB record limit".into());
    }
    let pending_name = format!(".{name}.pending");
    let path = root.join(&pending_name);
    let mut file = fs::File::create_new(&path)?;
    let mut pending = Pending {
        path,
        cleanup: true,
    };
    let staged = write(&mut file, &encoded).and_then(|()| file.sync_all().map_err(Into::into));
    drop(file);
    staged?;
    if super::evidence::read(root, &pending_name)?.bytes != encoded {
        return Err("staged record changed before publication".into());
    }
    // Unlike rename, hard_link never replaces an existing destination. Complete
    // same-directory bytes become visible at once; unsupported filesystems fail closed.
    fs::hard_link(&pending.path, &destination)?;
    fs::remove_file(&pending.path)
        .map_err(|error| format!("record published, but pending alias cleanup failed: {error}"))?;
    pending.cleanup = false;
    Ok(())
}

struct Pending {
    path: PathBuf,
    cleanup: bool,
}
impl Drop for Pending {
    fn drop(&mut self) {
        if self.cleanup {
            let _ = fs::remove_file(&self.path);
        }
    }
}
