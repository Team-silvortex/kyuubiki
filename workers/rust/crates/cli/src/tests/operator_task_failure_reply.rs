use super::{bind_failure_report, operator_task_failure_reply, publish_operator_task_result};
use crate::agent_execution_control::register_cancel;
use crate::agent_watchdog::FailureReport;
use crate::operator_task_runtime::OperatorTaskRuntimeError;
use crate::{agent_lifecycle, operator_task_receipts, transport::AgentReply};
use serde_json::{Value, json};

fn task() -> Value {
    json!({"task_id":"publication-task", "operator":{"id":"solve.bar_1d"},
        "integrity":{"task_digest":"sha256:publication-fixture"}})
}

fn report(code: &str, message: &str) -> FailureReport {
    FailureReport {
        request_id: "failure-rebind".into(),
        generation: 1,
        job_id: Some("failure-rebind-job".into()),
        method: "run_operator_task_ir".into(),
        reason_code: code.into(),
        message: message.into(),
        elapsed_ms: 10_000,
        occurred_unix_ms: 10_000,
    }
}

#[test]
fn publication_boundary_discards_cancelled_result_and_allows_explicit_rerun() {
    let job = "operator-publication-cancel-job";
    let task = task();
    let guard = agent_lifecycle::begin_execution(
        "publication-cancel".into(),
        Some(job.into()),
        "run_operator_task_ir".into(),
    )
    .unwrap();
    let result = json!({"operator_task_ir_status":"executed", "result":{"value":42}});
    register_cancel(job.into()).unwrap();
    let AgentReply::Stream(frames, response) = publish_operator_task_result(
        "publication-cancel".into(),
        guard,
        None,
        None,
        result.clone(),
        Some(&task),
    );
    assert!(frames.is_empty());
    assert!(!response.ok);
    assert!(response.result.is_none());
    let error = response.error.unwrap();
    assert_eq!(error.code, "cancelled");
    let details = error.details.unwrap();
    let failure = &details["operator_task_failure_receipt"];
    assert_eq!(failure["failure_stage"], "publish_result");
    assert_eq!(failure["task_id"], task["task_id"]);
    assert_eq!(failure["task_digest"], task["integrity"]["task_digest"]);
    assert_eq!(failure["operator_id"], task["operator"]["id"]);
    assert_eq!(failure["reason_code"], error.code);
    assert_eq!(failure["message"], error.message);
    assert_eq!(failure["recovery"]["retryable"], false);
    assert_eq!(failure["recovery"]["safe_to_continue_other_tasks"], true);
    assert!(details["solver_checkpoint"].is_null());

    let guard = agent_lifecycle::begin_execution(
        "publication-rerun".into(),
        Some(job.into()),
        "run_operator_task_ir".into(),
    )
    .expect("cancelled publication must release the request lease");
    let AgentReply::Stream(frames, response) = publish_operator_task_result(
        "publication-rerun".into(),
        guard,
        None,
        None,
        result.clone(),
        Some(&task),
    );
    assert!(frames.is_empty());
    assert!(
        response.ok,
        "explicit rerun inherited cancellation: {response:?}"
    );
    assert_eq!(response.result, Some(result));
}

#[test]
fn watchdog_terminal_reason_rebinds_task_and_recovery_without_stale_retry_advice() {
    let task = task();
    let stale = operator_task_receipts::operator_task_failure_receipt(
        "operator_package_fetch_failed",
        "temporary fetch failure",
        "fetch_package",
        None,
    );
    assert_eq!(stale["recovery"]["retryable"], true);
    let report = report(
        "watchdog_timeout",
        "execution exceeded watchdog progress budget",
    );
    let receipt = bind_failure_report(&report, stale, Some(&task));
    assert_eq!(receipt["reason_code"], report.reason_code);
    assert_eq!(receipt["message"], report.message);
    assert_eq!(receipt["failure_stage"], "fetch_package");
    assert_eq!(receipt["task_id"], task["task_id"]);
    assert_eq!(receipt["operator_id"], task["operator"]["id"]);
    assert_eq!(receipt["task_digest"], task["integrity"]["task_digest"]);
    assert_eq!(receipt["recovery"]["retryable"], false);
    assert_eq!(
        receipt["recovery"]["required_action"],
        "inspect_watchdog_timeout_before_explicit_rerun"
    );
}

#[test]
fn terminal_failure_keeps_admission_diagnostics_while_rebinding_core_fields() {
    let task = task();
    let report = report(
        "operator_task_admission_rejected",
        "routing policy rejected",
    );
    let mut receipt = operator_task_receipts::operator_task_failure_receipt(
        &report.reason_code,
        &report.message,
        "validate_admission_policy",
        None,
    );
    let admission = json!({"accepted":false, "violations":[{"code":"routing_policy"}]});
    receipt["admission_report"] = admission.clone();
    let receipt = bind_failure_report(&report, receipt, Some(&task));
    assert_eq!(receipt["admission_report"], admission);
    assert_eq!(receipt["task_id"], task["task_id"]);
    assert_eq!(
        receipt["recovery"]["required_action"],
        "fix_task_ir_authority_and_routing_policy"
    );
}

#[test]
fn late_cancel_reply_reuses_the_recorded_watchdog_failure_for_the_same_generation() {
    let task = task();
    let guard = agent_lifecycle::begin_execution(
        "watchdog-task-cancel".into(),
        Some("watchdog-task-cancel-job".into()),
        "run_operator_task_ir".into(),
    )
    .unwrap();
    let recorded = agent_lifecycle::fail_execution(
        guard.clone(),
        "watchdog_timeout",
        "recorded progress budget exceeded",
    );
    let AgentReply::Stream(frames, response) = operator_task_failure_reply(
        "watchdog-task-cancel".into(),
        guard,
        None,
        OperatorTaskRuntimeError::with_task(
            "cancelled",
            "solver observed cancellation later",
            "execute_solver",
            Some(&task),
        ),
        Some(&task),
    );
    assert!(frames.is_empty());
    assert!(!response.ok);
    assert!(response.result.is_none());
    let error = response.error.unwrap();
    assert_eq!(error.code, recorded.reason_code);
    assert_eq!(error.message, recorded.message);
    let details = error.details.unwrap();
    assert_eq!(details["generation"], recorded.generation);
    assert_eq!(details["job_id"], "watchdog-task-cancel-job");
    let failure = &details["operator_task_failure_receipt"];
    assert_eq!(failure["reason_code"], error.code);
    assert_eq!(failure["message"], error.message);
    assert_eq!(failure["task_id"], task["task_id"]);
    assert_eq!(failure["failure_stage"], "execute_solver");
    assert_eq!(failure["recovery"]["retryable"], false);
    assert_eq!(
        failure["recovery"]["required_action"],
        "inspect_watchdog_timeout_before_explicit_rerun"
    );
}
