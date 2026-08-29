use std::ffi::OsString;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::RunnerResult;
use crate::desktop::{Platform, host_platform};

#[derive(Clone, Copy)]
struct AppDefinition {
    product_name: &'static str,
    executable_name: &'static str,
    linux_package_name: &'static str,
}

pub(crate) fn run_desktop_install_host(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args
        .iter()
        .any(|arg| matches!(arg.to_str(), Some("--help" | "-h")))
    {
        println!(
            "usage: kyuubiki-script-runner desktop-install-host\n\nInstalls the current three-shell package set on macOS or Ubuntu Linux."
        );
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("desktop-install-host does not accept positional arguments".to_string());
    }
    match host_platform() {
        Platform::Macos => install_macos_apps(root),
        Platform::Linux => install_linux_debs(root),
        Platform::Windows => Err(
            "desktop-install-host does not yet install Windows packages; use the signed NSIS lifecycle"
                .to_string(),
        ),
    }
}

fn install_macos_apps(root: &Path) -> RunnerResult<u8> {
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
            linux_package_name: "kyuubiki-hub",
        },
        AppDefinition {
            product_name: "Kyuubiki Installer",
            executable_name: "kyuubiki-installer-gui",
            linux_package_name: "kyuubiki-installer",
        },
        AppDefinition {
            product_name: "Kyuubiki Workbench",
            executable_name: "kyuubiki-workbench-gui",
            linux_package_name: "kyuubiki-workbench",
        },
    ]
}

fn install_linux_debs(root: &Path) -> RunnerResult<u8> {
    for required in [
        "/usr/bin/apt-get",
        "/usr/bin/dpkg",
        "/usr/bin/dpkg-deb",
        "/usr/bin/dpkg-query",
        "/usr/bin/sudo",
    ] {
        if !Path::new(required).is_file() {
            return Err(format!(
                "required Ubuntu package tool is unavailable: {required}"
            ));
        }
    }

    let source_root = root.join("target/desktop-cache/linux/release/bundle/deb");
    let debs = collect_debs(&source_root)?;
    if debs.len() != app_definitions().len() {
        return Err(format!(
            "Linux desktop install requires exactly three .deb files, found {} in {}",
            debs.len(),
            source_root.display()
        ));
    }

    let expected_version = shipping_version(root)?;
    let architecture = command_stdout("/usr/bin/dpkg", &["--print-architecture"])?;
    let mut packages = Vec::new();
    for app in app_definitions() {
        let mut matching = Vec::new();
        for deb in &debs {
            if deb_field(deb, "Package")? == app.linux_package_name {
                matching.push(deb.clone());
            }
        }
        if matching.len() != 1 {
            return Err(format!(
                "expected one {} package, found {}",
                app.linux_package_name,
                matching.len()
            ));
        }
        let package = matching.remove(0);
        let version = deb_field(&package, "Version")?;
        let package_architecture = deb_field(&package, "Architecture")?;
        if version != expected_version || package_architecture != architecture {
            return Err(format!(
                "{} metadata mismatch: expected version {} architecture {}, found version {} architecture {}",
                package.display(),
                expected_version,
                architecture,
                version,
                package_architecture
            ));
        }
        packages.push(package);
    }

    let staged = stage_debs_for_apt(&packages)?;
    let status = Command::new("/usr/bin/sudo")
        .args([
            "-n",
            "/usr/bin/apt-get",
            "install",
            "-y",
            "--no-install-recommends",
        ])
        .args(&staged.packages)
        .status()
        .map_err(|error| format!("failed to launch Ubuntu package installer: {error}"))?;
    if !status.success() {
        return Err(format!(
            "Ubuntu package installation failed with {status}; configure installer-managed passwordless package privileges"
        ));
    }

    for app in app_definitions() {
        let installed_version = command_stdout(
            "/usr/bin/dpkg-query",
            &["-W", "-f=${Version}", app.linux_package_name],
        )?;
        let executable = Path::new("/usr/bin").join(app.executable_name);
        if installed_version != expected_version || !executable.is_file() {
            return Err(format!(
                "installed {} failed verification: version {}, executable {}",
                app.product_name,
                installed_version,
                executable.display()
            ));
        }
        println!("installed {} {}", app.product_name, installed_version);
    }
    println!("installed all three Kyuubiki desktop shells through Ubuntu packages");
    Ok(0)
}

fn collect_debs(source_root: &Path) -> RunnerResult<Vec<PathBuf>> {
    let entries = fs::read_dir(source_root).map_err(|error| {
        format!(
            "failed to read Linux desktop package directory {}: {error}",
            source_root.display()
        )
    })?;
    let mut debs = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("deb"))
        .collect::<Vec<_>>();
    debs.sort();
    Ok(debs)
}

struct TemporaryPackageDir {
    root: PathBuf,
    packages: Vec<PathBuf>,
}

impl Drop for TemporaryPackageDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn stage_debs_for_apt(packages: &[PathBuf]) -> RunnerResult<TemporaryPackageDir> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-desktop-install-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&root)
        .map_err(|error| format!("failed to create apt staging directory: {error}"))?;
    let mut staged = TemporaryPackageDir {
        root,
        packages: Vec::new(),
    };
    #[cfg(unix)]
    fs::set_permissions(&staged.root, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("failed to protect apt staging directory: {error}"))?;

    for package in packages {
        let name = package
            .file_name()
            .ok_or_else(|| format!("package path has no file name: {}", package.display()))?;
        let destination = staged.root.join(name);
        fs::copy(package, &destination).map_err(|error| {
            format!(
                "failed to stage package {} for apt: {error}",
                package.display()
            )
        })?;
        #[cfg(unix)]
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o644)).map_err(|error| {
            format!(
                "failed to protect staged package {}: {error}",
                destination.display()
            )
        })?;
        staged.packages.push(destination);
    }
    Ok(staged)
}

fn deb_field(path: &Path, field: &str) -> RunnerResult<String> {
    let output = Command::new("/usr/bin/dpkg-deb")
        .args(["--field"])
        .arg(path)
        .arg(field)
        .output()
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    if !output.status.success() {
        return Err(format!("failed to read {field} from {}", path.display()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn command_stdout(program: &str, args: &[&str]) -> RunnerResult<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{program} failed with {}", output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn shipping_version(root: &Path) -> RunnerResult<String> {
    let path = root.join("deploy/update-channels.json");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    value
        .get("shipping_version")
        .and_then(serde_json::Value::as_str)
        .filter(|version| !version.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("{} has no shipping_version", path.display()))
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
        let linux_packages = apps
            .iter()
            .map(|app| app.linux_package_name)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(names.len(), 3);
        assert_eq!(executables.len(), 3);
        assert_eq!(linux_packages.len(), 3);
        assert!(
            linux_packages
                .iter()
                .all(|name| name.starts_with("kyuubiki-"))
        );
    }

    #[test]
    fn cleanup_guard_rejects_non_work_paths() {
        let root = Path::new("/Applications");
        let error = remove_exact_work_path(root, &root.join("Kyuubiki Hub.app"))
            .expect_err("installed app must never pass the work-path cleanup guard");
        assert!(error.contains("unexpected work path"));
    }

    #[cfg(unix)]
    #[test]
    fn apt_staging_is_world_readable_and_removed_on_drop() {
        let source_root = std::env::temp_dir().join(format!(
            "kyuubiki-desktop-install-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&source_root);
        fs::create_dir(&source_root).unwrap();
        let package = source_root.join("Kyuubiki Hub_2.18.3_amd64.deb");
        fs::write(&package, b"package").unwrap();

        let staged = stage_debs_for_apt(&[package]).unwrap();
        let staged_root = staged.root.clone();
        assert_eq!(
            fs::metadata(&staged.root).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fs::metadata(&staged.packages[0])
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o644
        );
        drop(staged);
        assert!(!staged_root.exists());
        fs::remove_dir_all(source_root).unwrap();
    }
}
