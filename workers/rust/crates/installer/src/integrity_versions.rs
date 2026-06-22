use std::fs;
use std::path::{Path, PathBuf};

use crate::VersionAlignmentCheck;

pub(super) fn collect_version_checks(root: &Path, expected: &str) -> Vec<VersionAlignmentCheck> {
    let version_files = [
        (
            "release index",
            "releases/index.json",
            VersionKey::CurrentVersion,
        ),
        (
            "frontend package",
            "apps/frontend/package.json",
            VersionKey::Version,
        ),
        (
            "hub package",
            "apps/hub-gui/package.json",
            VersionKey::Version,
        ),
        (
            "hub tauri",
            "apps/hub-gui/src-tauri/tauri.conf.json",
            VersionKey::Version,
        ),
        (
            "hub brand",
            "apps/hub-gui/ui/assets/brand.json",
            VersionKey::ReleaseVersion,
        ),
        (
            "workbench package",
            "apps/workbench-gui/package.json",
            VersionKey::Version,
        ),
        (
            "workbench tauri",
            "apps/workbench-gui/src-tauri/tauri.conf.json",
            VersionKey::Version,
        ),
        (
            "workbench brand",
            "apps/workbench-gui/ui/assets/brand.json",
            VersionKey::ReleaseVersion,
        ),
        (
            "installer package",
            "apps/installer-gui/package.json",
            VersionKey::Version,
        ),
        (
            "installer tauri",
            "apps/installer-gui/src-tauri/tauri.conf.json",
            VersionKey::Version,
        ),
        (
            "installer brand",
            "apps/installer-gui/ui/assets/brand.json",
            VersionKey::ReleaseVersion,
        ),
    ];

    let mut checks: Vec<VersionAlignmentCheck> = version_files
        .into_iter()
        .map(|(label, relative_path, key)| {
            let actual = read_json_version(root.join(relative_path), key)
                .unwrap_or_else(|| "missing".to_string());
            VersionAlignmentCheck {
                label: label.to_string(),
                expected: expected.to_string(),
                ok: actual == expected,
                actual,
            }
        })
        .collect();

    let rust_workspace_version = read_workspace_cargo_version(root.join("workers/rust/Cargo.toml"))
        .unwrap_or_else(|| "missing".to_string());
    checks.push(VersionAlignmentCheck {
        label: "rust workspace".to_string(),
        expected: expected.to_string(),
        ok: rust_workspace_version == expected,
        actual: rust_workspace_version,
    });

    checks
}

pub(super) fn current_release_version(root: &Path) -> Option<String> {
    read_json_version(root.join("releases/index.json"), VersionKey::CurrentVersion)
}

fn read_json_version(path: PathBuf, key: VersionKey) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    let payload: serde_json::Value = serde_json::from_str(&contents).ok()?;
    let key_name = match key {
        VersionKey::CurrentVersion => "current_version",
        VersionKey::ReleaseVersion => "releaseVersion",
        VersionKey::Version => "version",
    };
    payload.get(key_name)?.as_str().map(str::to_string)
}

fn read_workspace_cargo_version(path: PathBuf) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    let mut in_workspace_package = false;

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            in_workspace_package = line == "[workspace.package]";
            continue;
        }

        if in_workspace_package && line.starts_with("version") {
            let (_, value) = line.split_once('=')?;
            return Some(value.trim().trim_matches('"').to_string());
        }
    }

    None
}

#[derive(Clone, Copy)]
enum VersionKey {
    CurrentVersion,
    ReleaseVersion,
    Version,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_version_reader_understands_workspace_package_block() {
        let root = std::env::temp_dir().join("kyuubiki-installer-integrity-test");
        let _ = fs::create_dir_all(&root);
        let path = root.join("Cargo.toml");
        let contents = "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"1.6.0\"\n";
        fs::write(&path, contents).unwrap();
        assert_eq!(
            read_workspace_cargo_version(path),
            Some("1.6.0".to_string())
        );
        let _ = fs::remove_dir_all(root);
    }
}
