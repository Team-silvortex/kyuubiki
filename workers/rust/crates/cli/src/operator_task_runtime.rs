use std::sync::{Mutex, OnceLock};

use serde_json::Value;

use crate::config::AgentConfig;
use crate::operator_task_builtin::{
    is_agent_native_builtin_operator, run_agent_native_builtin_task,
};
use crate::operator_task_receipts::{
    operator_task_provenance_receipt, operator_task_validation_receipt,
};
use kyuubiki_protocol::{
    OperatorTaskDigestError, OperatorTaskExecutionPreview, OperatorTaskExecutionSummary,
    OperatorTaskSummaryError, OperatorTaskSummaryErrorCode, preview_operator_task_execution,
    summarize_operator_task_execution_checked, verify_operator_task_digest,
};

pub(crate) const OPERATOR_TASK_STATUS_VERIFIED_PENDING: &str = "verified_pending_engine_execution";
pub(crate) const OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED: &str =
    "operator_package_runtime_not_yet_attached";
const OPERATOR_PACKAGE_RUNTIME_ATTACHED_PENDING_FETCH: &str =
    "operator_package_runtime_attached_pending_package_fetch";
const OPERATOR_PACKAGE_RUNTIME_READY_FOR_FETCH: &str = "operator_package_runtime_ready_for_fetch";
const OPERATOR_TASK_STATUS_EXECUTED: &str = "executed";
const OPERATOR_TASK_AGENT_NATIVE_STATUS: &str = "agent_native_builtin_executed";
pub(crate) const OPERATOR_TASK_BLOCKED_STAGE: &str = "fetch_package";
pub(crate) const OPERATOR_TASK_MODE_PREFLIGHT: &str = "preflight";
pub(crate) const OPERATOR_TASK_MODE_EXECUTE: &str = "execute";
const OPERATOR_PACKAGE_RUNTIME_HOST: &str = "kyuubiki-engine.operator-sdk-host/v1";
const OPERATOR_PACKAGE_RUNTIME_SDK: &str = "kyuubiki-operator-sdk";
const OPERATOR_PACKAGE_RUNTIME_STATUS_DETACHED: &str = "not_attached";
const OPERATOR_PACKAGE_RUNTIME_STATUS_ATTACHED: &str = "attached";
const OPERATOR_PACKAGE_FETCH_REQUEST_SCHEMA: &str = "kyuubiki.operator-package-fetch-request/v1";
const OPERATOR_TASK_READINESS_BLOCKED: &str = "blocked";
const OPERATOR_TASK_READINESS_READY: &str = "ready_for_package_resolution";
const OPERATOR_TASK_READINESS_EXECUTED: &str = "executed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperatorPackageRuntimeAttachment {
    pub host_id: String,
    pub packages_root: String,
    pub activated_package_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OperatorPackageRuntimeBinding {
    Detached,
    // Reserved for the next host-wiring step; production currently defaults to Detached.
    #[allow(dead_code)]
    Attached(OperatorPackageRuntimeAttachment),
}

pub(crate) fn operator_package_runtime_binding_from_config(
    config: &AgentConfig,
) -> OperatorPackageRuntimeBinding {
    let Some(packages_root) = config.operator_packages_root.clone() else {
        return OperatorPackageRuntimeBinding::Detached;
    };

    OperatorPackageRuntimeBinding::Attached(OperatorPackageRuntimeAttachment {
        host_id: config
            .operator_package_host_id
            .clone()
            .or_else(|| config.agent_id.clone())
            .unwrap_or_else(|| "agent-local/operator-host".to_string()),
        packages_root,
        activated_package_count: config.operator_activated_package_count,
    })
}

pub(crate) fn store_operator_package_runtime_binding(binding: OperatorPackageRuntimeBinding) {
    if let Ok(mut current) = runtime_binding().lock() {
        *current = binding;
    }
}

pub(crate) fn operator_package_runtime_snapshot() -> Value {
    let binding = current_runtime_binding();
    operator_package_runtime_snapshot_from_binding(&binding)
}

pub(crate) fn operator_package_runtime_snapshot_for_config(config: &AgentConfig) -> Value {
    let binding = operator_package_runtime_binding_from_config(config);
    operator_package_runtime_snapshot_from_binding(&binding)
}

fn operator_package_runtime_snapshot_from_binding(
    binding: &OperatorPackageRuntimeBinding,
) -> Value {
    serde_json::json!({
        "ready": binding.is_attached(),
        "status": binding.status(),
        "expected_host": OPERATOR_PACKAGE_RUNTIME_HOST,
        "expected_sdk": OPERATOR_PACKAGE_RUNTIME_SDK,
        "attachment": operator_package_runtime_attachment(&binding)
    })
}

impl OperatorPackageRuntimeBinding {
    fn status(&self) -> &'static str {
        match self {
            Self::Detached => OPERATOR_PACKAGE_RUNTIME_STATUS_DETACHED,
            Self::Attached(_) => OPERATOR_PACKAGE_RUNTIME_STATUS_ATTACHED,
        }
    }

    pub(crate) fn is_attached(&self) -> bool {
        match self {
            Self::Detached => false,
            Self::Attached(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperatorTaskRuntimeError {
    pub code: &'static str,
    pub message: String,
}

impl OperatorTaskRuntimeError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub(crate) fn run_operator_task_ir(params: &Value) -> Result<Value, OperatorTaskRuntimeError> {
    run_operator_task_ir_with_runtime(params, current_runtime_binding())
}

pub(crate) fn run_operator_task_ir_with_runtime(
    params: &Value,
    package_runtime: OperatorPackageRuntimeBinding,
) -> Result<Value, OperatorTaskRuntimeError> {
    let mode = parse_mode(params)?;
    let task_ir = params
        .get("task_ir")
        .ok_or_else(|| OperatorTaskRuntimeError::new("invalid_params", "missing task_ir"))?;

    verify_operator_task_digest(task_ir).map_err(classify_digest_error)?;

    let summary =
        summarize_operator_task_execution_checked(task_ir).map_err(classify_operator_task_error)?;
    let preview = preview_operator_task_execution(task_ir).map_err(classify_operator_task_error)?;

    if mode == OPERATOR_TASK_MODE_EXECUTE && is_agent_native_builtin_operator(&summary.operator_id)
    {
        let result =
            run_agent_native_builtin_task(&summary.operator_id, task_ir).map_err(|error| {
                OperatorTaskRuntimeError::new("operator_task_execution_failed", error)
            })?;
        return Ok(build_agent_native_execution_payload(
            summary,
            preview,
            package_runtime,
            result,
        ));
    }

    Ok(build_preflight_payload(
        summary,
        preview,
        mode,
        package_runtime,
    ))
}

fn classify_digest_error(error: OperatorTaskDigestError) -> OperatorTaskRuntimeError {
    match error {
        OperatorTaskDigestError::Missing => OperatorTaskRuntimeError::new(
            "operator_task_digest_missing",
            "missing operator task digest",
        ),
        OperatorTaskDigestError::Mismatch { expected, actual } => OperatorTaskRuntimeError::new(
            "operator_task_digest_mismatch",
            format!("operator task digest mismatch: expected {expected}, actual {actual}"),
        ),
        OperatorTaskDigestError::InvalidTask(message) => {
            OperatorTaskRuntimeError::new("operator_task_digest_invalid", message)
        }
    }
}

fn classify_operator_task_error(error: OperatorTaskSummaryError) -> OperatorTaskRuntimeError {
    let code = match error.code {
        OperatorTaskSummaryErrorCode::MirrorMismatch => "operator_task_mirror_mismatch",
        OperatorTaskSummaryErrorCode::ExecutionAbiMismatch => {
            "operator_task_execution_abi_mismatch"
        }
        OperatorTaskSummaryErrorCode::ProgramMismatch => "operator_task_program_mismatch",
        OperatorTaskSummaryErrorCode::EntrypointMismatch => "operator_task_entrypoint_mismatch",
        OperatorTaskSummaryErrorCode::MissingField | OperatorTaskSummaryErrorCode::Invalid => {
            "operator_task_invalid"
        }
    };
    OperatorTaskRuntimeError::new(code, error.message)
}

fn runtime_binding() -> &'static Mutex<OperatorPackageRuntimeBinding> {
    static BINDING: OnceLock<Mutex<OperatorPackageRuntimeBinding>> = OnceLock::new();
    BINDING.get_or_init(|| Mutex::new(OperatorPackageRuntimeBinding::Detached))
}

fn current_runtime_binding() -> OperatorPackageRuntimeBinding {
    runtime_binding()
        .lock()
        .map(|binding| binding.clone())
        .unwrap_or(OperatorPackageRuntimeBinding::Detached)
}

fn parse_mode(params: &Value) -> Result<&'static str, OperatorTaskRuntimeError> {
    match params.get("mode").and_then(Value::as_str) {
        None | Some(OPERATOR_TASK_MODE_PREFLIGHT) => Ok(OPERATOR_TASK_MODE_PREFLIGHT),
        Some(OPERATOR_TASK_MODE_EXECUTE) => Ok(OPERATOR_TASK_MODE_EXECUTE),
        Some(other) => Err(OperatorTaskRuntimeError::new(
            "invalid_params",
            format!("unsupported operator task mode: {other}"),
        )),
    }
}

fn build_preflight_payload(
    summary: OperatorTaskExecutionSummary,
    preview: OperatorTaskExecutionPreview,
    mode: &str,
    package_runtime: OperatorPackageRuntimeBinding,
) -> Value {
    serde_json::json!({
        "requested_mode": mode,
        "task_execution_preview": operator_task_execution_preview_payload(&preview),
        "operator_task_ir_status": OPERATOR_TASK_STATUS_VERIFIED_PENDING,
        "execution_runtime_status": execution_runtime_status(&package_runtime),
        "operator_package_runtime": operator_package_runtime_contract(&summary, &package_runtime),
        "validation_receipt": operator_task_validation_receipt(&summary, &preview, &package_runtime),
        "provenance_receipt": operator_task_provenance_receipt(&summary, &preview, mode, &package_runtime),
        "package_fetch_request": package_fetch_request(&summary, &package_runtime),
        "operator_package_runtime_ready": package_runtime.is_attached(),
        "blocked_stage": blocked_stage(&package_runtime),
        "next_stage": OPERATOR_TASK_BLOCKED_STAGE,
        "execution_readiness": execution_readiness(mode, &package_runtime),
        "execution_plan": execution_plan(mode, &package_runtime),
        "task_digest": summary.task_digest,
        "task_id": summary.task_id,
        "operator_id": summary.operator_id,
        "operator_kind": summary.operator_kind,
        "program_id": summary.program_id,
        "program_kind": summary.program_kind,
        "runtime_protocol": summary.runtime_protocol,
        "abi_kind": summary.abi_kind,
        "entrypoint_kind": summary.entrypoint_kind,
        "entrypoint_name": summary.entrypoint_name,
        "package_ref": summary.package_ref,
        "package_version": summary.package_version,
        "authority_mode": summary.authority_mode,
        "execution_mode": summary.execution_mode,
        "cache_scope": summary.cache_scope,
        "agent_fetchable": summary.agent_fetchable
    })
}

fn build_agent_native_execution_payload(
    summary: OperatorTaskExecutionSummary,
    preview: OperatorTaskExecutionPreview,
    package_runtime: OperatorPackageRuntimeBinding,
    result: Value,
) -> Value {
    serde_json::json!({
        "requested_mode": OPERATOR_TASK_MODE_EXECUTE,
        "task_execution_preview": operator_task_execution_preview_payload(&preview),
        "operator_task_ir_status": OPERATOR_TASK_STATUS_EXECUTED,
        "execution_runtime_status": OPERATOR_TASK_AGENT_NATIVE_STATUS,
        "operator_package_runtime": operator_package_runtime_contract(&summary, &package_runtime),
        "validation_receipt": operator_task_validation_receipt(&summary, &preview, &package_runtime),
        "provenance_receipt": operator_task_provenance_receipt(
            &summary,
            &preview,
            OPERATOR_TASK_MODE_EXECUTE,
            &package_runtime
        ),
        "package_fetch_request": Value::Null,
        "operator_package_runtime_ready": package_runtime.is_attached(),
        "blocked_stage": Value::Null,
        "next_stage": "serialize_result",
        "execution_readiness": agent_native_execution_readiness(),
        "execution_plan": agent_native_execution_plan(),
        "task_digest": summary.task_digest,
        "task_id": summary.task_id,
        "operator_id": summary.operator_id,
        "operator_kind": summary.operator_kind,
        "program_id": summary.program_id,
        "program_kind": summary.program_kind,
        "runtime_protocol": summary.runtime_protocol,
        "abi_kind": summary.abi_kind,
        "entrypoint_kind": summary.entrypoint_kind,
        "entrypoint_name": summary.entrypoint_name,
        "package_ref": summary.package_ref,
        "package_version": summary.package_version,
        "authority_mode": summary.authority_mode,
        "execution_mode": summary.execution_mode,
        "cache_scope": summary.cache_scope,
        "agent_fetchable": summary.agent_fetchable,
        "result": result
    })
}

fn operator_task_execution_preview_payload(preview: &OperatorTaskExecutionPreview) -> Value {
    serde_json::json!({
        "task_digest": preview.task_digest,
        "task_id": preview.task_id,
        "operator_id": preview.operator_id,
        "operator_kind": preview.operator_kind,
        "dispatch_route": preview.dispatch_route,
        "package_ref": preview.package_ref,
        "package_version": preview.package_version,
        "package_fetch_required": preview.package_fetch_required,
        "package_readiness_gate": preview.package_readiness_gate,
        "result_serialization": preview.result_serialization,
        "authority_mode": preview.authority_mode,
        "execution_mode": preview.execution_mode,
        "cache_scope": preview.cache_scope,
        "agent_fetchable": preview.agent_fetchable,
        "offline_runnable": preview.offline_runnable,
        "dispatch_warnings": preview.dispatch_warnings
    })
}

fn execution_runtime_status(binding: &OperatorPackageRuntimeBinding) -> &'static str {
    match binding {
        OperatorPackageRuntimeBinding::Detached => OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED,
        OperatorPackageRuntimeBinding::Attached(_) => {
            OPERATOR_PACKAGE_RUNTIME_ATTACHED_PENDING_FETCH
        }
    }
}

fn blocked_stage(binding: &OperatorPackageRuntimeBinding) -> Value {
    match binding {
        OperatorPackageRuntimeBinding::Detached => {
            Value::String(OPERATOR_TASK_BLOCKED_STAGE.into())
        }
        OperatorPackageRuntimeBinding::Attached(_) => Value::Null,
    }
}

fn operator_package_runtime_contract(
    summary: &OperatorTaskExecutionSummary,
    binding: &OperatorPackageRuntimeBinding,
) -> Value {
    let mut contract = serde_json::json!({
        "status": binding.status(),
        "expected_host": OPERATOR_PACKAGE_RUNTIME_HOST,
        "expected_sdk": OPERATOR_PACKAGE_RUNTIME_SDK,
        "package_ref": summary.package_ref,
        "package_version": summary.package_version,
        "fetch_policy": {
            "authority_mode": summary.authority_mode,
            "execution_mode": summary.execution_mode,
            "cache_scope": summary.cache_scope,
            "agent_fetchable": summary.agent_fetchable
        },
        "required_interfaces": [
            "resolve_package",
            "fetch_package",
            "verify_package_integrity",
            "activate_operator_registry",
            "dispatch_entrypoint",
            "serialize_result"
        ],
        "trust_policy": {
            "allowed_runtimes": ["rust_crate"],
            "allow_absolute_entrypoints": false,
            "require_entrypoint_within_package_root": true
        }
    });

    if let OperatorPackageRuntimeBinding::Attached(_) = binding {
        let attachment = operator_package_runtime_attachment(binding);
        contract["host_id"] = attachment["host_id"].clone();
        contract["packages_root"] = attachment["packages_root"].clone();
        contract["activated_package_count"] = attachment["activated_package_count"].clone();
    }

    contract
}

fn package_fetch_request(
    summary: &OperatorTaskExecutionSummary,
    binding: &OperatorPackageRuntimeBinding,
) -> Value {
    let mut request = serde_json::json!({
        "schema_version": OPERATOR_PACKAGE_FETCH_REQUEST_SCHEMA,
        "request_status": package_fetch_request_status(binding),
        "package_ref": summary.package_ref,
        "package_version": summary.package_version,
        "task_digest": summary.task_digest,
        "operator_id": summary.operator_id,
        "program_id": summary.program_id,
        "runtime_protocol": summary.runtime_protocol,
        "abi_kind": summary.abi_kind,
        "agent_fetchable": summary.agent_fetchable,
        "fetch_policy": {
            "authority_mode": summary.authority_mode,
            "execution_mode": summary.execution_mode,
            "cache_scope": summary.cache_scope
        },
        "target": {
            "runtime_attached": binding.is_attached(),
            "host_id": null,
            "packages_root": null
        }
    });

    if let OperatorPackageRuntimeBinding::Attached(_) = binding {
        let attachment = operator_package_runtime_attachment(binding);
        request["target"]["host_id"] = attachment["host_id"].clone();
        request["target"]["packages_root"] = attachment["packages_root"].clone();
    }

    request
}

fn package_fetch_request_status(binding: &OperatorPackageRuntimeBinding) -> &'static str {
    match binding {
        OperatorPackageRuntimeBinding::Detached => "blocked_runtime_not_attached",
        OperatorPackageRuntimeBinding::Attached(_) => "ready_to_resolve",
    }
}

fn execution_readiness(mode: &str, binding: &OperatorPackageRuntimeBinding) -> Value {
    match binding {
        OperatorPackageRuntimeBinding::Detached => serde_json::json!({
            "status": OPERATOR_TASK_READINESS_BLOCKED,
            "requested_mode": mode,
            "ready_to_dispatch": false,
            "current_stage": OPERATOR_TASK_BLOCKED_STAGE,
            "blocking_stage": OPERATOR_TASK_BLOCKED_STAGE,
            "blocking_reason": OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED,
            "blocking_owner": "operator_package_runtime",
            "required_action": "attach_operator_package_runtime"
        }),
        OperatorPackageRuntimeBinding::Attached(_) => serde_json::json!({
            "status": OPERATOR_TASK_READINESS_READY,
            "requested_mode": mode,
            "ready_to_dispatch": false,
            "current_stage": OPERATOR_TASK_BLOCKED_STAGE,
            "blocking_stage": Value::Null,
            "blocking_reason": Value::Null,
            "blocking_owner": Value::Null,
            "required_action": "resolve_fetch_verify_and_activate_package"
        }),
    }
}

fn agent_native_execution_readiness() -> Value {
    serde_json::json!({
        "status": OPERATOR_TASK_READINESS_EXECUTED,
        "requested_mode": OPERATOR_TASK_MODE_EXECUTE,
        "ready_to_dispatch": true,
        "current_stage": "serialize_result",
        "blocking_stage": Value::Null,
        "blocking_reason": Value::Null,
        "blocking_owner": Value::Null,
        "required_action": Value::Null
    })
}

fn operator_package_runtime_attachment(binding: &OperatorPackageRuntimeBinding) -> Value {
    match binding {
        OperatorPackageRuntimeBinding::Detached => Value::Null,
        OperatorPackageRuntimeBinding::Attached(attachment) => serde_json::json!({
            "host_id": attachment.host_id,
            "packages_root": attachment.packages_root,
            "activated_package_count": attachment.activated_package_count
        }),
    }
}

fn agent_native_execution_plan() -> Value {
    serde_json::json!([
        {
            "stage": "verify_digest",
            "status": "complete",
            "owner": "agent_runtime",
            "gate": "passed"
        },
        {
            "stage": "summarize_execution_program",
            "status": "complete",
            "owner": "agent_runtime",
            "gate": "passed"
        },
        {
            "stage": OPERATOR_TASK_BLOCKED_STAGE,
            "status": "skipped",
            "owner": "agent_runtime",
            "gate": "not_required",
            "reason": "agent_native_builtin"
        },
        {
            "stage": "verify_package_integrity",
            "status": "skipped",
            "owner": "agent_runtime",
            "gate": "not_required",
            "reason": "agent_native_builtin"
        },
        {
            "stage": "dispatch_entrypoint",
            "status": "complete",
            "owner": "agent_runtime",
            "gate": "passed",
            "reason": "agent_native_builtin"
        },
        {
            "stage": "serialize_result",
            "status": "complete",
            "owner": "agent_runtime",
            "gate": "passed"
        }
    ])
}

fn execution_plan(mode: &str, binding: &OperatorPackageRuntimeBinding) -> Value {
    let fetch_stage = match binding {
        OperatorPackageRuntimeBinding::Detached => serde_json::json!({
            "stage": OPERATOR_TASK_BLOCKED_STAGE,
            "status": "blocked",
            "owner": "operator_package_runtime",
            "gate": "blocked",
            "requested_mode": mode,
            "reason": OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED
        }),
        OperatorPackageRuntimeBinding::Attached(_) => serde_json::json!({
            "stage": OPERATOR_TASK_BLOCKED_STAGE,
            "status": "pending",
            "owner": "operator_package_runtime",
            "gate": "open",
            "requested_mode": mode,
            "reason": OPERATOR_PACKAGE_RUNTIME_READY_FOR_FETCH
        }),
    };

    serde_json::json!([
        {
            "stage": "verify_digest",
            "status": "complete",
            "owner": "agent_runtime",
            "gate": "passed"
        },
        {
            "stage": "summarize_execution_program",
            "status": "complete",
            "owner": "agent_runtime",
            "gate": "passed"
        },
        fetch_stage,
        {
            "stage": "verify_package_integrity",
            "status": "pending",
            "owner": "operator_package_runtime",
            "gate": "waiting_for_fetch"
        },
        {
            "stage": "dispatch_entrypoint",
            "status": "pending",
            "owner": "operator_package_runtime",
            "gate": "waiting_for_integrity"
        },
        {
            "stage": "serialize_result",
            "status": "pending",
            "owner": "operator_package_runtime",
            "gate": "waiting_for_dispatch"
        }
    ])
}
