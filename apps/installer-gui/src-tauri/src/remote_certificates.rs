use std::path::Path;

use crate::certificates::resolve_runtime_certificate_binding;
use crate::certificates_types::ActiveCertificateBinding;
use crate::remote_exec::{run_remote_scp_files, run_remote_ssh_command};

#[derive(Clone, Debug)]
pub(crate) struct PreparedRemoteCertificateMaterial {
    pub certificate_id: String,
    pub fingerprint: String,
    pub remote_cert_path: String,
    pub remote_key_path: String,
    pub remote_ca_cert_path: String,
    pub remote_env_exports: Vec<(String, String)>,
}

pub(crate) fn prepare_remote_certificate_material(
    ssh_port: Option<u16>,
    target: &str,
    remote_workspace: &str,
    certificate_id: Option<&str>,
    agent_id: &str,
    target_host: &str,
    advertise_host: &str,
    control_mode: &str,
) -> Result<Option<PreparedRemoteCertificateMaterial>, String> {
    let Some(binding) = resolve_runtime_certificate_binding(
        certificate_id,
        agent_id,
        target_host,
        advertise_host,
        control_mode,
    )?
    else {
        return Ok(None);
    };

    let remote_dir = format!(
        "{}/runtime/agent-certificates/{}",
        remote_workspace.trim_end_matches('/'),
        binding.certificate_id
    );
    run_remote_ssh_command(
        ssh_port,
        target,
        &format!(
            "mkdir -p {dir} && chmod 700 {dir}",
            dir = shell_escape(&remote_dir)
        ),
    )?;

    run_remote_scp_files(
        ssh_port,
        target,
        &remote_dir,
        &[
            binding.cert_path.as_str(),
            binding.key_path.as_str(),
            binding.ca_cert_path.as_str(),
        ],
    )?;

    let remote_cert_path = join_remote_path(&remote_dir, &binding.cert_path)?;
    let remote_key_path = join_remote_path(&remote_dir, &binding.key_path)?;
    let remote_ca_cert_path = join_remote_path(&remote_dir, &binding.ca_cert_path)?;
    let remote_env_exports = runtime_env_exports(
        &binding,
        &remote_cert_path,
        &remote_key_path,
        &remote_ca_cert_path,
    );
    run_remote_ssh_command(
        ssh_port,
        target,
        &format!(
            "chmod 600 {key} && chmod 644 {cert} {ca}",
            key = shell_escape(&remote_key_path),
            cert = shell_escape(&remote_cert_path),
            ca = shell_escape(&remote_ca_cert_path)
        ),
    )?;

    Ok(Some(PreparedRemoteCertificateMaterial {
        certificate_id: binding.certificate_id,
        fingerprint: binding.fingerprint,
        remote_cert_path: remote_cert_path.clone(),
        remote_key_path: remote_key_path.clone(),
        remote_ca_cert_path: remote_ca_cert_path.clone(),
        remote_env_exports,
    }))
}

fn runtime_env_exports(
    binding: &ActiveCertificateBinding,
    remote_cert_path: &str,
    remote_key_path: &str,
    remote_ca_cert_path: &str,
) -> Vec<(String, String)> {
    vec![
        (
            "KYUUBIKI_AGENT_CERTIFICATE_ID".to_string(),
            binding.certificate_id.clone(),
        ),
        (
            "KYUUBIKI_AGENT_CERT_PATH".to_string(),
            remote_cert_path.to_string(),
        ),
        (
            "KYUUBIKI_AGENT_KEY_PATH".to_string(),
            remote_key_path.to_string(),
        ),
        (
            "KYUUBIKI_AGENT_CA_CERT_PATH".to_string(),
            remote_ca_cert_path.to_string(),
        ),
        (
            "KYUUBIKI_AGENT_FINGERPRINT".to_string(),
            binding.fingerprint.clone(),
        ),
    ]
}

fn join_remote_path(remote_dir: &str, local_source: &str) -> Result<String, String> {
    let name = Path::new(local_source)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("failed to derive remote filename from {local_source}"))?;
    Ok(format!("{}/{}", remote_dir.trim_end_matches('/'), name))
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
