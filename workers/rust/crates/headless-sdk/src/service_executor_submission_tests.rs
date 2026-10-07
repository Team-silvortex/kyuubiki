use crate::service_executor_ack_loss_tests::{observe, observe_sequence, run};
use crate::{HeadlessExecutor, HeadlessRunReport, ServiceHeadlessExecutor, all_direct_fem_routes};
use serde_json::{Value, json};

fn response(receipt: &Value) -> Vec<u8> {
    let body = serde_json::to_string(receipt).unwrap();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn assert_unknown(report: HeadlessRunReport, calls: Vec<String>, action: &str) {
    assert_eq!(calls, [action]);
    assert_eq!(report.status, "failed");
    assert_eq!(report.executed_step_count, 0);
    assert_eq!(report.steps.len(), 1);
    let preview = &report.steps[0].result_preview;
    assert!(preview.get("job_id").is_none());
    assert!(preview.get("job").is_none());
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "service_request_outcome_unknown");
    assert_eq!(failure.stage, "transport");
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
    assert!(failure.message.contains("job_receipt_invalid:"));
    assert!(!failure.message.contains("rejected-secret"));
}

fn rejects(receipt: Value) {
    let ((report, calls), request) = observe(&response(&receipt), |url| {
        run(url, "solve_bar_1d", json!({"model":{"tip_force":1000}}))
    });
    assert!(request.starts_with("POST /api/v1/fem/axial-bar/jobs HTTP/1.1"));
    assert_unknown(report, calls, "solve_bar_1d");
}

#[test]
fn submission_requires_an_explicit_job_object_and_usable_unmodified_identity() {
    for receipt in [
        Value::Null,
        json!(true),
        json!(1),
        json!("queued"),
        json!([]),
        json!({}),
        json!({"job":null}),
        json!({"job":[]}),
        json!({"job_id":"owned","status":"queued"}),
        json!({"job":{"status":"queued"}}),
    ] {
        rejects(receipt);
    }
    for id in [
        Value::Null,
        json!(7),
        json!(false),
        json!(""),
        json!(" "),
        json!(" owned"),
        json!("owned "),
        json!(".owned"),
        json!("a..b"),
        json!("rejected-secret/escape"),
        json!("owned%2fescape"),
        json!("owned\n"),
        json!("owned\\escape"),
        json!("\u{4efb}\u{52a1}"),
    ] {
        rejects(json!({"job":{"job_id":id,"status":"queued"}}));
    }
}

#[test]
fn submission_requires_public_status_and_consistent_repeated_fields() {
    rejects(json!({"job":{"job_id":"owned"}}));
    for status in [
        Value::Null,
        json!(true),
        json!(1),
        json!(""),
        json!("running"),
        json!("QUEUED"),
        json!("queued "),
        json!("rejected-secret-status"),
    ] {
        rejects(json!({"job":{"job_id":"owned","status":status}}));
    }
    for (key, value) in [
        ("job_id", Value::Null),
        ("job_id", json!(7)),
        ("job_id", json!("rejected-secret-other")),
        ("status", Value::Null),
        ("status", json!("completed")),
        ("status", json!(false)),
    ] {
        let mut receipt = json!({"job":{"job_id":"owned","status":"queued"}});
        receipt[key] = value;
        rejects(receipt);
    }
}

#[test]
fn every_registered_direct_solver_rejects_empty_successful_acknowledgements() {
    for route in all_direct_fem_routes() {
        let ((report, calls), request) = observe(&response(&json!({})), |url| {
            run(url, route.action, json!({"model":{}}))
        });
        assert!(request.starts_with(&format!("POST {} HTTP/1.1", route.route)));
        assert_unknown(report, calls, route.action);
    }
}

#[test]
fn workflow_composite_and_mesh_submissions_share_the_semantic_ack_gate() {
    for (action, path, payload) in [
        (
            "workflow_submit_catalog",
            "/api/v1/workflows/catalog/owned/jobs",
            json!({"workflow_id":"owned"}),
        ),
        (
            "workflow_submit_graph",
            "/api/v1/workflows/graph/jobs",
            json!({"graph":{}}),
        ),
        (
            "solve_composite_thermo_electric_panel",
            "/api/v1/fem/composite-thermo-electric-panel/jobs",
            json!({"electrostatic_model":{},"electric_conduction_model":{},
                "heat_model":{},"thermal_model":{},"electrothermal_loss":{},
                "electrothermal_feedback":{},"electric_conduction_feedback":{},
                "thermal_expansion_feedback":{}}),
        ),
        (
            "direct_mesh_solve",
            "/api/direct-mesh/solve",
            json!({"study_kind":"heat_bar_1d","input":{"nodes":[],"elements":[]},"endpoints":["127.0.0.1:7001"]}),
        ),
        (
            "direct_mesh_solve",
            "/api/v1/fem/axial-bar/jobs",
            json!({"study_kind":"axial_bar_1d","input":{}}),
        ),
    ] {
        let ((report, calls), request) =
            observe(&response(&json!({})), |url| run(url, action, payload));
        assert!(request.starts_with(&format!("POST {path} HTTP/1.1")));
        assert_unknown(report, calls, action);
    }
}

#[test]
fn model_version_submission_failure_stops_before_polling_or_result_reads() {
    for action in [
        "solve_from_model_version",
        "solve_and_wait_from_model_version",
    ] {
        for (kind, path) in [
            ("axial_bar_1d", "/api/v1/fem/axial-bar/jobs"),
            ("custom_study", "/api/direct-mesh/solve"),
        ] {
            let version = json!({"version":{"version_id":"owned-version","model_id":"owned-model",
                "project_id":"owned-project","kind":kind,"payload":{"nodes":[],"elements":[]}}});
            let ((report, calls), requests) = observe_sequence(
                vec![
                    response(&version),
                    response(&json!({"job":{"status":"queued"}})),
                ],
                |url| {
                    run(
                        url,
                        action,
                        json!({"model_version_id":"owned-version","study_kind":"heat_bar_1d",
                    "endpoints":["127.0.0.1:7001"],"timeout_ms":100,"interval_ms":10}),
                    )
                },
            );
            assert_eq!(requests.len(), 2);
            assert!(requests[0].starts_with("GET /api/v1/model-versions/owned-version HTTP/1.1"));
            assert!(requests[1].starts_with(&format!("POST {path} HTTP/1.1")));
            assert_unknown(report, calls, action);
        }
    }
}

#[test]
fn valid_submission_states_supply_bindings_without_claiming_a_numerical_result() {
    for status in [
        "queued",
        "preprocessing",
        "partitioning",
        "solving",
        "postprocessing",
        "completed",
    ] {
        let receipt = json!({"job_id":"owned-1.v2","status":status,
            "job":{"job_id":"owned-1.v2","status":status}});
        let (outcome, _) = observe(&response(&receipt), |url| {
            ServiceHeadlessExecutor::new(url).execute_step("solve_bar_1d", 1, &json!({"model":{}}))
        });
        let outcome = outcome.unwrap();
        assert_eq!(outcome.status, "executed");
        assert_eq!(outcome.result["job_id"], "owned-1.v2");
        assert_eq!(outcome.result["status"], status);
        assert_eq!(outcome.result["raw"], receipt);
        assert!(outcome.result.get("result").is_none());
    }
}

#[test]
fn explicit_failed_or_cancelled_submission_is_known_failure_not_unknown_write() {
    for status in ["failed", "cancelled"] {
        for detail in [
            "explicit terminal outcome",
            "failed to connect to a peer",
            "connection refused",
            "agent_queue_timeout waiting for agent capacity",
            "job_wait timed out",
            "cancelled",
            "unauthorized HTTP status 403",
            "service_request_outcome_unknown:",
        ] {
            for action in ["solve_bar_1d", "job_wait"] {
                let receipt =
                    json!({"job":{"job_id":"owned-job","status":status,"message":detail}});
                let ((report, calls), _) = observe(&response(&receipt), |url| {
                    run(
                        url,
                        action,
                        json!({"model":{},"job_id":"owned-job","timeout_ms":100,"interval_ms":10}),
                    )
                });
                assert_eq!(calls, [action]);
                assert_eq!(report.status, "failed");
                assert_eq!(report.executed_step_count, 0);
                let failure = report.execution_summary.failure.unwrap();
                assert_eq!(
                    failure.category,
                    if status == "failed" {
                        "runtime_failure"
                    } else {
                        "job_cancelled"
                    }
                );
                assert_eq!(failure.stage, "execution");
                assert!(
                    failure
                        .message
                        .contains(&format!("terminal status {status}"))
                );
                assert!(!failure.retryable);
                assert_eq!(failure.retry_strategy, "none");
            }
        }
    }
}
