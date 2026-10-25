use kyuubiki_headless_sdk::DurableExecutionHandle;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn publish(
    out: &Path,
    input: &Value,
    handle: &DurableExecutionHandle,
    receipt: &Value,
    admission_available: bool,
) -> Result<()> {
    publish_with(out, input, handle, receipt, admission_available, |_| Ok(()))
}

// The hook is local to the example/test module, never a runtime fault-injection API.
pub(crate) fn publish_with(
    out: &Path,
    input: &Value,
    handle: &DurableExecutionHandle,
    receipt: &Value,
    admission_available: bool,
    after_stage: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    let published = out.join("verified");
    require_absent(&published)?;
    let result = handle.completed_result(receipt)?;
    super::checks::verify(input, result)?;
    let mut staging = Staging::create(out.join(".verification-pending"))?;
    let encoded = serde_json::to_vec_pretty(result)?;
    write(&staging.path, "result.json", &encoded)?;
    after_stage(&staging.path)?;
    let readback = super::evidence::read(&staging.path, "result.json")?;
    if readback.bytes != encoded {
        return Err("staged modal result changed before verification".into());
    }
    let mut report = super::checks::verify(input, &readback.value)?;
    report["execution"] = serde_json::to_value(handle)?;
    report["admission_response_available"] = json!(admission_available);
    report["recovery_basis"] = json!("prepared_input_and_original_store");
    report["result_sha256"] = json!(format!("{:x}", Sha256::digest(readback.bytes)));
    let verification = serde_json::to_vec_pretty(&report)?;
    write(&staging.path, "verification.json", &verification)?;
    if super::evidence::read(&staging.path, "verification.json")?.bytes != verification {
        return Err("staged modal verification changed before publication".into());
    }
    require_absent(&published)?;
    // One same-parent rename publishes the result and verification together.
    // This assumes a caller-owned, single-writer local directory, not power-loss durability.
    fs::rename(&staging.path, published)?;
    staging.cleanup = false;
    Ok(())
}

fn write(root: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    let mut file = fs::File::create_new(root.join(name))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn require_absent(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
        Ok(_) => Err("refusing to replace an existing verified modal export".into()),
    }
}

struct Staging {
    path: PathBuf,
    cleanup: bool,
}

impl Staging {
    fn create(path: PathBuf) -> Result<Self> {
        fs::create_dir(&path)?;
        Ok(Self {
            path,
            cleanup: true,
        })
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if self.cleanup {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
