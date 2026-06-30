use std::fs;
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};

use kyuubiki_installer::{credential_sandbox_root, workspace_root};
use serde_json::{Value, json};

use crate::certificates_types::{
    CERTIFICATE_POLICY_SCHEMA_VERSION, CertificateAuthorityConfig, CertificateRecord, MaterialPaths,
};
use crate::remote::validate_host_token;

pub(crate) fn policy_path() -> PathBuf {
    workspace_root()
        .join("config")
        .join("installer-certificate-policy.json")
}

pub(crate) fn inventory_path() -> PathBuf {
    credential_sandbox_root(&workspace_root())
        .join("installer")
        .join("certificates")
        .join("inventory.json")
}

pub(crate) fn default_storage_root() -> String {
    credential_sandbox_root(&workspace_root())
        .join("installer")
        .join("certificates")
        .display()
        .to_string()
}

pub(crate) fn read_policy_config() -> Result<CertificateAuthorityConfig, String> {
    let path = policy_path();
    if !path.exists() {
        return Ok(CertificateAuthorityConfig {
            schema_version: CERTIFICATE_POLICY_SCHEMA_VERSION.to_string(),
            storage_root: default_storage_root(),
            root_common_name: "kyuubiki-local-ca".to_string(),
            default_validity_days: 365,
            require_for_orchestrated: false,
            require_for_offline_mesh: true,
            allow_ssh_trust_bootstrap: true,
        });
    }

    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;

    Ok(CertificateAuthorityConfig {
        schema_version: value
            .get("schema_version")
            .and_then(Value::as_str)
            .unwrap_or(CERTIFICATE_POLICY_SCHEMA_VERSION)
            .to_string(),
        storage_root: validate_storage_root(
            value
                .get("storage_root")
                .and_then(Value::as_str)
                .unwrap_or(&default_storage_root()),
        )?,
        root_common_name: validate_root_common_name(
            value
                .get("root_common_name")
                .and_then(Value::as_str)
                .unwrap_or("kyuubiki-local-ca"),
        )?,
        default_validity_days: validate_validity_days(
            value
                .get("default_validity_days")
                .and_then(Value::as_u64)
                .unwrap_or(365) as u32,
        )?,
        require_for_orchestrated: value
            .get("require_for_orchestrated")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        require_for_offline_mesh: value
            .get("require_for_offline_mesh")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        allow_ssh_trust_bootstrap: value
            .get("allow_ssh_trust_bootstrap")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    })
}

pub(crate) fn write_policy_config(config: &CertificateAuthorityConfig) -> Result<(), String> {
    let path = policy_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "schema_version": config.schema_version,
            "storage_root": config.storage_root,
            "root_common_name": config.root_common_name,
            "default_validity_days": config.default_validity_days,
            "require_for_orchestrated": config.require_for_orchestrated,
            "require_for_offline_mesh": config.require_for_offline_mesh,
            "allow_ssh_trust_bootstrap": config.allow_ssh_trust_bootstrap,
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(())
}

pub(crate) fn read_inventory() -> Result<Vec<CertificateRecord>, String> {
    let path = if inventory_path().exists() {
        inventory_path()
    } else {
        legacy_inventory_path()
    };
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok(value
        .get("certificates")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(read_inventory_record).collect())
        .unwrap_or_default())
}

pub(crate) fn write_inventory(records: &[CertificateRecord]) -> Result<(), String> {
    let path = inventory_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "schema_version": CERTIFICATE_POLICY_SCHEMA_VERSION,
            "certificates": records.iter().map(|entry| json!({
                "certificate_id": entry.certificate_id,
                "label": entry.label,
                "target_host": entry.target_host,
                "advertise_host": entry.advertise_host,
                "agent_id": entry.agent_id,
                "control_mode": entry.control_mode,
                "status": entry.status,
                "serial": entry.serial,
                "fingerprint": entry.fingerprint,
                "subject": entry.subject,
                "not_after": entry.not_after,
                "cert_path": entry.cert_path,
                "key_path": entry.key_path,
                "issued_at_unix_ms": entry.issued_at_unix_ms,
                "revoked_at_unix_ms": entry.revoked_at_unix_ms,
            })).collect::<Vec<_>>(),
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(())
}

pub(crate) fn validate_storage_root(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("certificate storage root is required".to_string());
    }
    let path = Path::new(trimmed);
    if !path.is_absolute() {
        return Err("certificate storage root must be an absolute path".to_string());
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("certificate storage root must not contain parent traversal".to_string());
    }
    let sandbox_root = credential_sandbox_root(&workspace_root());
    if !path.starts_with(&sandbox_root) {
        return Err(format!(
            "certificate storage root must stay inside Kyuubiki credential sandbox: {}",
            sandbox_root.display()
        ));
    }
    reject_symlinked_existing_ancestor(path, &sandbox_root)?;
    Ok(trimmed.to_string())
}

pub(crate) fn validate_root_common_name(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("root common name is required".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\n') || trimmed.contains('\r') {
        return Err("root common name contains unsupported characters".to_string());
    }
    Ok(trimmed.to_string())
}

pub(crate) fn validate_label(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("certificate label is required".to_string());
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        return Err("certificate label contains unsupported characters".to_string());
    }
    Ok(trimmed.to_string())
}

pub(crate) fn validate_validity_days(days: u32) -> Result<u32, String> {
    if !(1..=3650).contains(&days) {
        return Err("certificate validity days must be between 1 and 3650".to_string());
    }
    Ok(days)
}

pub(crate) fn validate_subject_alt_names(
    target_host: &str,
    advertise_host: &str,
    input: Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    let mut values = vec![target_host.to_string(), advertise_host.to_string()];
    values.extend(input.unwrap_or_default());
    let mut normalized = Vec::new();
    for item in values {
        let host = validate_host_token(&item, "subject alt name")?;
        if !normalized.iter().any(|existing| existing == &host) {
            normalized.push(host);
        }
    }
    Ok(normalized)
}

pub(crate) fn render_subject_alt_name_ext(entries: &[String]) -> String {
    let values = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let prefix = if entry.parse::<IpAddr>().is_ok() {
                "IP"
            } else {
                "DNS"
            };
            format!("{prefix}.{} = {entry}", index + 1)
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("[v3_req]\nsubjectAltName = @alt_names\n[alt_names]\n{values}\n")
}

pub(crate) fn material_paths(config: &CertificateAuthorityConfig) -> MaterialPaths {
    let storage_root = PathBuf::from(&config.storage_root);
    let ca_dir = storage_root.join("authority");
    let nodes_dir = storage_root.join("nodes");
    MaterialPaths {
        ca_key: ca_dir.join("root-ca.key.pem"),
        ca_cert: ca_dir.join("root-ca.crt.pem"),
        ca_dir,
        nodes_dir,
    }
}

fn legacy_inventory_path() -> PathBuf {
    workspace_root()
        .join("config")
        .join("installer-certificates.json")
}

fn reject_symlinked_existing_ancestor(path: &Path, sandbox_root: &Path) -> Result<(), String> {
    let suffix = path
        .strip_prefix(sandbox_root)
        .map_err(|_| "certificate storage root must stay inside credential sandbox".to_string())?;
    let mut current = sandbox_root.to_path_buf();
    reject_symlink(&current)?;
    for component in suffix.components() {
        current.push(component.as_os_str());
        if current.exists() {
            reject_symlink(&current)?;
        }
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("failed to inspect {}: {error}", path.display())),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "certificate storage root must not pass through symlinked path: {}",
            path.display()
        ));
    }
    Ok(())
}

fn read_inventory_record(value: &Value) -> Option<CertificateRecord> {
    Some(CertificateRecord {
        certificate_id: value.get("certificate_id")?.as_str()?.to_string(),
        label: value.get("label")?.as_str()?.to_string(),
        target_host: value.get("target_host")?.as_str()?.to_string(),
        advertise_host: value.get("advertise_host")?.as_str()?.to_string(),
        agent_id: value.get("agent_id")?.as_str()?.to_string(),
        control_mode: value.get("control_mode")?.as_str()?.to_string(),
        status: value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("active")
            .to_string(),
        serial: value
            .get("serial")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        fingerprint: value
            .get("fingerprint")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        subject: value
            .get("subject")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        not_after: value
            .get("not_after")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        cert_path: value
            .get("cert_path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        key_path: value
            .get("key_path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        issued_at_unix_ms: value
            .get("issued_at_unix_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        revoked_at_unix_ms: value.get("revoked_at_unix_ms").and_then(Value::as_u64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_storage_root() {
        let error = validate_storage_root("./config/certificates").unwrap_err();
        assert!(error.contains("absolute path"));
    }

    #[test]
    fn rejects_storage_root_outside_credential_sandbox() {
        let error = validate_storage_root("/tmp/kyuubiki-certificates").unwrap_err();
        assert!(error.contains("credential sandbox"));
    }

    #[test]
    fn accepts_storage_root_inside_credential_sandbox() {
        let root = default_storage_root();
        assert_eq!(validate_storage_root(&root).unwrap(), root);
    }

    #[test]
    fn inventory_defaults_to_credential_sandbox() {
        let path = inventory_path();
        assert!(path.starts_with(credential_sandbox_root(&workspace_root())));
        assert!(path.ends_with("installer/certificates/inventory.json"));
    }

    #[test]
    fn rejects_parent_traversal_storage_root() {
        let root = credential_sandbox_root(&workspace_root())
            .join("installer")
            .join("..")
            .join("certificates")
            .display()
            .to_string();
        let error = validate_storage_root(&root).unwrap_err();
        assert!(error.contains("parent traversal"));
    }

    #[test]
    fn rejects_invalid_validity_days() {
        let error = validate_validity_days(0).unwrap_err();
        assert!(error.contains("between 1 and 3650"));
    }
}
