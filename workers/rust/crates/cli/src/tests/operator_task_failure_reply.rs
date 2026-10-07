use super::{
    bind_failure_report, execute_solver, operator_task_failure_reply, publish_operator_task_result,
    run_solver,
};
use crate::agent_execution_control::register_cancel;
use crate::agent_watchdog::FailureReport;
use crate::operator_task_runtime::OperatorTaskRuntimeError;
use crate::{agent_lifecycle, operator_task_receipts, transport::AgentReply};
use kyuubiki_protocol::{RPC_VERSION, RpcMethod, RpcRequest};
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

#[test]
fn heartbeat_stop_cancellation_is_retained_as_failure_not_computed_success() {
    let id = "publication-heartbeat-stop";
    let job = "publication-heartbeat-stop-job";
    let mut task = task();
    task["integrity"]["task_digest"] = json!("a".repeat(64));
    task["execution_program"] = json!({"program_id":"solve.bar_1d"});
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: id.into(),
        method: RpcMethod::RunOperatorTaskIr,
        params: json!({"mode":"execute", "dispatch_attempt_id":"c".repeat(32),
            "task_ir":task}),
    };
    let guard = agent_lifecycle::begin_execution(
        id.into(),
        Some(job.into()),
        "run_operator_task_ir".into(),
    )
    .unwrap();
    let generation = guard.generation();
    let retained = crate::agent_task_results::begin(&request, generation);
    assert!(retained.is_some());
    let cancelling = guard.clone();
    let heartbeat = crate::transport::HeartbeatHandle::on_stop_for_test(move || {
        cancelling.request_cancellation();
    });
    assert!(!guard.cancellation_requested());
    let AgentReply::Stream(frames, response) = publish_operator_task_result(
        id.into(),
        guard,
        Some(heartbeat),
        None,
        json!({"operator_task_ir_status":"executed", "result":{"value":42}}),
        Some(&task),
    );
    crate::agent_task_results::finish(retained, &response);
    assert!(frames.is_empty());
    assert!(
        !response.ok,
        "shutdown cancellation leaked success: {response:?}"
    );
    assert!(response.result.is_none());

    let AgentReply::Stream(_, fetched) = crate::agent_task_results::handle_fetch(RpcRequest {
        rpc_version: RPC_VERSION,
        id: "read-heartbeat-stop".into(),
        method: RpcMethod::FetchOperatorTaskResult,
        params: json!({"attempt_id":request.params["dispatch_attempt_id"], "request_id":id,
            "task_id":task["task_id"], "task_digest":task["integrity"]["task_digest"],
            "operator_id":task["operator"]["id"],
            "program_id":task["execution_program"]["program_id"]}),
    });
    let retained = fetched.result.unwrap();
    assert_eq!(retained["generation"], generation);
    assert_eq!(retained["status"], "receipt_retained");
    assert_eq!(retained["automatic_replay_authorized"], false);
    assert_eq!(
        retained["response"],
        serde_json::to_value(&response).unwrap()
    );
    let failure = &retained["response"]["error"]["details"]["operator_task_failure_receipt"];
    assert_eq!(failure["reason_code"], "cancelled");
    assert_eq!(failure["failure_stage"], "publish_result");
    assert_eq!(failure["task_digest"], task["integrity"]["task_digest"]);
    assert_eq!(failure["recovery"]["retryable"], false);

    let rerun = agent_lifecycle::begin_execution(
        "publication-heartbeat-rerun".into(),
        Some(job.into()),
        "run_operator_task_ir".into(),
    )
    .unwrap();
    let AgentReply::Stream(_, response) = publish_operator_task_result(
        "publication-heartbeat-rerun".into(),
        rerun,
        None,
        None,
        json!({"operator_task_ir_status":"executed"}),
        Some(&task),
    );
    assert!(
        response.ok,
        "shutdown cancellation poisoned an explicit rerun"
    );
}

#[test]
fn watchdog_failure_during_heartbeat_stop_fences_publication_before_cancel_propagates() {
    let id = "publication-watchdog-stop";
    let task = task();
    let guard = agent_lifecycle::begin_execution(
        id.into(),
        Some("publication-watchdog-stop-job".into()),
        "run_operator_task_ir".into(),
    )
    .unwrap();
    let generation = guard.generation();
    let control = guard.solver_control();
    let expiring = guard.clone();
    let heartbeat = crate::transport::HeartbeatHandle::on_stop_for_test(move || {
        agent_lifecycle::fail_execution(
            expiring,
            "watchdog_timeout",
            "expired while stopping heartbeat",
        );
    });
    let AgentReply::Stream(frames, response) = publish_operator_task_result(
        id.into(),
        guard,
        Some(heartbeat),
        None,
        json!({"operator_task_ir_status":"executed"}),
        Some(&task),
    );
    assert!(
        !control.cancellation_requested(),
        "test must cover the pre-propagation window"
    );
    assert!(frames.is_empty());
    assert!(
        !response.ok,
        "terminal watchdog generation leaked success: {response:?}"
    );
    assert!(response.result.is_none());
    let error = response.error.unwrap();
    assert_eq!(error.code, "watchdog_timeout");
    assert_eq!(error.message, "expired while stopping heartbeat");
    let details = error.details.unwrap();
    assert_eq!(details["generation"], generation);
    let receipt = &details["operator_task_failure_receipt"];
    assert_eq!(receipt["failure_stage"], "publish_result");
    assert_eq!(receipt["reason_code"], "watchdog_timeout");
    assert_eq!(receipt["task_id"], task["task_id"]);
    assert_eq!(receipt["recovery"]["retryable"], false);
}

#[test]
fn legacy_solver_cancellation_during_serialization_discards_result_and_progress() {
    const JOB: &str = "legacy-publication-serialize-job";
    struct CancellingResult;
    impl serde::Serialize for CancellingResult {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            register_cancel(JOB.into()).unwrap();
            json!({"value":42}).serialize(serializer)
        }
    }
    let AgentReply::Stream(frames, response) = run_solver::<Value, _, _, _>(
        RpcRequest {
            rpc_version: RPC_VERSION,
            id: "legacy-publication-serialize".into(),
            method: RpcMethod::SolveBar1d,
            params: json!({"job_id":JOB}),
        },
        None,
        "bar_1d",
        "bar result",
        |_| 2,
        |_| Ok(CancellingResult),
    );
    assert!(
        !response.ok,
        "serialization cancellation leaked success: {response:?}"
    );
    assert!(response.result.is_none());
    assert!(
        frames.is_empty(),
        "cancelled execution must not publish success progress"
    );
    let error = response.error.unwrap();
    assert_eq!(error.code, "cancelled");
    assert_eq!(
        error.message,
        "execution cancelled before result publication"
    );
    assert_eq!(error.details.unwrap()["method"], "solve_bar_1d");
}

fn legacy_request(id: &str, params: Value) -> RpcRequest {
    RpcRequest {
        rpc_version: RPC_VERSION,
        id: id.into(),
        method: RpcMethod::SolveBar1d,
        params,
    }
}

fn recorded_legacy_failure(
    id: &str,
    job: &str,
) -> (agent_lifecycle::ExecutionGuard, FailureReport) {
    let guard =
        agent_lifecycle::begin_execution(id.into(), Some(job.into()), "solve_bar_1d".into())
            .unwrap();
    let recorded = agent_lifecycle::fail_execution(
        guard.clone(),
        "watchdog_timeout",
        "original watchdog progress budget expired",
    );
    assert!(
        !guard.cancellation_requested(),
        "cover failure before propagation"
    );
    (guard, recorded)
}

fn assert_original_failure(reply: AgentReply, recorded: &FailureReport) {
    let AgentReply::Stream(frames, response) = reply;
    assert!(
        frames.is_empty(),
        "failed execution leaked success progress"
    );
    assert!(!response.ok);
    assert!(response.result.is_none());
    assert_eq!(response.id, recorded.request_id);
    let error = response.error.unwrap();
    assert_eq!(error.code, recorded.reason_code);
    assert_eq!(error.message, recorded.message);
    assert_eq!(
        error.details.unwrap(),
        serde_json::to_value(recorded).unwrap()
    );
}

fn assert_legacy_explicit_rerun(id: &str, job: &str, previous_generation: u64) {
    let guard =
        agent_lifecycle::begin_execution(id.into(), Some(job.into()), "solve_bar_1d".into())
            .expect("failed execution must release its lifecycle lease");
    assert!(guard.generation() > previous_generation);
    assert!(guard.result_publication_allowed());
    let AgentReply::Stream(frames, response) = execute_solver::<Value, _, _, _>(
        legacy_request(id, json!({"job_id": job})),
        None,
        guard,
        "bar_1d",
        "bar result",
        |_| 2,
        |_| Ok(json!({"value": 42})),
    );
    assert!(
        response.ok,
        "explicit rerun inherited prior failure: {response:?}"
    );
    assert_eq!(response.result, Some(json!({"value": 42})));
    assert!(!frames.is_empty());
}

#[test]
fn legacy_decode_error_preserves_original_watchdog_failure_and_allows_explicit_rerun() {
    let id = "legacy-decode-watchdog-rebind";
    let job = "legacy-decode-watchdog-rebind-job";
    let (guard, recorded) = recorded_legacy_failure(id, job);
    let reply = execute_solver::<String, Value, _, _>(
        legacy_request(id, json!({"job_id": job})),
        None,
        guard,
        "bar_1d",
        "bar result",
        |_| panic!("decode failure must not publish progress"),
        |_| panic!("decode failure must not enter the solver"),
    );
    assert_original_failure(reply, &recorded);
    assert_legacy_explicit_rerun(id, job, recorded.generation);
}

struct UnencodableResult;

impl serde::Serialize for UnencodableResult {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("fixture cannot encode result"))
    }
}

#[test]
fn legacy_encoding_error_preserves_original_watchdog_failure_and_allows_explicit_rerun() {
    let id = "legacy-encoding-watchdog-rebind";
    let job = "legacy-encoding-watchdog-rebind-job";
    let (guard, recorded) = recorded_legacy_failure(id, job);
    let solver_called = std::cell::Cell::new(false);
    let reply = execute_solver::<Value, _, _, _>(
        legacy_request(id, json!({"job_id": job})),
        None,
        guard,
        "bar_1d",
        "bar result",
        |_| panic!("encoding failure must not publish progress"),
        |_| {
            solver_called.set(true);
            Ok(UnencodableResult)
        },
    );
    assert!(solver_called.get(), "fixture must reach result encoding");
    assert_original_failure(reply, &recorded);
    assert_legacy_explicit_rerun(id, job, recorded.generation);
}

#[test]
fn legacy_decode_error_without_prior_failure_keeps_invalid_params_classification() {
    let AgentReply::Stream(frames, response) = run_solver::<String, Value, _, _>(
        legacy_request(
            "legacy-decode-primary-failure",
            json!({"job_id":"legacy-decode-primary-job"}),
        ),
        None,
        "bar_1d",
        "bar result",
        |_| panic!("decode failure must not publish progress"),
        |_| panic!("decode failure must not enter the solver"),
    );
    assert!(frames.is_empty());
    assert!(!response.ok);
    assert!(response.result.is_none());
    let error = response.error.unwrap();
    assert_eq!(error.code, "invalid_params");
    assert!(
        error
            .message
            .starts_with("failed to decode inline solver parameters:")
    );
    let details = error.details.unwrap();
    assert_eq!(details["reason_code"], error.code);
    assert_eq!(details["message"], error.message);
    assert_eq!(details["method"], "solve_bar_1d");
}

#[test]
fn legacy_encoding_error_without_prior_failure_keeps_transport_classification() {
    let AgentReply::Stream(frames, response) = run_solver::<Value, _, _, _>(
        legacy_request(
            "legacy-encoding-primary-failure",
            json!({"job_id":"legacy-encoding-primary-job"}),
        ),
        None,
        "bar_1d",
        "bar result",
        |_| panic!("encoding failure must not publish progress"),
        |_| Ok(UnencodableResult),
    );
    assert!(frames.is_empty());
    assert!(!response.ok);
    assert!(response.result.is_none());
    let error = response.error.unwrap();
    assert_eq!(error.code, "result_transport_failed");
    assert_eq!(
        error.message,
        "failed to serialize bar result: fixture cannot encode result"
    );
    let details = error.details.unwrap();
    assert_eq!(details["reason_code"], error.code);
    assert_eq!(details["message"], error.message);
    assert_eq!(details["method"], "solve_bar_1d");
}
