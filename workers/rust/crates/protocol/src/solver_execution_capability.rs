use crate::{
    OperatorTaskExecutionPreview, OperatorTaskExecutionSummary, OperatorTaskSummaryError,
    preview_operator_task_execution, summarize_operator_task_execution_checked,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverExecutionCapability {
    pub capability_id: String,
    pub runtime_protocols: Vec<String>,
    pub program_kinds: Vec<String>,
    pub abi_kinds: Vec<String>,
    pub entrypoint_kinds: Vec<String>,
    pub operator_ids: Vec<String>,
    pub execution_modes: Vec<String>,
    pub authority_modes: Vec<String>,
    pub result_serializations: Vec<String>,
    pub supports_package_fetch: bool,
    pub supports_offline_execution: bool,
}

impl SolverExecutionCapability {
    pub fn agent_builtin() -> Self {
        Self {
            capability_id: "agent-builtin-solver-execution".to_string(),
            runtime_protocols: vec![
                "kyuubiki.solver-rpc/v1".to_string(),
                "kyuubiki.operator-execution/v1".to_string(),
            ],
            program_kinds: vec!["solver".to_string(), "transform".to_string()],
            abi_kinds: vec!["solver_rpc".to_string(), "operator_task".to_string()],
            entrypoint_kinds: vec!["solver_method".to_string(), "operator_id".to_string()],
            operator_ids: Vec::new(),
            execution_modes: vec![
                "local_builtin".to_string(),
                "local_bundle".to_string(),
                "agent_native".to_string(),
            ],
            authority_modes: vec!["offline_mesh".to_string(), "agent_local".to_string()],
            result_serializations: vec!["json".to_string()],
            supports_package_fetch: false,
            supports_offline_execution: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverExecutionCapabilityReport {
    pub capability_id: String,
    pub accepted: bool,
    pub task_id: String,
    pub operator_id: String,
    pub operator_kind: String,
    pub program_kind: String,
    pub runtime_protocol: String,
    pub dispatch_route: String,
    pub rejection_reasons: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn check_operator_task_execution_capability(
    task: &Value,
    capability: &SolverExecutionCapability,
) -> Result<SolverExecutionCapabilityReport, OperatorTaskSummaryError> {
    let summary = summarize_operator_task_execution_checked(task)?;
    let preview = preview_operator_task_execution(task)?;
    Ok(build_capability_report(&summary, &preview, capability))
}

fn build_capability_report(
    summary: &OperatorTaskExecutionSummary,
    preview: &OperatorTaskExecutionPreview,
    capability: &SolverExecutionCapability,
) -> SolverExecutionCapabilityReport {
    let mut rejection_reasons = Vec::new();

    require_match(
        &mut rejection_reasons,
        "runtime_protocol",
        &summary.runtime_protocol,
        &capability.runtime_protocols,
    );
    require_match(
        &mut rejection_reasons,
        "program_kind",
        &summary.program_kind,
        &capability.program_kinds,
    );
    require_match(
        &mut rejection_reasons,
        "abi_kind",
        &summary.abi_kind,
        &capability.abi_kinds,
    );
    require_match(
        &mut rejection_reasons,
        "entrypoint_kind",
        &summary.entrypoint_kind,
        &capability.entrypoint_kinds,
    );
    require_optional_match(
        &mut rejection_reasons,
        "operator_id",
        &summary.operator_id,
        &capability.operator_ids,
    );
    require_optional_match(
        &mut rejection_reasons,
        "result_serialization",
        &preview.result_serialization,
        &capability.result_serializations,
    );
    require_optional_value_match(
        &mut rejection_reasons,
        "execution_mode",
        summary.execution_mode.as_deref(),
        &capability.execution_modes,
    );
    require_optional_value_match(
        &mut rejection_reasons,
        "authority_mode",
        summary.authority_mode.as_deref(),
        &capability.authority_modes,
    );

    if preview.package_fetch_required && !capability.supports_package_fetch {
        rejection_reasons.push(
            "package_fetch_required is true but capability does not support package fetch"
                .to_string(),
        );
    }
    if preview.offline_runnable && !capability.supports_offline_execution {
        rejection_reasons.push(
            "offline_runnable is true but capability does not support offline execution"
                .to_string(),
        );
    }

    SolverExecutionCapabilityReport {
        capability_id: capability.capability_id.clone(),
        accepted: rejection_reasons.is_empty(),
        task_id: summary.task_id.clone(),
        operator_id: summary.operator_id.clone(),
        operator_kind: summary.operator_kind.clone(),
        program_kind: summary.program_kind.clone(),
        runtime_protocol: summary.runtime_protocol.clone(),
        dispatch_route: preview.dispatch_route.clone(),
        rejection_reasons,
        warnings: preview.dispatch_warnings.clone(),
    }
}

fn require_match(
    rejection_reasons: &mut Vec<String>,
    field: &str,
    value: &str,
    supported: &[String],
) {
    if !supported.iter().any(|item| item == value) {
        rejection_reasons.push(format!("{field} `{value}` is not supported"));
    }
}

fn require_optional_match(
    rejection_reasons: &mut Vec<String>,
    field: &str,
    value: &str,
    supported: &[String],
) {
    if !supported.is_empty() {
        require_match(rejection_reasons, field, value, supported);
    }
}

fn require_optional_value_match(
    rejection_reasons: &mut Vec<String>,
    field: &str,
    value: Option<&str>,
    supported: &[String],
) {
    if let Some(value) = value {
        require_optional_match(rejection_reasons, field, value, supported);
    }
}
