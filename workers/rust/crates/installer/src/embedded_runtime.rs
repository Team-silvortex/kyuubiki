use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::{Platform, workspace_root};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddedRuntimeReport {
    pub platform: String,
    pub manifest_status: String,
    pub entries: Vec<EmbeddedRuntimeEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddedRuntimeEntry {
    pub id: String,
    pub version: String,
    pub target_dir: String,
    pub required: bool,
}

impl EmbeddedRuntimeReport {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki embedded runtimes".to_string(),
            format!("platform: {}", self.platform),
            format!("manifest: {}", self.manifest_status),
        ];

        for entry in &self.entries {
            lines.push(format!(
                "[{}] {} {} -> {}",
                if entry.required {
                    "required"
                } else {
                    "optional"
                },
                entry.id,
                entry.version,
                entry.target_dir
            ));
        }

        lines.join("\n")
    }
}

pub fn embedded_runtime_report() -> Result<EmbeddedRuntimeReport, String> {
    let root = workspace_root();
    let platform = Platform::current();
    let manifest = embedded_runtime_manifest_value(&root, platform)?;
    let entries = manifest
        .get("runtimes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|runtime| EmbeddedRuntimeEntry {
            id: string_field(&runtime, "id"),
            version: string_field(&runtime, "version"),
            target_dir: string_field(&runtime, "target_dir"),
            required: runtime
                .get("required_for_self_host")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
        .collect();

    Ok(EmbeddedRuntimeReport {
        platform: platform.as_str().to_string(),
        manifest_status: "generated from config/toolchains.json".to_string(),
        entries,
    })
}

pub fn build_embedded_runtime_manifest(root: &Path, platform: Platform) -> Result<String, String> {
    let manifest = embedded_runtime_manifest_value(root, platform)?;
    serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())
}

fn embedded_runtime_manifest_value(root: &Path, platform: Platform) -> Result<Value, String> {
    let contract = read_toolchain_contract(root)?;
    let elixir = contract
        .get("elixir")
        .ok_or_else(|| "config/toolchains.json is missing elixir".to_string())?;
    let node = contract
        .get("node")
        .ok_or_else(|| "config/toolchains.json is missing node".to_string())?;

    Ok(json!({
        "schema_version": "kyuubiki.embedded-runtimes/v1",
        "platform": platform.as_str(),
        "owner": "installer",
        "policy": {
            "mode": "bundled_first",
            "description": "Kyuubiki self-host deployments should prefer installer-managed runtimes before falling back to host-installed tools.",
            "host_fallback": "allowed_for_development_only"
        },
        "layout": {
            "root": "runtimes",
            "manifest": "manifests/embedded-runtimes.json",
            "path_injection": "prepend runtime bin paths from this manifest before launching services"
        },
        "runtimes": [
            {
                "id": "elixir-otp",
                "kind": "control-plane-runtime",
                "version": string_value(elixir, "lab_elixir"),
                "minimum": string_value(elixir, "minimum"),
                "otp_minimum": string_value(elixir, "otp_minimum"),
                "target_dir": format!("runtimes/{}/elixir-otp", platform.as_str()),
                "bin_dirs": [
                    format!("runtimes/{}/elixir-otp/otp/bin", platform.as_str()),
                    format!("runtimes/{}/elixir-otp/elixir/bin", platform.as_str())
                ],
                "required_for_self_host": true,
                "used_by": ["orchestrator", "workflow-mesh-regression", "headless-live-test"],
                "source_contract": "config/toolchains.json#/elixir"
            },
            {
                "id": "node",
                "kind": "ui-and-tooling-runtime",
                "version": string_value(node, "preferred"),
                "minimum": string_value(node, "minimum"),
                "target_dir": format!("runtimes/{}/node", platform.as_str()),
                "bin_dirs": [format!("runtimes/{}/node/bin", platform.as_str())],
                "required_for_self_host": true,
                "used_by": ["frontend", "runtime-scripts", "docs-and-contract-checks"],
                "source_contract": "config/toolchains.json#/node"
            }
        ],
        "visibility": {
            "user_editable": false,
            "operator_visible": true,
            "notes": [
                "Runtime payloads may be downloaded or mounted by the installer, but their target paths and versions remain manifest-visible.",
                "The release scaffold can be created before payload download; missing payloads are deployment blockers, not hidden host requirements."
            ]
        }
    }))
}

fn read_toolchain_contract(root: &Path) -> Result<Value, String> {
    let path = root.join("config").join("toolchains.json");
    let bytes =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn string_value(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_default()
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| "--".to_string())
}
