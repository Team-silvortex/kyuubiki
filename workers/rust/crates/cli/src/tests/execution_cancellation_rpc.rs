use crate::{agent_execution_control, agent_lifecycle, rpc::handle_request, transport::AgentReply};
use kyuubiki_protocol::RpcRequest;
use serde_json::{Value, json};

fn target(request_id: &str, generation: u64, job_id: &str) -> Value {
    json!({"process_instance_id":agent_lifecycle::snapshot().process_instance_id,
        "request_id":request_id, "generation":generation, "job_id":job_id})
}

fn cancel(params: Value) -> kyuubiki_protocol::RpcResponse {
    let request: RpcRequest = serde_json::from_value(json!({"rpc_version":1,
        "id":"scoped-cancel-control", "method":"cancel_execution", "params":params}))
    .expect("generation-scoped cancellation must be a supported RPC");
    let AgentReply::Stream(frames, response) = handle_request(request, None);
    assert!(frames.is_empty());
    response
}

fn assert_ack(response: kyuubiki_protocol::RpcResponse, params: &Value, matched: bool) {
    assert!(response.ok, "{response:?}");
    assert_eq!(
        response.result.unwrap(),
        json!({
            "schema_version":"kyuubiki.agent-execution-cancellation/v1",
            "execution_target":params, "cancel_registered":matched,
            "status":if matched {"requested"} else {"target_not_observed"},
            "execution_terminal_confirmed":false, "pending_cancellation_created":false,
            "operator_package_cleanup_performed":false, "automatic_replay_authorized":false
        })
    );
}

#[test]
fn exact_execution_cancel_marks_only_the_selected_generation_and_keeps_the_job_reusable() {
    let job = "scoped-cancel-selected-job";
    let first =
        agent_execution_control::begin("scoped-cancel-selected".into(), 1, Some(job.into()))
            .unwrap();
    let sibling =
        agent_execution_control::begin("scoped-cancel-sibling".into(), 2, Some(job.into()))
            .unwrap();
    let params = target("scoped-cancel-selected", 1, job);
    assert_ack(cancel(params.clone()), &params, true);
    assert!(first.solver.cancellation_requested());
    assert!(!sibling.solver.cancellation_requested());
    assert_ack(cancel(params.clone()), &params, true);
    drop(first);
    assert_ack(cancel(params.clone()), &params, false);
    let next = agent_execution_control::begin("scoped-cancel-selected".into(), 3, Some(job.into()))
        .unwrap();
    assert!(!next.solver.cancellation_requested());
    assert!(!sibling.solver.cancellation_requested());
}

#[test]
fn stale_execution_cancel_never_marks_a_reused_request_or_creates_pending_job_cancel() {
    let job = "scoped-cancel-reused-job";
    let old = agent_execution_control::begin("scoped-cancel-reused".into(), 10, Some(job.into()))
        .unwrap();
    let params = target("scoped-cancel-reused", 10, job);
    drop(old);
    let next = agent_execution_control::begin("scoped-cancel-reused".into(), 11, Some(job.into()))
        .unwrap();
    assert_ack(cancel(params.clone()), &params, false);
    assert!(!next.solver.cancellation_requested());
    drop(next);
    assert_ack(cancel(params.clone()), &params, false);
    let later = agent_execution_control::begin("scoped-cancel-reused".into(), 12, Some(job.into()))
        .unwrap();
    assert!(!later.solver.cancellation_requested());
}

#[test]
fn scoped_cancel_requires_matching_process_request_generation_and_job_identity() {
    let job = "scoped-cancel-identity-job";
    let active =
        agent_execution_control::begin("scoped-cancel-identity".into(), 20, Some(job.into()))
            .unwrap();
    let exact = target("scoped-cancel-identity", 20, job);
    for (field, incorrect) in [
        ("process_instance_id", json!("previous-agent-process")),
        ("request_id", json!("unrelated-request")),
        ("generation", json!(21)),
        ("job_id", json!("unrelated-job")),
    ] {
        let mut params = exact.clone();
        params[field] = incorrect;
        assert_ack(cancel(params.clone()), &params, false);
        assert!(
            !active.solver.cancellation_requested(),
            "mismatched {field} widened cancellation"
        );
    }
    assert_ack(cancel(exact.clone()), &exact, true);
    assert!(active.solver.cancellation_requested());
}

#[test]
fn malformed_execution_cancel_fails_before_marking_any_control_or_pending_job() {
    let job = "scoped-cancel-malformed-job";
    let active =
        agent_execution_control::begin("scoped-cancel-malformed".into(), 30, Some(job.into()))
            .unwrap();
    let exact = target("scoped-cancel-malformed", 30, job);
    let mut malformed = vec![Value::Null, json!({"job_id":job})];
    for field in ["process_instance_id", "request_id", "generation", "job_id"] {
        let mut params = exact.clone();
        params.as_object_mut().unwrap().remove(field);
        malformed.push(params);
        let mut params = exact.clone();
        params[field] = Value::Null;
        malformed.push(params);
    }
    for invalid in [json!(0), json!(-1), json!(1.5), json!("30")] {
        let mut params = exact.clone();
        params["generation"] = invalid;
        malformed.push(params);
    }
    for field in ["process_instance_id", "request_id", "job_id"] {
        for invalid in [
            "",
            " ",
            "broken\nidentity",
            &"x".repeat(257),
            &"\u{754c}".repeat(100),
        ] {
            let mut params = exact.clone();
            params[field] = json!(invalid);
            malformed.push(params);
        }
    }
    let mut extra = exact.clone();
    extra["cancel_all"] = json!(true);
    malformed.push(extra);
    for params in malformed {
        let response = cancel(params.clone());
        assert!(!response.ok, "malformed target accepted: {params}");
        assert_eq!(response.error.unwrap().code, "invalid_params", "{params}");
        assert!(!active.solver.cancellation_requested());
    }
    assert!(!agent_execution_control::take_cancelled(job));
}
