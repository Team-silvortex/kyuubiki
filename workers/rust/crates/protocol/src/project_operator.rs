//! Project intent and language-neutral task construction. No store login or engine loading.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectOperatorPin {
    pub package_id: String,
    pub package_version: String,
    pub manifest_sha256: String,
    pub entrypoint_sha256: String,
    pub operators: Vec<ProjectOperatorEntry>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectOperatorEntry {
    pub operator_id: String,
    pub kind: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectOperatorRequest {
    pub project_id: String,
    pub scope: String,
    pub operation: ProjectOperatorOperation,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectOperatorOperation {
    List,
    ReadDeployment {
        package: ProjectOperatorPin,
        offset: u64,
    },
    Enable {
        package: ProjectOperatorPin,
    },
    Disable {
        package: ProjectOperatorPin,
    },
    Prepare {
        package: ProjectOperatorPin,
        operator_id: String,
        task_id: String,
        input: Value,
        config: Value,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectOperatorStatus {
    pub schema_version: String,
    pub project_id: String,
    pub scope: String,
    pub packages_root: String,
    pub installed: Vec<ProjectOperatorPin>,
    pub installed_error: Option<String>,
    pub enabled: Vec<ProjectOperatorState>,
    pub task: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment: Option<crate::OperatorDeploymentChunk>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectOperatorState {
    pub package: ProjectOperatorPin,
    pub state: String,
}
pub fn valid_project_operator_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
pub fn valid_operator_component(value: &str) -> bool {
    let upper = value
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    !value.is_empty()
        && value.len() <= 200
        && !value.starts_with('.')
        && !value.ends_with('.')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        && !matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && matches!(upper.as_bytes()[3], b'1'..=b'9'))
}
impl ProjectOperatorPin {
    pub fn validate(&self) -> Result<(), String> {
        let hash = |s: &str| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        let mut ids = std::collections::HashSet::new();
        if !valid_operator_component(&self.package_id)
            || !valid_operator_component(&self.package_version)
            || !self.package_version.as_bytes()[0].is_ascii_digit()
            || !hash(&self.manifest_sha256)
            || !hash(&self.entrypoint_sha256)
            || self.operators.is_empty()
            || self.operators.len() > 128
            || self.operators.iter().any(|o| {
                !valid_operator_component(&o.operator_id)
                    || !matches!(o.kind.as_str(), "extract" | "transform" | "solve")
                    || !ids.insert(&o.operator_id)
            })
        {
            return Err("project_operator_invalid_pin".into());
        }
        Ok(())
    }
}

/// Builds a task only; the caller must independently verify project enablement and live package bytes.
pub fn build_project_operator_task(
    project: &str,
    pin: &ProjectOperatorPin,
    operator: &str,
    task_id: &str,
    input: Value,
    config: Value,
) -> Result<Value, String> {
    pin.validate()?;
    if !valid_project_operator_id(project)
        || !valid_operator_component(task_id)
        || !input.is_object()
        || !config.is_object()
        || serde_json::to_vec(&(&input, &config))
            .map_err(|_| "project_operator_invalid_input")?
            .len()
            > 65536
    {
        return Err("project_operator_invalid_input".into());
    }
    let entry = pin
        .operators
        .iter()
        .find(|o| o.operator_id == operator)
        .ok_or("project_operator_not_exported")?;
    let family = operator
        .split_once('.')
        .map_or(operator, |(_, family)| family);
    let package_ref = format!("bundle://{}", pin.package_id);
    let mut task = json!({"schema_version":"kyuubiki.operator-task-ir/v1","task_id":task_id,
        "operator":{"id":operator,"family":family,"kind":entry.kind},
        "descriptor_authoring":{"schema_version":"kyuubiki.operator-descriptor-authoring/v1","mode":"rust_native",
            "runtime":"rust","source":"project_operator","hot_reloadable":false,"execution_language":"language_neutral"},
        "node":{},"input_artifact":input,"config":config,
        "execution_program":{"schema_version":"kyuubiki.operator-execution-program/v1","program_id":operator,
            "program_family":family,"program_kind":entry.kind,"operator_category_id":null,
            "package_ref":package_ref,"package_version":pin.package_version,
            "package_integrity":{"algorithm":"sha256","digest":pin.entrypoint_sha256},
            "runtime_protocol":"kyuubiki.operator-execution/v1",
            "abi":{"kind":"operator_task","input_encoding":"json","output_encoding":"json"},
            "entrypoint":{"kind":"operator_id","name":operator,"operator_kind":entry.kind},
            "bindings":{"input_artifact":"task.input_artifact","config":"task.config","output_artifact":"task.output_artifact"},
            "node_binding":{"node_id":null,"input_ports":[],"output_ports":[]}},
        "dataset_contract":{},"orchestration_context":{"project_id":project},
        "runtime_hints":{"authority_mode":"agent_local","execution_mode":"local_bundle","cache_scope":"agent",
            "agent_fetchable":false,"operator_kind":entry.kind,"package_ref":package_ref,"package_version":pin.package_version}});
    crate::seal_operator_task_digest(&mut task, crate::TASK_DIGEST_TYPED_V2)?;
    let summary = crate::summarize_operator_task_execution_checked(&task)
        .map_err(|_| "project_operator_invalid_task")?;
    if !crate::build_operator_task_admission_report(&task, &summary).accepted {
        return Err("project_operator_admission_rejected".into());
    }
    Ok(task)
}
