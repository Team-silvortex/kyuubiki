use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    OperatorTaskExecutionSummary, OperatorTaskSummaryError,
    summarize_operator_task_execution_checked,
};

pub const OPERATOR_TASK_ADMISSION_SCHEMA: &str = "kyuubiki.operator-task-admission/v1";

const CENTRAL_AUTHORITIES: &[&str] = &["central_operator_library", "single_orchestrator"];
const LOCAL_AUTHORITIES: &[&str] = &["agent_local", "offline_mesh", "self_directed"];
const ORCHESTRA_FETCH: &str = "orchestra_fetch";
const LOCAL_EXECUTION_MODES: &[&str] = &["agent_native", "local_builtin", "local_bundle"];
const CACHE_SCOPES: &[&str] = &["job", "session", "agent", "none"];
const MAX_ROUTING_VALUES: usize = 64;
const MAX_ROUTING_VALUE_LEN: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorTaskAdmissionViolation {
    pub code: String,
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorTaskAdmissionReport {
    pub schema_version: String,
    pub accepted: bool,
    pub task_id: String,
    pub operator_id: String,
    pub authority_mode: Option<String>,
    pub execution_mode: Option<String>,
    pub cache_scope: Option<String>,
    pub agent_fetchable: Option<bool>,
    pub package_ref: Option<String>,
    pub violations: Vec<OperatorTaskAdmissionViolation>,
}

pub fn check_operator_task_admission(
    task: &Value,
) -> Result<OperatorTaskAdmissionReport, OperatorTaskSummaryError> {
    let summary = summarize_operator_task_execution_checked(task)?;
    Ok(build_operator_task_admission_report(task, &summary))
}

pub fn build_operator_task_admission_report(
    task: &Value,
    summary: &OperatorTaskExecutionSummary,
) -> OperatorTaskAdmissionReport {
    let mut violations = Vec::new();
    validate_operator_identity(summary, &mut violations);
    validate_authority_and_execution(summary, &mut violations);
    validate_cache_scope(summary, &mut violations);
    validate_package_authority(summary, &mut violations);
    validate_string_list(
        task.pointer("/runtime_hints/required_capabilities"),
        "runtime_hints.required_capabilities",
        &mut violations,
    );
    validate_string_list(
        task.pointer("/runtime_hints/placement_tags"),
        "runtime_hints.placement_tags",
        &mut violations,
    );

    OperatorTaskAdmissionReport {
        schema_version: OPERATOR_TASK_ADMISSION_SCHEMA.to_string(),
        accepted: violations.is_empty(),
        task_id: summary.task_id.clone(),
        operator_id: summary.operator_id.clone(),
        authority_mode: summary.authority_mode.clone(),
        execution_mode: summary.execution_mode.clone(),
        cache_scope: summary.cache_scope.clone(),
        agent_fetchable: summary.agent_fetchable,
        package_ref: summary.package_ref.clone(),
        violations,
    }
}

fn validate_operator_identity(
    summary: &OperatorTaskExecutionSummary,
    violations: &mut Vec<OperatorTaskAdmissionViolation>,
) {
    let id = summary.operator_id.as_str();
    let valid = !id.is_empty()
        && id.len() <= 128
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if !valid {
        violation(
            violations,
            "operator_id_unsafe_for_resolution",
            "operator.id",
            "operator id must use 1-128 ASCII alphanumeric, dot, underscore, or hyphen bytes",
        );
    }
}

fn validate_authority_and_execution(
    summary: &OperatorTaskExecutionSummary,
    violations: &mut Vec<OperatorTaskAdmissionViolation>,
) {
    let authority = summary.authority_mode.as_deref();
    let execution = summary.execution_mode.as_deref();
    let central_authority = is_central_authority(authority);
    let authority_known =
        central_authority || authority.is_some_and(|value| LOCAL_AUTHORITIES.contains(&value));
    let execution_known = matches!(execution, Some(ORCHESTRA_FETCH))
        || execution.is_some_and(|value| LOCAL_EXECUTION_MODES.contains(&value));

    require_known_value(
        authority,
        authority_known,
        "runtime_hints.authority_mode",
        "authority_mode_missing",
        "authority_mode_unsupported",
        violations,
    );
    require_known_value(
        execution,
        execution_known,
        "runtime_hints.execution_mode",
        "execution_mode_missing",
        "execution_mode_unsupported",
        violations,
    );

    match summary.agent_fetchable {
        None => violation(
            violations,
            "agent_fetchable_missing",
            "runtime_hints.agent_fetchable",
            "agent_fetchable must be declared explicitly",
        ),
        Some(true) if !central_authority => violation(
            violations,
            "agent_fetch_requires_central_authority",
            "runtime_hints.agent_fetchable",
            "agent package fetch requires central package authority",
        ),
        Some(false) if central_authority => violation(
            violations,
            "central_authority_requires_agent_fetch",
            "runtime_hints.agent_fetchable",
            "central authority tasks must remain agent fetchable",
        ),
        _ => {}
    }

    if central_authority && execution != Some(ORCHESTRA_FETCH) {
        violation(
            violations,
            "central_authority_requires_orchestra_fetch",
            "runtime_hints.execution_mode",
            "central authority tasks must use orchestra_fetch",
        );
    }
    if execution == Some(ORCHESTRA_FETCH) && !central_authority {
        violation(
            violations,
            "orchestra_fetch_requires_central_authority",
            "runtime_hints.authority_mode",
            "orchestra_fetch requires central_operator_library authority",
        );
    }
}

fn validate_cache_scope(
    summary: &OperatorTaskExecutionSummary,
    violations: &mut Vec<OperatorTaskAdmissionViolation>,
) {
    match summary.cache_scope.as_deref() {
        None => violation(
            violations,
            "cache_scope_missing",
            "runtime_hints.cache_scope",
            "cache_scope must be declared explicitly",
        ),
        Some(value) if !CACHE_SCOPES.contains(&value) => violation(
            violations,
            "cache_scope_unsupported",
            "runtime_hints.cache_scope",
            format!("cache_scope `{value}` is not supported"),
        ),
        _ => {}
    }
}

fn validate_package_authority(
    summary: &OperatorTaskExecutionSummary,
    violations: &mut Vec<OperatorTaskAdmissionViolation>,
) {
    let authority = summary.authority_mode.as_deref();
    let execution = summary.execution_mode.as_deref();
    let package_ref = summary.package_ref.as_deref();

    if is_central_authority(authority) {
        let expected = format!("orchestra://operator-package/{}", summary.operator_id);
        match package_ref {
            None => violation(
                violations,
                "central_package_ref_missing",
                "execution_program.package_ref",
                "central operator tasks must declare an orchestra package reference",
            ),
            Some(actual) if actual != expected => violation(
                violations,
                "central_package_ref_mismatch",
                "execution_program.package_ref",
                format!("central package reference must be `{expected}`, got `{actual}`"),
            ),
            _ => {}
        }
    } else if package_ref.is_some_and(|value| value.starts_with("orchestra://")) {
        violation(
            violations,
            "local_authority_forbids_orchestra_package",
            "execution_program.package_ref",
            "local and offline authority cannot resolve orchestra package references",
        );
    }

    if execution == Some("local_bundle") && !package_ref.is_some_and(is_safe_bundle_ref) {
        violation(
            violations,
            "local_bundle_package_ref_invalid",
            "execution_program.package_ref",
            "local_bundle execution requires a bundle:// package reference",
        );
    }
}

fn is_safe_bundle_ref(value: &str) -> bool {
    let Some(path) = value.strip_prefix("bundle://") else {
        return false;
    };
    !path.is_empty()
        && !path.contains('\\')
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
}

fn is_central_authority(authority: Option<&str>) -> bool {
    authority.is_some_and(|value| CENTRAL_AUTHORITIES.contains(&value))
}

fn validate_string_list(
    value: Option<&Value>,
    field: &str,
    violations: &mut Vec<OperatorTaskAdmissionViolation>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(values) = value.as_array() else {
        violation(
            violations,
            "routing_values_not_array",
            field,
            format!("{field} must be an array of strings"),
        );
        return;
    };
    if values.len() > MAX_ROUTING_VALUES {
        violation(
            violations,
            "routing_values_over_budget",
            field,
            format!("{field} exceeds {MAX_ROUTING_VALUES} entries"),
        );
    }

    let mut seen = HashSet::new();
    for value in values {
        let Some(value) = value.as_str() else {
            violation(
                violations,
                "routing_value_not_string",
                field,
                format!("{field} entries must be strings"),
            );
            continue;
        };
        if value.is_empty() || value.len() > MAX_ROUTING_VALUE_LEN {
            violation(
                violations,
                "routing_value_invalid_length",
                field,
                format!("{field} entries must contain 1-{MAX_ROUTING_VALUE_LEN} bytes"),
            );
        }
        if !seen.insert(value) {
            violation(
                violations,
                "routing_value_duplicate",
                field,
                format!("{field} contains duplicate value `{value}`"),
            );
        }
    }
}

fn require_known_value(
    value: Option<&str>,
    known: bool,
    field: &str,
    missing_code: &str,
    unsupported_code: &str,
    violations: &mut Vec<OperatorTaskAdmissionViolation>,
) {
    match value {
        None => violation(
            violations,
            missing_code,
            field,
            format!("{field} must be declared explicitly"),
        ),
        Some(value) if !known => violation(
            violations,
            unsupported_code,
            field,
            format!("{field} value `{value}` is not supported"),
        ),
        _ => {}
    }
}

fn violation(
    violations: &mut Vec<OperatorTaskAdmissionViolation>,
    code: &str,
    field: &str,
    message: impl Into<String>,
) {
    violations.push(OperatorTaskAdmissionViolation {
        code: code.to_string(),
        field: field.to_string(),
        message: message.into(),
    });
}
