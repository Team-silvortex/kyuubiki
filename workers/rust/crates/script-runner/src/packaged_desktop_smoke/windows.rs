use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use super::SurfaceDefinition;
use crate::RunnerResult;

const CLEANUP_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) struct InstalledPackages {
    roots: Vec<PathBuf>,
}

impl InstalledPackages {
    pub(super) fn cleanup(&mut self) -> RunnerResult<()> {
        let mut failures = Vec::new();
        for root in self.roots.drain(..).rev() {
            if let Err(error) = uninstall(&root) {
                failures.push(error);
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "Windows desktop smoke cleanup failed: {}",
                failures.join("; ")
            ))
        }
    }
}

impl Drop for InstalledPackages {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

pub(super) fn install_nsis_packages(
    root: &Path,
    bundle_root: &Path,
    definitions: &[SurfaceDefinition],
) -> RunnerResult<InstalledPackages> {
    let installers = installers(root)?;
    let mut installed = InstalledPackages { roots: Vec::new() };
    for definition in definitions {
        let app_root = bundle_root.join(definition.product_name);
        let executable = app_root.join(format!("{}.exe", definition.executable_name));
        if app_root.exists() {
            return Err(format!(
                "refusing to overwrite an existing Windows installation: {}",
                app_root.display()
            ));
        }
        let installer = find_installer(&installers, definition.product_name)?;
        let status = Command::new(&installer)
            .arg("/S")
            .status()
            .map_err(|error| format!("failed to run {}: {error}", installer.display()))?;
        if !status.success() {
            return Err(format!(
                "NSIS installer failed for {} with {status}",
                definition.product_name
            ));
        }
        installed.roots.push(app_root);
        wait_for_path(&executable, true).map_err(|error| {
            format!(
                "{} did not install its executable: {error}",
                definition.product_name
            )
        })?;
    }
    Ok(installed)
}

fn installers(root: &Path) -> RunnerResult<Vec<PathBuf>> {
    let path = root.join("target/desktop-cache/windows/release/bundle/nsis");
    let mut installers = fs::read_dir(&path)
        .map_err(|error| format!("failed to read NSIS bundle {}: {error}", path.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|entry| {
            entry.is_file()
                && entry.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .to_ascii_lowercase()
                        .ends_with("-setup.exe")
                })
        })
        .collect::<Vec<_>>();
    installers.sort();
    Ok(installers)
}

fn find_installer(installers: &[PathBuf], product_name: &str) -> RunnerResult<PathBuf> {
    let product = product_name.to_ascii_lowercase();
    let matches = installers
        .iter()
        .filter(|path| {
            path.file_name().is_some_and(|name| {
                name.to_string_lossy()
                    .to_ascii_lowercase()
                    .contains(&product)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [installer] => Ok(installer.clone()),
        [] => Err(format!("missing NSIS installer for {product_name}")),
        _ => Err(format!("multiple NSIS installers found for {product_name}")),
    }
}

fn uninstall(root: &Path) -> RunnerResult<()> {
    if !root.exists() {
        return Ok(());
    }
    let uninstaller = root.join("uninstall.exe");
    let status = Command::new(&uninstaller)
        .arg("/S")
        .status()
        .map_err(|error| format!("failed to run {}: {error}", uninstaller.display()))?;
    if !status.success() {
        return Err(format!(
            "uninstaller failed for {} with {status}",
            root.display()
        ));
    }
    wait_for_path(root, false)
}

fn wait_for_path(path: &Path, should_exist: bool) -> RunnerResult<()> {
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    while path.exists() != should_exist {
        if Instant::now() >= deadline {
            return Err(format!(
                "timed out waiting for {} to {}",
                path.display(),
                if should_exist { "exist" } else { "disappear" }
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_each_product_installer_without_cross_talk() {
        let installers = [
            "Kyuubiki Hub_2.17.0_x64-setup.exe",
            "Kyuubiki Installer_2.17.0_x64-setup.exe",
            "Kyuubiki Workbench_2.17.0_x64-setup.exe",
        ]
        .map(PathBuf::from);
        let selected = find_installer(&installers, "Kyuubiki Installer")
            .expect("installer package should match");
        assert_eq!(
            selected,
            PathBuf::from("Kyuubiki Installer_2.17.0_x64-setup.exe")
        );
    }
}
