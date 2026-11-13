//! Local Installer handoff, not a store authorization or permission to execute code.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StoreOperatorInstallRequest {
    pub schema_version: String,
    pub package_id: String,
    pub package_version: String,
    pub platform: String,
    pub files: Vec<StoreInstallFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StoreInstallFile {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
}
