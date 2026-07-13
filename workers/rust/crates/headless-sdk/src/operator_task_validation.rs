use kyuubiki_protocol::{
    OperatorTaskDigestError, SolverExecutionCapability, SolverExecutionCapabilityReport,
    check_operator_task_execution_capability, verify_operator_task_digest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessOperatorTaskValidationReport {
    pub schema_version: String,
    pub ok: bool,
    pub status: String,
    pub digest_ok: bool,
    pub capability: Option<SolverExecutionCapabilityReport>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn validate_operator_task_for_agent(
    task: &Value,
    capability: &SolverExecutionCapability,
) -> HeadlessOperatorTaskValidationReport {
    let mut errors = Vec::new();
    let digest_ok = match verify_operator_task_digest(task) {
        Ok(()) => true,
        Err(error) => {
            errors.push(digest_error_message(error));
            false
        }
    };

    let capability_report = match check_operator_task_execution_capability(task, capability) {
        Ok(report) => {
            errors.extend(report.rejection_reasons.iter().cloned());
            Some(report)
        }
        Err(error) => {
            errors.push(error.message);
            None
        }
    };

    let warnings = capability_report
        .as_ref()
        .map(|report| report.warnings.clone())
        .unwrap_or_default();
    let capability_ok = capability_report
        .as_ref()
        .map(|report| report.accepted)
        .unwrap_or(false);
    let ok = digest_ok && capability_ok && errors.is_empty();

    HeadlessOperatorTaskValidationReport {
        schema_version: "kyuubiki.headless-operator-task-validation/v1".to_string(),
        ok,
        status: if ok { "accepted" } else { "rejected" }.to_string(),
        digest_ok,
        capability: capability_report,
        errors,
        warnings,
    }
}

pub fn validate_operator_task_for_builtin_agent(
    task: &Value,
) -> HeadlessOperatorTaskValidationReport {
    validate_operator_task_for_agent(task, &SolverExecutionCapability::agent_builtin())
}

fn digest_error_message(error: OperatorTaskDigestError) -> String {
    match error {
        OperatorTaskDigestError::Missing => "operator task digest is missing".to_string(),
        OperatorTaskDigestError::InvalidTask(message) => {
            format!("operator task is invalid: {message}")
        }
        OperatorTaskDigestError::Mismatch { expected, actual } => {
            format!("operator task digest mismatch: expected {expected}, actual {actual}")
        }
    }
}
