use std::path::PathBuf;

use serde::Serialize;

pub(crate) const CERTIFICATE_POLICY_SCHEMA_VERSION: &str =
    "kyuubiki.installer.certificate-policy/v1";

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteCertificateAuthorityPolicyPayload {
    pub storage_root: String,
    pub root_common_name: String,
    pub default_validity_days: u32,
    pub require_for_orchestrated: bool,
    pub require_for_offline_mesh: bool,
    pub allow_ssh_trust_bootstrap: bool,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueNodeCertificatePayload {
    pub label: String,
    pub target_host: String,
    pub advertise_host: String,
    pub agent_id: String,
    pub control_mode: Option<String>,
    pub validity_days: Option<u32>,
    pub subject_alt_names: Option<Vec<String>>,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeNodeCertificatePayload {
    pub certificate_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateRecordPayload {
    pub certificate_id: String,
    pub label: String,
    pub target_host: String,
    pub advertise_host: String,
    pub agent_id: String,
    pub control_mode: String,
    pub status: String,
    pub serial: String,
    pub fingerprint: String,
    pub subject: String,
    pub not_after: String,
    pub cert_path: String,
    pub key_path: String,
    pub issued_at_unix_ms: u64,
    pub revoked_at_unix_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateAuthorityPayload {
    pub schema_version: String,
    pub config_path: String,
    pub inventory_path: String,
    pub storage_root: String,
    pub root_common_name: String,
    pub default_validity_days: u32,
    pub require_for_orchestrated: bool,
    pub require_for_offline_mesh: bool,
    pub allow_ssh_trust_bootstrap: bool,
    pub ca_initialized: bool,
    pub ca_cert_path: String,
    pub ca_key_path: String,
    pub ca_fingerprint: String,
    pub ca_subject: String,
    pub active_certificate_count: usize,
    pub revoked_certificate_count: usize,
    pub certificates: Vec<CertificateRecordPayload>,
    pub rendered: String,
}

#[derive(Clone, Debug)]
pub(crate) struct CertificateRuntimePolicy {
    pub require_for_orchestrated: bool,
    pub require_for_offline_mesh: bool,
    pub ca_cert_path: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveCertificateBinding {
    pub certificate_id: String,
    pub fingerprint: String,
    pub cert_path: String,
    pub key_path: String,
    pub ca_cert_path: String,
}

#[derive(Clone, Debug)]
pub(crate) struct CertificateAuthorityConfig {
    pub schema_version: String,
    pub storage_root: String,
    pub root_common_name: String,
    pub default_validity_days: u32,
    pub require_for_orchestrated: bool,
    pub require_for_offline_mesh: bool,
    pub allow_ssh_trust_bootstrap: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct CertificateRecord {
    pub certificate_id: String,
    pub label: String,
    pub target_host: String,
    pub advertise_host: String,
    pub agent_id: String,
    pub control_mode: String,
    pub status: String,
    pub serial: String,
    pub fingerprint: String,
    pub subject: String,
    pub not_after: String,
    pub cert_path: String,
    pub key_path: String,
    pub issued_at_unix_ms: u64,
    pub revoked_at_unix_ms: Option<u64>,
}

pub(crate) struct MaterialPaths {
    pub ca_dir: PathBuf,
    pub nodes_dir: PathBuf,
    pub ca_key: PathBuf,
    pub ca_cert: PathBuf,
}

pub(crate) struct CertificateMetadata {
    pub serial: String,
    pub fingerprint: String,
    pub subject: String,
    pub not_after: String,
}
