use serde_json::Value;

use crate::operator_task_runtime::{
    OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED, OperatorPackageRuntimeBinding,
};
use kyuubiki_protocol::{OperatorTaskExecutionPreview, OperatorTaskExecutionSummary};

const OPERATOR_TASK_VALIDATION_RECEIPT_SCHEMA: &str = "kyuubiki.agent-operator-task-validation/v1";
const OPERATOR_TASK_PROVENANCE_RECEIPT_SCHEMA: &str = "kyuubiki.agent-operator-task-provenance/v1";

pub(crate) fn operator_task_validation_receipt(
    summary: &OperatorTaskExecutionSummary,
    preview: &OperatorTaskExecutionPreview,
    binding: &OperatorPackageRuntimeBinding,
) -> Value {
    let blocked_reason = if preview.package_fetch_required && !binding.is_attached() {
        Value::String(OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED.to_string())
    } else {
        Value::Null
    };

    serde_json::json!({
        "schema_version": OPERATOR_TASK_VALIDATION_RECEIPT_SCHEMA,
        "validation_owner": "agent_runtime",
        "validation_status": if blocked_reason.is_null() { "accepted" } else { "blocked" },
        "digest_verified": true,
        "execution_program_verified": true,
        "runtime_protocol": summary.runtime_protocol,
        "abi_kind": summary.abi_kind,
        "dispatch_route": preview.dispatch_route,
        "package_fetch_required": preview.package_fetch_required,
        "operator_package_runtime_attached": binding.is_attached(),
        "blocked_reason": blocked_reason
    })
}

pub(crate) fn operator_task_provenance_receipt(
    summary: &OperatorTaskExecutionSummary,
    preview: &OperatorTaskExecutionPreview,
    mode: &str,
    binding: &OperatorPackageRuntimeBinding,
) -> Value {
    serde_json::json!({
        "schema_version": OPERATOR_TASK_PROVENANCE_RECEIPT_SCHEMA,
        "provenance_owner": "agent_runtime",
        "retention_scope": summary.cache_scope,
        "requested_mode": mode,
        "task_digest": summary.task_digest,
        "task_id": summary.task_id,
        "operator_id": summary.operator_id,
        "program_id": summary.program_id,
        "package_ref": summary.package_ref,
        "package_version": summary.package_version,
        "runtime_protocol": summary.runtime_protocol,
        "abi_kind": summary.abi_kind,
        "dispatch_route": preview.dispatch_route,
        "agent_fetchable": summary.agent_fetchable,
        "offline_runnable": preview.offline_runnable,
        "operator_package_runtime_attached": binding.is_attached(),
        "lineage": {
            "digest_verified": true,
            "execution_program_verified": true,
            "preview_digest": preview.task_digest
        }
    })
}
