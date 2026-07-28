use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::RunnerResult;
use crate::desktop::{Platform, host_platform};

#[derive(Clone, Copy)]
struct AppDefinition {
    product_name: &'static str,
    executable_name: &'static str,
}

pub(crate) fn run_desktop_install_host(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args
        .iter()
        .any(|arg| matches!(arg.to_str(), Some("--help" | "-h")))
    {
        println!("usage: kyuubiki-script-runner desktop-install-host");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("desktop-install-host does not accept positional arguments".to_string());
    }
    if host_platform() != Platform::Macos {
        return Err(
            "desktop-install-host currently supports macOS; use the platform installer on Linux or Windows"
                .to_string(),
        );
    }

    let source_root = root.join("target/desktop-cache/macos/release/bundle/macos");
    let applications_root = PathBuf::from("/Applications");
    if !applications_root.is_dir() {
        return Err("macOS application directory is unavailable".to_string());
    }
    for app in app_definitions() {
        install_app(&source_root, &applications_root, app)?;
    }
    println!("installed all three Kyuubiki desktop shells into /Applications");
    Ok(0)
}

fn app_definitions() -> [AppDefinition; 3] {
    [
        AppDefinition {
            product_name: "Kyuubiki Hub",
            executable_name: "kyuubiki-hub-gui",
        },
        AppDefinition {
            product_name: "Kyuubiki Installer",
            executable_name: "kyuubiki-installer-gui",
        },
        AppDefinition {
            product_name: "Kyuubiki Workbench",
            executable_name: "kyuubiki-workbench-gui",
        },
    ]
}

fn install_app(
    source_root: &Path,
    applications_root: &Path,
    app: AppDefinition,
) -> RunnerResult<()> {
    let bundle_name = format!("{}.app", app.product_name);
    let source = source_root.join(&bundle_name);
    validate_bundle(&source, app)?;

    let nonce = std::process::id();
    let staging = applications_root.join(format!(".kyuubiki-install-{nonce}-{bundle_name}"));
    let backup = applications_root.join(format!(".kyuubiki-backup-{nonce}-{bundle_name}"));
    remove_exact_work_path(applications_root, &staging)?;
    remove_exact_work_path(applications_root, &backup)?;

    let copy_status = Command::new("/usr/bin/ditto")
        .arg(&source)
        .arg(&staging)
        .status()
        .map_err(|error| format!("failed to stage {}: {error}", app.product_name))?;
    if !copy_status.success() {
        remove_exact_work_path(applications_root, &staging)?;
        return Err(format!(
            "ditto failed while staging {} with {copy_status}",
            app.product_name
        ));
    }
    validate_bundle(&staging, app)?;

    let destination = applications_root.join(&bundle_name);
    let had_existing = destination.exists();
    if had_existing {
        fs::rename(&destination, &backup).map_err(|error| {
            format!(
                "failed to preserve existing {} before replacement: {error}",
                destination.display()
            )
        })?;
    }
    if let Err(error) = fs::rename(&staging, &destination) {
        if had_existing {
            let _ = fs::rename(&backup, &destination);
        }
        return Err(format!(
            "failed to activate {}: {error}; previous bundle restored",
            app.product_name
        ));
    }
    if had_existing {
        remove_exact_work_path(applications_root, &backup)?;
    }
    println!("installed {}", destination.display());
    Ok(())
}

fn validate_bundle(path: &Path, app: AppDefinition) -> RunnerResult<()> {
    let executable = path.join("Contents/MacOS").join(app.executable_name);
    let info = path.join("Contents/Info.plist");
    if !path.is_dir() || !info.is_file() || !executable.is_file() {
        return Err(format!(
            "{} is not a complete {} application bundle",
            path.display(),
            app.product_name
        ));
    }
    Ok(())
}

fn remove_exact_work_path(applications_root: &Path, path: &Path) -> RunnerResult<()> {
    if path.parent() != Some(applications_root) {
        return Err(format!(
            "refusing to remove installer work path outside {}",
            applications_root.display()
        ));
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !name.starts_with(".kyuubiki-") || !name.ends_with(".app") {
        return Err(format!("refusing to remove unexpected work path {name}"));
    }
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to clean {}: {error}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_definitions_are_unique_and_complete() {
        let apps = app_definitions();
        let names = apps
            .iter()
            .map(|app| app.product_name)
            .collect::<std::collections::BTreeSet<_>>();
        let executables = apps
            .iter()
            .map(|app| app.executable_name)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(names.len(), 3);
        assert_eq!(executables.len(), 3);
    }

    #[test]
    fn cleanup_guard_rejects_non_work_paths() {
        let root = Path::new("/Applications");
        let error = remove_exact_work_path(root, &root.join("Kyuubiki Hub.app"))
            .expect_err("installed app must never pass the work-path cleanup guard");
        assert!(error.contains("unexpected work path"));
    }
}
