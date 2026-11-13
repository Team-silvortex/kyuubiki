//! Package delivery is control-plane work, never a store grant or a solver task.
use crate::ProjectOperatorPin;
use serde::{Deserialize, Serialize};
pub const OPERATOR_DEPLOYMENT_SCHEMA: &str = "kyuubiki.operator-deployment/v1";
pub const OPERATOR_DEPLOYMENT_MAX_BYTES: u64 = 32 * 1024 * 1024;
pub const OPERATOR_DEPLOYMENT_CHUNK_BYTES: usize = 65536;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorDeploymentSpec {
    pub package: ProjectOperatorPin,
    pub platform: String,
    pub manifest_json: String,
    pub size_bytes: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperatorDeploymentAction {
    Inspect,
    Begin { spec: OperatorDeploymentSpec },
    Chunk { offset: u64, data: Vec<u8> },
    Commit,
    Abort,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorDeploymentRequest {
    pub package: ProjectOperatorPin,
    pub expected_process_instance_id: Option<String>,
    pub operation: OperatorDeploymentAction,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorDeploymentStatus {
    pub schema_version: String,
    pub package: ProjectOperatorPin,
    pub platform: String,
    pub state: String,
    pub installed: Option<ProjectOperatorPin>,
    pub active: Option<ProjectOperatorPin>,
    pub received_bytes: u64,
    pub total_bytes: u64,
    pub process_instance_id: Option<String>,
    pub deployment_allowed: bool,
}
impl OperatorDeploymentStatus {
    /// A ready label alone is not evidence that the installed code is loaded.
    pub fn validate(&self) -> Result<(), String> {
        self.package.validate()?;
        for pin in [&self.installed, &self.active].into_iter().flatten() {
            pin.validate()?;
            if pin.package_id != self.package.package_id {
                return Err("deployment_invalid_receipt".into());
            }
        }
        let matches = self.installed.as_ref() == Some(&self.package);
        let valid_state = match self.state.as_str() {
            "missing" => self.installed.is_none() && self.total_bytes == 0,
            "uploading" => self.installed.is_none() && self.total_bytes > 0,
            "installed_restart_required" => matches,
            "ready" => matches && self.active.as_ref() == Some(&self.package),
            "version_conflict" => self.installed.is_some() && !matches,
            _ => false,
        };
        if self.schema_version != OPERATOR_DEPLOYMENT_SCHEMA
            || !valid_state
            || self.platform.is_empty()
            || self.platform.len() > 128
            || !self
                .platform
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || self.received_bytes > self.total_bytes
            || self.total_bytes > OPERATOR_DEPLOYMENT_MAX_BYTES
            || self
                .process_instance_id
                .as_ref()
                .is_some_and(|p| p.is_empty() || p.len() > 256 || p.chars().any(char::is_control))
        {
            return Err("deployment_invalid_receipt".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorDeploymentChunk {
    pub spec: OperatorDeploymentSpec,
    pub offset: u64,
    pub data: Vec<u8>,
}
