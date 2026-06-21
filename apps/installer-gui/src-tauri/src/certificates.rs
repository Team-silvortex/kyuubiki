use std::fs;
use std::process::Command;

use crate::certificates_openssl::{
    ensure_openssl_available, inspect_certificate, run_command, unix_ms,
};
use crate::certificates_store::{
    inventory_path, material_paths, policy_path, read_inventory, read_policy_config,
    render_subject_alt_name_ext, validate_label, validate_root_common_name, validate_storage_root,
    validate_subject_alt_names, validate_validity_days, write_inventory, write_policy_config,
};
use crate::certificates_types::{
    ActiveCertificateBinding, CERTIFICATE_POLICY_SCHEMA_VERSION, CertificateAuthorityConfig,
    CertificateAuthorityPayload, CertificateRecord, CertificateRecordPayload,
    CertificateRuntimePolicy, IssueNodeCertificatePayload, RevokeNodeCertificatePayload,
    WriteCertificateAuthorityPolicyPayload,
};
use crate::remote::{validate_agent_id, validate_host_token};
use crate::remote_nodes::validate_control_mode;

#[tauri::command]
pub fn certificate_authority_policy() -> Result<CertificateAuthorityPayload, String> {
    build_certificate_authority_payload()
}

pub fn write_certificate_authority_policy(
    payload: WriteCertificateAuthorityPolicyPayload,
) -> Result<String, String> {
    let config = CertificateAuthorityConfig {
        schema_version: CERTIFICATE_POLICY_SCHEMA_VERSION.to_string(),
        storage_root: validate_storage_root(&payload.storage_root)?,
        root_common_name: validate_root_common_name(&payload.root_common_name)?,
        default_validity_days: validate_validity_days(payload.default_validity_days)?,
        require_for_orchestrated: payload.require_for_orchestrated,
        require_for_offline_mesh: payload.require_for_offline_mesh,
        allow_ssh_trust_bootstrap: payload.allow_ssh_trust_bootstrap,
    };
    write_policy_config(&config)?;
    Ok(build_certificate_authority_payload()?.rendered)
}

pub fn initialize_certificate_authority() -> Result<String, String> {
    let config = read_policy_config()?;
    ensure_openssl_available()?;
    let paths = material_paths(&config);
    fs::create_dir_all(&paths.ca_dir)
        .map_err(|error| format!("failed to create {}: {error}", paths.ca_dir.display()))?;
    fs::create_dir_all(&paths.nodes_dir)
        .map_err(|error| format!("failed to create {}: {error}", paths.nodes_dir.display()))?;

    if !paths.ca_key.exists() {
        run_command(
            Command::new("openssl")
                .arg("genrsa")
                .arg("-out")
                .arg(&paths.ca_key)
                .arg("4096"),
            "generate certificate authority key",
        )?;
    }
    if !paths.ca_cert.exists() {
        run_command(
            Command::new("openssl")
                .arg("req")
                .arg("-x509")
                .arg("-new")
                .arg("-key")
                .arg(&paths.ca_key)
                .arg("-sha256")
                .arg("-days")
                .arg("3650")
                .arg("-out")
                .arg(&paths.ca_cert)
                .arg("-subj")
                .arg(format!("/CN={}", config.root_common_name)),
            "generate certificate authority certificate",
        )?;
    }

    Ok(build_certificate_authority_payload()?.rendered)
}

pub fn issue_node_certificate(payload: IssueNodeCertificatePayload) -> Result<String, String> {
    let config = read_policy_config()?;
    ensure_openssl_available()?;
    let paths = material_paths(&config);
    if !paths.ca_cert.exists() || !paths.ca_key.exists() {
        return Err(
            "certificate authority is not initialized; initialize the CA before issuing node certificates"
                .to_string(),
        );
    }

    let label = validate_label(&payload.label)?;
    let target_host = validate_host_token(&payload.target_host, "target host")?;
    let advertise_host = validate_host_token(&payload.advertise_host, "advertise host")?;
    let agent_id = validate_agent_id(&payload.agent_id)?;
    let control_mode = validate_control_mode(payload.control_mode.as_deref())?;
    let validity_days = validate_validity_days(
        payload
            .validity_days
            .unwrap_or(config.default_validity_days),
    )?;
    let san_entries =
        validate_subject_alt_names(&target_host, &advertise_host, payload.subject_alt_names)?;

    let certificate_id = format!("{}-{}", label, unix_ms());
    let node_dir = paths.nodes_dir.join(&certificate_id);
    fs::create_dir_all(&node_dir)
        .map_err(|error| format!("failed to create {}: {error}", node_dir.display()))?;

    let key_path = node_dir.join("node.key.pem");
    let csr_path = node_dir.join("node.csr.pem");
    let cert_path = node_dir.join("node.crt.pem");
    let ext_path = node_dir.join("node.ext");
    let subject = format!("/CN={}", agent_id);

    fs::write(&ext_path, render_subject_alt_name_ext(&san_entries))
        .map_err(|error| format!("failed to write {}: {error}", ext_path.display()))?;

    run_command(
        Command::new("openssl")
            .arg("genrsa")
            .arg("-out")
            .arg(&key_path)
            .arg("2048"),
        "generate node private key",
    )?;
    run_command(
        Command::new("openssl")
            .arg("req")
            .arg("-new")
            .arg("-key")
            .arg(&key_path)
            .arg("-out")
            .arg(&csr_path)
            .arg("-subj")
            .arg(&subject),
        "generate node certificate signing request",
    )?;
    run_command(
        Command::new("openssl")
            .arg("x509")
            .arg("-req")
            .arg("-in")
            .arg(&csr_path)
            .arg("-CA")
            .arg(&paths.ca_cert)
            .arg("-CAkey")
            .arg(&paths.ca_key)
            .arg("-CAcreateserial")
            .arg("-out")
            .arg(&cert_path)
            .arg("-days")
            .arg(validity_days.to_string())
            .arg("-sha256")
            .arg("-extfile")
            .arg(&ext_path),
        "sign node certificate",
    )?;

    let metadata = inspect_certificate(&cert_path)?;
    let mut inventory = read_inventory()?;
    inventory.push(CertificateRecord {
        certificate_id: certificate_id.clone(),
        label,
        target_host,
        advertise_host,
        agent_id,
        control_mode,
        status: "active".to_string(),
        serial: metadata.serial,
        fingerprint: metadata.fingerprint,
        subject: metadata.subject,
        not_after: metadata.not_after,
        cert_path: cert_path.display().to_string(),
        key_path: key_path.display().to_string(),
        issued_at_unix_ms: unix_ms(),
        revoked_at_unix_ms: None,
    });
    write_inventory(&inventory)?;

    Ok(build_certificate_authority_payload()?.rendered)
}

pub fn revoke_node_certificate(payload: RevokeNodeCertificatePayload) -> Result<String, String> {
    let certificate_id = payload.certificate_id.trim();
    if certificate_id.is_empty() {
        return Err("certificate id is required".to_string());
    }
    let mut inventory = read_inventory()?;
    let now = unix_ms();
    let mut found = false;
    for entry in &mut inventory {
        if entry.certificate_id == certificate_id {
            entry.status = "revoked".to_string();
            entry.revoked_at_unix_ms = Some(now);
            found = true;
        }
    }
    if !found {
        return Err(format!("certificate id not found: {certificate_id}"));
    }
    write_inventory(&inventory)?;
    Ok(build_certificate_authority_payload()?.rendered)
}

pub(crate) fn certificate_runtime_policy() -> Result<CertificateRuntimePolicy, String> {
    let config = read_policy_config()?;
    let paths = material_paths(&config);
    Ok(CertificateRuntimePolicy {
        require_for_orchestrated: config.require_for_orchestrated,
        require_for_offline_mesh: config.require_for_offline_mesh,
        ca_cert_path: paths.ca_cert.display().to_string(),
    })
}

pub(crate) fn resolve_runtime_certificate_binding(
    certificate_id: Option<&str>,
    agent_id: &str,
    target_host: &str,
    advertise_host: &str,
    control_mode: &str,
) -> Result<Option<ActiveCertificateBinding>, String> {
    let policy = certificate_runtime_policy()?;
    let inventory = read_inventory()?;
    let required = match control_mode {
        "offline_mesh" => policy.require_for_offline_mesh,
        _ => policy.require_for_orchestrated,
    };

    if let Some(selected_id) = certificate_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let binding = inventory
            .into_iter()
            .find(|entry| entry.certificate_id == selected_id)
            .ok_or_else(|| format!("certificate id not found: {selected_id}"))?;
        if binding.status != "active" {
            return Err(format!(
                "certificate id is not active and cannot be used for agent startup: {selected_id}"
            ));
        }
        validate_binding_contract(
            &binding,
            agent_id,
            target_host,
            advertise_host,
            control_mode,
        )?;
        return Ok(Some(to_runtime_binding(binding, &policy)));
    }

    let matches = inventory
        .into_iter()
        .filter(|entry| entry.status == "active")
        .filter(|entry| entry.agent_id == agent_id)
        .filter(|entry| entry.control_mode == control_mode)
        .filter(|entry| entry.target_host == target_host || entry.advertise_host == advertise_host)
        .collect::<Vec<_>>();

    match matches.len() {
        0 if required => Err(format!(
            "certificate policy requires an active {control_mode} certificate for agent {agent_id}, but none matched target {target_host}"
        )),
        0 => Ok(None),
        1 => Ok(matches
            .into_iter()
            .next()
            .map(|entry| to_runtime_binding(entry, &policy))),
        _ => Err(format!(
            "multiple active certificates match agent {agent_id}; select certificate_id explicitly"
        )),
    }
}

fn build_certificate_authority_payload() -> Result<CertificateAuthorityPayload, String> {
    let config = read_policy_config()?;
    let paths = material_paths(&config);
    let inventory = read_inventory()?;
    let ca_initialized = paths.ca_cert.exists() && paths.ca_key.exists();
    let (ca_fingerprint, ca_subject) = if ca_initialized {
        let metadata = inspect_certificate(&paths.ca_cert)?;
        (metadata.fingerprint, metadata.subject)
    } else {
        (
            "(not initialized)".to_string(),
            "(not initialized)".to_string(),
        )
    };
    let active_count = inventory
        .iter()
        .filter(|entry| entry.status == "active")
        .count();
    let revoked_count = inventory
        .iter()
        .filter(|entry| entry.status == "revoked")
        .count();
    let rendered = [
        "installer certificate authority".to_string(),
        format!("config_path: {}", policy_path().display()),
        format!("inventory_path: {}", inventory_path().display()),
        format!("storage_root: {}", config.storage_root),
        format!("root_common_name: {}", config.root_common_name),
        format!("default_validity_days: {}", config.default_validity_days),
        format!("ca_initialized: {}", ca_initialized),
        format!("ca_cert_path: {}", paths.ca_cert.display()),
        format!("ca_key_path: {}", paths.ca_key.display()),
        format!("ca_fingerprint: {}", ca_fingerprint),
        format!("active_certificates: {}", active_count),
        format!("revoked_certificates: {}", revoked_count),
    ]
    .join("\n");

    Ok(CertificateAuthorityPayload {
        schema_version: config.schema_version,
        config_path: policy_path().display().to_string(),
        inventory_path: inventory_path().display().to_string(),
        storage_root: config.storage_root.clone(),
        root_common_name: config.root_common_name,
        default_validity_days: config.default_validity_days,
        require_for_orchestrated: config.require_for_orchestrated,
        require_for_offline_mesh: config.require_for_offline_mesh,
        allow_ssh_trust_bootstrap: config.allow_ssh_trust_bootstrap,
        ca_initialized,
        ca_cert_path: paths.ca_cert.display().to_string(),
        ca_key_path: paths.ca_key.display().to_string(),
        ca_fingerprint,
        ca_subject,
        active_certificate_count: active_count,
        revoked_certificate_count: revoked_count,
        certificates: inventory.into_iter().map(to_payload_record).collect(),
        rendered,
    })
}

fn to_payload_record(entry: CertificateRecord) -> CertificateRecordPayload {
    CertificateRecordPayload {
        certificate_id: entry.certificate_id,
        label: entry.label,
        target_host: entry.target_host,
        advertise_host: entry.advertise_host,
        agent_id: entry.agent_id,
        control_mode: entry.control_mode,
        status: entry.status,
        serial: entry.serial,
        fingerprint: entry.fingerprint,
        subject: entry.subject,
        not_after: entry.not_after,
        cert_path: entry.cert_path,
        key_path: entry.key_path,
        issued_at_unix_ms: entry.issued_at_unix_ms,
        revoked_at_unix_ms: entry.revoked_at_unix_ms,
    }
}

fn to_runtime_binding(
    entry: CertificateRecord,
    policy: &CertificateRuntimePolicy,
) -> ActiveCertificateBinding {
    ActiveCertificateBinding {
        certificate_id: entry.certificate_id,
        fingerprint: entry.fingerprint,
        cert_path: entry.cert_path,
        key_path: entry.key_path,
        ca_cert_path: policy.ca_cert_path.clone(),
    }
}

fn validate_binding_contract(
    binding: &CertificateRecord,
    agent_id: &str,
    target_host: &str,
    advertise_host: &str,
    control_mode: &str,
) -> Result<(), String> {
    if binding.agent_id != agent_id {
        return Err(format!(
            "certificate {} was issued for agent {}, not {}",
            binding.certificate_id, binding.agent_id, agent_id
        ));
    }
    if binding.control_mode != control_mode {
        return Err(format!(
            "certificate {} was issued for control mode {}, not {}",
            binding.certificate_id, binding.control_mode, control_mode
        ));
    }
    if binding.target_host != target_host {
        return Err(format!(
            "certificate {} was issued for target host {}, not {}",
            binding.certificate_id, binding.target_host, target_host
        ));
    }
    if binding.advertise_host != advertise_host {
        return Err(format!(
            "certificate {} was issued for advertise host {}, not {}",
            binding.certificate_id, binding.advertise_host, advertise_host
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::certificates_store::render_subject_alt_name_ext;

    #[test]
    fn renders_subject_alt_name_entries_for_dns_and_ip() {
        let rendered = render_subject_alt_name_ext(&[
            "solver-a.local".to_string(),
            "192.168.1.12".to_string(),
        ]);
        assert!(rendered.contains("DNS.1 = solver-a.local"));
        assert!(rendered.contains("IP.2 = 192.168.1.12"));
    }
}
