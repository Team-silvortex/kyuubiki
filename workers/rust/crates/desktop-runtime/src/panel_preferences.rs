use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

const FILE_NAME: &str = "desktop-workbench-layout.json";
const MAX_BYTES: u64 = 1024;
static NEXT_WRITE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkbenchPanelLayout {
    pub version: u8,
    pub sizes: WorkbenchPanelSizes,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkbenchPanelSizes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sidebar: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspector: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<f64>,
}

impl WorkbenchPanelLayout {
    fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("unsupported Workbench panel layout version".into());
        }
        for value in [self.sizes.sidebar, self.sizes.inspector, self.sizes.report]
            .into_iter()
            .flatten()
        {
            if !value.is_finite() || value <= 0.0 || value > 5000.0 {
                return Err("Workbench panel size must be finite and within (0, 5000]".into());
            }
        }
        Ok(())
    }
}

pub fn read_workbench_panel_layout() -> Result<Option<WorkbenchPanelLayout>, String> {
    read_at(&super::desktop_preferences_dir()?.join(FILE_NAME))
}

pub fn write_workbench_panel_layout(
    layout: WorkbenchPanelLayout,
) -> Result<WorkbenchPanelLayout, String> {
    write_at(&super::desktop_preferences_dir()?.join(FILE_NAME), &layout)?;
    Ok(layout)
}

fn reject_link_or_special(path: &Path, directory: bool) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if (directory && meta.is_dir()) || (!directory && meta.is_file()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => {
            Err("Workbench layout storage must use a regular file and directory, not a link".into())
        }
    }
}

fn read_at(path: &Path) -> Result<Option<WorkbenchPanelLayout>, String> {
    if let Some(parent) = path.parent() {
        reject_link_or_special(parent, true)?;
    }
    reject_link_or_special(path, false)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot open Workbench layout: {error}")),
    };
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Workbench layout is not a regular file".into());
    }
    let mut raw = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() as u64 > MAX_BYTES {
        return Err("Workbench layout exceeds 1024 bytes".into());
    }
    let layout: WorkbenchPanelLayout = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    layout.validate()?;
    Ok(Some(layout))
}

fn write_at(path: &Path, layout: &WorkbenchPanelLayout) -> Result<(), String> {
    layout.validate()?;
    let parent = path
        .parent()
        .ok_or("Workbench layout directory is missing")?;
    reject_link_or_special(parent, true)?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    reject_link_or_special(path, false)?;
    let pending = parent.join(format!(
        ".desktop-workbench-layout-{}-{}.pending",
        std::process::id(),
        NEXT_WRITE.fetch_add(1, Ordering::Relaxed),
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&pending).map_err(|e| e.to_string())?;
    let result = (|| {
        let bytes = serde_json::to_vec(layout).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        // Replace only after the complete record is durable; never truncate the last good copy.
        fs::rename(&pending, path).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&pending);
    }
    result
}

#[cfg(test)]
#[path = "panel_preferences_tests.rs"]
mod tests;
