use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::RunnerResult;

pub(crate) struct BundleStaging {
    bundle: PathBuf,
    staging: PathBuf,
    captured: Vec<String>,
    had_previous: bool,
    finished: bool,
}

impl BundleStaging {
    pub(crate) fn begin(bundle: PathBuf, staging: PathBuf) -> RunnerResult<Self> {
        if let Some(parent) = staging.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("create staging parent: {error}"))?;
        }
        // Never erase an unfinished build's recovery copy or another build's staging area.
        fs::create_dir(&staging).map_err(|error| format!(
            "cannot reserve desktop staging {}: {error}; another build or unfinished recovery may exist",
            staging.display()
        ))?;
        let had_previous = match fs::symlink_metadata(&bundle) {
            Ok(metadata) if metadata.is_dir() => true,
            Ok(_) => {
                let _ = fs::remove_dir(&staging);
                return Err(format!(
                    "desktop bundle root is not a real directory: {}",
                    bundle.display()
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => {
                let _ = fs::remove_dir(&staging);
                return Err(format!("inspect desktop bundles: {error}"));
            }
        };
        if had_previous && let Err(error) = fs::rename(&bundle, staging.join("previous")) {
            let _ = fs::remove_dir(&staging);
            return Err(format!("preserve desktop bundles without copying: {error}"));
        }
        Ok(Self {
            bundle,
            staging,
            captured: Vec::new(),
            had_previous,
            finished: false,
        })
    }

    pub(crate) fn capture(&mut self, app: &str) -> RunnerResult<()> {
        self.require_active()?;
        if !matches!(app, "hub-gui" | "installer-gui" | "workbench-gui")
            || self.captured.iter().any(|name| name == app)
        {
            return Err(format!(
                "invalid or duplicate desktop artifact owner: {app}"
            ));
        }
        if !fs::symlink_metadata(&self.bundle)
            .map_err(|error| {
                format!(
                    "desktop build did not produce {}: {error}",
                    self.bundle.display()
                )
            })?
            .is_dir()
        {
            return Err("desktop build output is not a real directory".into());
        }
        if fs::read_dir(&self.bundle)
            .map_err(|error| format!("read desktop bundle output: {error}"))?
            .next()
            .transpose()
            .map_err(|error| format!("read desktop bundle output entry: {error}"))?
            .is_none()
        {
            return Err(format!(
                "desktop build produced no bundle artifacts for {app}"
            ));
        }
        fs::rename(&self.bundle, self.staging.join(app))
            .map_err(|error| format!("move {app} artifacts into staging: {error}"))?;
        self.captured.push(app.to_string());
        println!("staged {app} bundle artifacts without copying");
        Ok(())
    }

    pub(crate) fn commit(&mut self) -> RunnerResult<()> {
        self.require_active()?;
        if self.captured.len() != 3 {
            return Err(
                "all three independent desktop bundles are required before publication".into(),
            );
        }
        let assembled = self.staging.join("assembled");
        fs::create_dir(&assembled).map_err(|error| format!("create assembled bundles: {error}"))?;
        for app in &self.captured {
            merge_by_move(&self.staging.join(app), &assembled)?;
        }
        fs::rename(&assembled, &self.bundle)
            .map_err(|error| format!("publish desktop bundle set: {error}"))?;
        self.finished = true;
        // Publication succeeded; cleanup errors must not roll back the valid bundle set.
        if let Err(error) = fs::remove_dir_all(&self.staging) {
            eprintln!(
                "desktop bundles published; staging cleanup failed at {}: {error}",
                self.staging.display()
            );
        }
        Ok(())
    }

    pub(crate) fn rollback(&mut self) -> RunnerResult<()> {
        self.require_active()?;
        if self.had_previous && !self.staging.join("previous").is_dir() {
            return Err(format!(
                "previous bundles are missing; preserve {} for inspection",
                self.staging.display()
            ));
        }
        match fs::remove_dir_all(&self.bundle) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "remove partial bundles: {error}; recovery: {}",
                    self.staging.display()
                ));
            }
        }
        if self.had_previous {
            fs::rename(self.staging.join("previous"), &self.bundle).map_err(|error| {
                format!(
                    "restore previous bundles: {error}; recovery: {}",
                    self.staging.display()
                )
            })?;
        }
        self.finished = true;
        fs::remove_dir_all(&self.staging).map_err(|error| {
            format!(
                "clean failed build staging {}: {error}",
                self.staging.display()
            )
        })?;
        println!("discarded partial desktop build; previous bundle state restored");
        Ok(())
    }

    fn require_active(&self) -> RunnerResult<()> {
        if self.finished {
            Err("desktop bundle transaction is already finished".into())
        } else {
            Ok(())
        }
    }
}

fn merge_by_move(source: &Path, target: &Path) -> RunnerResult<()> {
    for entry in
        fs::read_dir(source).map_err(|error| format!("read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("read bundle entry: {error}"))?;
        let destination = target.join(entry.file_name());
        let source_type = entry
            .file_type()
            .map_err(|error| format!("inspect bundle entry: {error}"))?;
        match fs::symlink_metadata(&destination) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // Move whole app trees: preserve modes, symlinks, and signatures without copying.
                fs::rename(entry.path(), &destination)
                    .map_err(|error| format!("move bundle {}: {error}", destination.display()))?;
            }
            Ok(_)
                if destination
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        matches!(
                            extension.to_ascii_lowercase().as_str(),
                            "app" | "dmg" | "deb" | "rpm" | "exe" | "msi" | "appimage"
                        )
                    }) =>
            {
                return Err(format!(
                    "conflicting desktop bundle artifact: {}",
                    destination.display()
                ));
            }
            Ok(metadata) if metadata.is_dir() && source_type.is_dir() => {
                merge_by_move(&entry.path(), &destination)?;
            }
            Ok(metadata)
                if metadata.is_file()
                    && source_type.is_file()
                    && same_file_bytes(&entry.path(), &destination)? => {}
            Ok(metadata)
                if metadata.file_type().is_symlink()
                    && source_type.is_symlink()
                    && fs::read_link(entry.path()).map_err(|error| error.to_string())?
                        == fs::read_link(&destination).map_err(|error| error.to_string())? => {}
            Ok(_) => {
                return Err(format!(
                    "conflicting desktop bundle artifact: {}",
                    destination.display()
                ));
            }
            Err(error) => return Err(format!("inspect bundle {}: {error}", destination.display())),
        }
    }
    Ok(())
}

fn same_file_bytes(left: &Path, right: &Path) -> RunnerResult<bool> {
    let compare = || -> std::io::Result<bool> {
        let mut left = fs::File::open(left)?;
        let mut right = fs::File::open(right)?;
        if left.metadata()?.len() != right.metadata()?.len() {
            return Ok(false);
        }
        let mut a = [0; 65536];
        let mut b = [0; 65536];
        loop {
            let count = left.read(&mut a)?;
            if count == 0 {
                return Ok(true);
            }
            right.read_exact(&mut b[..count])?;
            if a[..count] != b[..count] {
                return Ok(false);
            }
        }
    };
    compare().map_err(|error| format!("compare bundle artifacts: {error}"))
}
