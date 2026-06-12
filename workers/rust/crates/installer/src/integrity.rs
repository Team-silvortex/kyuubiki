use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    IntegrityContract, Platform, contract_path, load_integrity_contract, prepare_layout,
    workspace_root,
};

const INTEGRITY_SCHEMA_VERSION: &str = "kyuubiki.installation-integrity/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallationIntegrityEntry {
    pub label: String,
    pub relative_path: String,
    pub required: bool,
    pub present: bool,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionAlignmentCheck {
    pub label: String,
    pub expected: String,
    pub actual: String,
    pub ok: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResidueCandidate {
    pub relative_path: String,
    pub reason: String,
    pub removable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrityContractRule {
    pub category: String,
    pub label: String,
    pub value: String,
    pub editable: bool,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallationIntegrityReport {
    pub schema_version: String,
    pub platform: String,
    pub workspace: String,
    pub current_version: String,
    pub contract_rules: Vec<IntegrityContractRule>,
    pub layout: Vec<InstallationIntegrityEntry>,
    pub version_checks: Vec<VersionAlignmentCheck>,
    pub residues: Vec<ResidueCandidate>,
    pub issues: Vec<String>,
}

impl InstallationIntegrityReport {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki installation integrity".to_string(),
            format!("schema: {}", self.schema_version),
            format!("platform: {}", self.platform),
            format!("workspace: {}", self.workspace),
            format!("current_version: {}", self.current_version),
        ];

        lines.push("contract_rules:".to_string());
        for rule in &self.contract_rules {
            lines.push(format!(
                "  [{}] {} => {} ({})",
                if rule.editable {
                    "editable"
                } else {
                    "read-only"
                },
                rule.label,
                rule.value,
                rule.category
            ));
        }

        lines.push("layout:".to_string());
        for entry in &self.layout {
            lines.push(format!(
                "  [{}] {} ({}, {} bytes)",
                if entry.present { "ok" } else { "missing" },
                entry.relative_path,
                entry.label,
                entry.size_bytes
            ));
        }

        lines.push("version_alignment:".to_string());
        for check in &self.version_checks {
            lines.push(format!(
                "  [{}] {} => expected {}, actual {}",
                if check.ok { "ok" } else { "mismatch" },
                check.label,
                check.expected,
                check.actual
            ));
        }

        lines.push("residue:".to_string());
        if self.residues.is_empty() {
            lines.push("  [ok] no removable residue detected".to_string());
        } else {
            for residue in &self.residues {
                lines.push(format!(
                    "  [{}] {} ({})",
                    if residue.removable {
                        "remove"
                    } else {
                        "review"
                    },
                    residue.relative_path,
                    residue.reason
                ));
            }
        }

        if !self.issues.is_empty() {
            lines.push("issues:".to_string());
            for issue in &self.issues {
                lines.push(format!("  - {issue}"));
            }
        }

        lines.join("\n")
    }
}

pub fn installation_integrity_report() -> InstallationIntegrityReport {
    let root = workspace_root();
    let current_version = current_release_version(&root).unwrap_or_else(|| "unknown".to_string());
    let platform = Platform::current();
    let mut issues = Vec::new();
    let contract = match load_integrity_contract(&root, platform) {
        Ok(contract) => contract,
        Err(detail) => {
            issues.push(format!("installation contract load failed: {detail}"));
            fallback_contract(platform, &current_version)
        }
    };
    let contract_rules = contract
        .visible_rules
        .iter()
        .map(|rule| IntegrityContractRule {
            category: rule.category.clone(),
            label: rule.label.clone(),
            value: rule.value.clone(),
            editable: rule.editable,
            description: rule.description.clone(),
        })
        .chain(std::iter::once(IntegrityContractRule {
            category: "contract".to_string(),
            label: "contract source".to_string(),
            value: contract_path().to_string(),
            editable: false,
            description:
                "Shared JSON source that drives installer integrity behavior and documentation."
                    .to_string(),
        }))
        .collect();
    let layout = expected_layout(&root, &contract);
    let version_checks = collect_version_checks(&root, &contract.shipping_version);
    let residues = collect_residue_candidates(&root, &contract);
    issues.extend(collect_layout_issues(&layout));
    if current_version != contract.shipping_version {
        issues.push(format!(
            "release line mismatch: releases/index.json is {}, contract expects {}",
            current_version, contract.shipping_version
        ));
    }

    for check in &version_checks {
        if !check.ok {
            issues.push(format!(
                "version mismatch: {} expected {}, found {}",
                check.label, check.expected, check.actual
            ));
        }
    }

    for residue in &residues {
        let verb = if residue.removable {
            "removable residue"
        } else {
            "layout drift"
        };
        issues.push(format!("{verb}: {}", residue.relative_path));
    }

    InstallationIntegrityReport {
        schema_version: INTEGRITY_SCHEMA_VERSION.to_string(),
        platform: platform.as_str().to_string(),
        workspace: root.display().to_string(),
        current_version,
        contract_rules,
        layout,
        version_checks,
        residues,
        issues,
    }
}

pub fn repair_installation() -> Result<String, String> {
    prepare_layout()?;
    let root = workspace_root();
    let platform = Platform::current();
    let current_version = current_release_version(&root).unwrap_or_else(|| "unknown".to_string());
    let contract = load_integrity_contract(&root, platform)
        .unwrap_or_else(|_| fallback_contract(platform, &current_version));
    let residues = collect_residue_candidates(&root, &contract);
    let mut removed = Vec::new();
    let mut freed_bytes = 0u64;

    for residue in residues.iter().filter(|item| item.removable) {
        if is_protected_relative(&residue.relative_path, &contract.protected_paths) {
            continue;
        }
        let target = root.join(&residue.relative_path);
        if !target.exists() {
            continue;
        }

        let size = path_size(&target);
        remove_path(&target)?;
        freed_bytes += size;
        removed.push(residue.relative_path.clone());
    }

    Ok(format!(
        "installation repair completed (removed={}, freed_bytes={})",
        removed.len(),
        freed_bytes
    ))
}

fn expected_layout(root: &Path, contract: &IntegrityContract) -> Vec<InstallationIntegrityEntry> {
    contract
        .required_layout
        .iter()
        .map(|rule| {
            let absolute_path = if rule.relative_path == "." {
                root.to_path_buf()
            } else {
                root.join(&rule.relative_path)
            };

            InstallationIntegrityEntry {
                label: rule.label.clone(),
                relative_path: rule.relative_path.clone(),
                required: rule.required,
                present: absolute_path.exists(),
                size_bytes: path_size(&absolute_path),
            }
        })
        .collect()
}

fn collect_layout_issues(layout: &[InstallationIntegrityEntry]) -> Vec<String> {
    layout
        .iter()
        .filter(|entry| entry.required && !entry.present)
        .map(|entry| format!("missing required path: {}", entry.relative_path))
        .collect()
}

fn collect_version_checks(root: &Path, expected: &str) -> Vec<VersionAlignmentCheck> {
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

fn collect_residue_candidates(root: &Path, contract: &IntegrityContract) -> Vec<ResidueCandidate> {
    let mut residues = Vec::new();
    scan_for_residue_patterns(
        root,
        root,
        &contract.removable_patterns,
        &contract.protected_paths,
        &mut residues,
    );
    collect_dist_layout_drift(root, &contract.allowed_dist_children, &mut residues);
    residues.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    residues
}

fn scan_for_residue_patterns(
    root: &Path,
    current: &Path,
    removable_patterns: &[String],
    protected_paths: &[String],
    residues: &mut Vec<ResidueCandidate>,
) {
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let relative_path = relative_display(root, &path);
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        if file_name == ".git" {
            continue;
        }

        if relative_path != "." && is_protected_relative(&relative_path, protected_paths) {
            continue;
        }

        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            scan_for_residue_patterns(root, &path, removable_patterns, protected_paths, residues);
            continue;
        }

        if matches_residue_pattern(&relative_path, &file_name, removable_patterns) {
            residues.push(ResidueCandidate {
                relative_path,
                reason: "platform cache file".to_string(),
                removable: true,
            });
        }
    }
}

fn collect_dist_layout_drift(
    root: &Path,
    allowed_children: &[String],
    residues: &mut Vec<ResidueCandidate>,
) {
    let dist = root.join("dist");
    let Ok(entries) = fs::read_dir(&dist) else {
        return;
    };

    for entry in entries.flatten() {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if allowed_children.iter().any(|allowed| allowed == &name) {
            continue;
        }

        residues.push(ResidueCandidate {
            relative_path: relative_display(root, &entry.path()),
            reason: "non-standard dist child; review before removing".to_string(),
            removable: false,
        });
    }
}

fn fallback_contract(platform: Platform, current_version: &str) -> IntegrityContract {
    IntegrityContract {
        schema_version: "kyuubiki.installation-contract/v1".to_string(),
        product_line: "tamamono 1.x".to_string(),
        shipping_version: current_version.to_string(),
        required_layout: vec![
            fallback_layout_rule("workspace root", ".", true),
            fallback_layout_rule("runtime temp root", "tmp", true),
            fallback_layout_rule("runtime process state", "tmp/run", true),
            fallback_layout_rule("runtime data state", "tmp/data", true),
            fallback_layout_rule("release staging root", "dist", true),
            fallback_layout_rule(
                "platform release staging",
                &format!("dist/{}", platform.as_str()),
                true,
            ),
        ],
        protected_paths: vec![".env.local".to_string(), "tmp/data".to_string()],
        removable_patterns: vec![
            ".DS_Store".to_string(),
            "Thumbs.db".to_string(),
            "._*".to_string(),
            "tmp/run/*.pid".to_string(),
            "tmp/run/*.sock".to_string(),
            "tmp/run/*.lock".to_string(),
            "tmp/run/*.tmp".to_string(),
        ],
        allowed_dist_children: vec!["macos".to_string(), "linux".to_string(), "windows".to_string()],
        visible_rules: vec![IntegrityContractRule {
            category: "contract".to_string(),
            label: "contract source".to_string(),
            value: contract_path().to_string(),
            editable: false,
            description: "Fallback contract is active because the shared installation contract file could not be loaded.".to_string(),
        }]
        .into_iter()
        .map(|rule| crate::integrity_contract::IntegrityVisibleRule {
            category: rule.category,
            label: rule.label,
            value: rule.value,
            editable: rule.editable,
            description: rule.description,
        })
        .collect(),
    }
}

fn fallback_layout_rule(
    label: &str,
    relative_path: &str,
    required: bool,
) -> crate::integrity_contract::IntegrityLayoutRule {
    crate::integrity_contract::IntegrityLayoutRule {
        label: label.to_string(),
        relative_path: relative_path.to_string(),
        required,
    }
}

fn matches_residue_pattern(relative_path: &str, file_name: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        if pattern.contains('/') {
            if let Some((dir, suffix_pattern)) = pattern.rsplit_once('/') {
                if !relative_path.starts_with(dir) {
                    return false;
                }

                if let Some(stripped) = suffix_pattern.strip_prefix("*.") {
                    return file_name.ends_with(&format!(".{stripped}"));
                }

                return file_name == suffix_pattern;
            }
        }

        if let Some(stripped) = pattern.strip_prefix("*.") {
            return file_name.ends_with(&format!(".{stripped}"));
        }

        if let Some(stripped) = pattern.strip_suffix('*') {
            return file_name.starts_with(stripped);
        }

        file_name == pattern
    })
}

fn is_protected_relative(relative_path: &str, protected_paths: &[String]) -> bool {
    protected_paths.iter().any(|protected| {
        relative_path == protected
            || relative_path
                .strip_prefix(protected)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

fn current_release_version(root: &Path) -> Option<String> {
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

fn path_size(path: &Path) -> u64 {
    let Ok(metadata) = fs::metadata(path) else {
        return 0;
    };

    if metadata.is_file() {
        return metadata.len();
    }

    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    entries
        .flatten()
        .map(|entry| path_size(&entry.path()))
        .sum::<u64>()
}

fn remove_path(path: &Path) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;

    if metadata.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to remove {}: {error}", path.display()))
    } else {
        fs::remove_file(path)
            .map_err(|error| format!("failed to remove {}: {error}", path.display()))
    }
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
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
