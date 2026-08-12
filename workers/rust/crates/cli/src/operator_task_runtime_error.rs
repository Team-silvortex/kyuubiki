use kyuubiki_protocol::{
    OperatorTaskAdmissionReport, OperatorTaskDigestError, OperatorTaskSummaryError,
    OperatorTaskSummaryErrorCode,
};
use serde_json::Value;

use crate::operator_task_receipts::operator_task_failure_receipt;
use crate::operator_task_runtime::OperatorTaskRuntimeError;

pub(crate) fn classify_digest_error(
    error: OperatorTaskDigestError,
    task_ir: &Value,
) -> OperatorTaskRuntimeError {
    match error {
        OperatorTaskDigestError::Missing => runtime_error(
            "operator_task_digest_missing",
            "missing operator task digest",
            "verify_digest",
            task_ir,
        ),
        OperatorTaskDigestError::Mismatch { expected, actual } => runtime_error(
            "operator_task_digest_mismatch",
            format!("operator task digest mismatch: expected {expected}, actual {actual}"),
            "verify_digest",
            task_ir,
        ),
        OperatorTaskDigestError::InvalidTask(message) => runtime_error(
            "operator_task_digest_invalid",
            message,
            "verify_digest",
            task_ir,
        ),
    }
}

pub(crate) fn classify_summary_error(
    error: OperatorTaskSummaryError,
    task_ir: &Value,
) -> OperatorTaskRuntimeError {
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
    runtime_error(code, error.message, "summarize_execution_program", task_ir)
}

pub(crate) fn classify_admission_rejection(
    report: OperatorTaskAdmissionReport,
    task_ir: &Value,
) -> OperatorTaskRuntimeError {
    let reason_codes = report
        .violations
        .iter()
        .map(|violation| violation.code.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut error = runtime_error(
        "operator_task_admission_rejected",
        format!("operator task admission rejected: {reason_codes}"),
        "validate_admission_policy",
        task_ir,
    );
    error.details["admission_report"] =
        serde_json::to_value(report).expect("operator task admission report should serialize");
    error
}

fn runtime_error(
    code: &'static str,
    message: impl Into<String>,
    stage: &'static str,
    task_ir: &Value,
) -> OperatorTaskRuntimeError {
    let message = message.into();
    OperatorTaskRuntimeError {
        code,
        details: operator_task_failure_receipt(code, &message, stage, Some(task_ir)),
        message,
    }
}
