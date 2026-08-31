use kyuubiki_installer::{
    Platform, active_agent_binary_in, agent_update_status_in, install_agent_update_package_into,
    prepare_agent_update_package,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(super) const SETUP_SCHEMA: &str = "kyuubiki.operator-package-acquisition-host-setup/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct InstallationEvidence {
    pub(super) schema_version: String,
    pub(super) installer_owner: String,
    pub(super) package_schema: String,
    pub(super) activation_schema: String,
    pub(super) package_version: String,
    pub(super) platform: String,
    pub(super) entrypoint_sha256: String,
    pub(super) entrypoint_size_bytes: u64,
    pub(super) activation_generation: u64,
    pub(super) active_version: String,
    pub(super) installed_version_count: usize,
    pub(super) operator_cache_initially_empty: bool,
}

pub(super) fn prepare(
    agent_binary: &Path,
    work_root: &Path,
    output: &Path,
    package_version: &str,
) -> RunnerResult<InstallationEvidence> {
    if Platform::current() != Platform::Linux {
        return Err("package acquisition host setup requires Linux".to_string());
    }
    prepare_empty_root(work_root)?;
    let package_root = work_root.join("agent-package");
    let store_root = work_root.join("agent-store");
    let operator_packages_root = work_root.join("operator-store/packages");
    fs::create_dir_all(&operator_packages_root)
        .map_err(|error| format!("failed to create operator cache: {error}"))?;

    let package = prepare_agent_update_package(
        agent_binary,
        &package_root,
        package_version,
        Platform::Linux,
    )?;
    let activation =
        install_agent_update_package_into(&package_root, &store_root, Platform::Linux)?;
    let active_binary = active_agent_binary_in(&store_root, Platform::Linux)?;
    if !active_binary.is_file() {
        return Err("Installer activation did not expose an Agent binary".to_string());
    }
    let status = agent_update_status_in(&store_root)?;
    let evidence = InstallationEvidence {
        schema_version: SETUP_SCHEMA.to_string(),
        installer_owner: "kyuubiki-installer".to_string(),
        package_schema: package.schema_version,
        activation_schema: activation.schema_version,
        package_version: package.version,
        platform: package.platform,
        entrypoint_sha256: package.entrypoint_sha256,
        entrypoint_size_bytes: package.entrypoint_size_bytes,
        activation_generation: activation.generation,
        active_version: status
            .active_version
            .ok_or("Installer status omitted the active Agent version")?,
        installed_version_count: status.installed_versions.len(),
        operator_cache_initially_empty: directory_empty(&operator_packages_root)?,
    };
    write_json(output, &evidence)?;
    Ok(evidence)
}

pub(super) fn read(path: &Path) -> RunnerResult<InstallationEvidence> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read host setup {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid host setup {}: {error}", path.display()))
}

fn prepare_empty_root(path: &Path) -> RunnerResult<()> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to reset {}: {error}", path.display()))?;
    }
    fs::create_dir_all(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))
}

fn directory_empty(path: &Path) -> RunnerResult<bool> {
    Ok(fs::read_dir(path)
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?
        .next()
        .is_none())
}

fn write_json(path: &Path, value: &impl Serialize) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("failed to encode host setup: {error}"))?;
    fs::write(path, bytes)
        .map_err(|error| format!("failed to write host setup {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_schema_is_specific_to_package_acquisition() {
        assert_eq!(
            SETUP_SCHEMA,
            "kyuubiki.operator-package-acquisition-host-setup/v1"
        );
    }
}
