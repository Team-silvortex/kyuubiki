use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::ServiceMode;

pub(crate) fn service_mode_name(mode: ServiceMode) -> &'static str {
    match mode {
        ServiceMode::Default => "default",
        ServiceMode::Local => "local",
        ServiceMode::Cloud => "cloud",
        ServiceMode::Distributed => "distributed",
    }
}

pub(crate) fn remove_file_if_present(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("failed to remove {}: {error}", path.display())),
    }
}
