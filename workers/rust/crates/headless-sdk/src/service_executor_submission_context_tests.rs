use crate::service_executor_ack_loss_tests::{observe, observe_sequence, run};
use crate::{HeadlessExecutor, HeadlessRunReport, ServiceHeadlessExecutor, all_direct_fem_routes};
use serde_json::{Value, json};

fn response(value: &Value) -> Vec<u8> {
    let body = value.to_string();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn receipt() -> Value {
    json!({"job":{"job_id":"owned-job","status":"queued",
        "project_id":"owned-project","model_version_id":"owned-version"}})
}

fn payload() -> Value {
    json!({"model":{},"project_id":"owned-project","model_version_id":"owned-version"})
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

#[test]
fn all_direct_solvers_reject_missing_transmitted_submission_association() {
    for route in all_direct_fem_routes() {
        for key in ["project_id", "model_version_id"] {
            let mut reply = receipt();
            reply["job"].as_object_mut().unwrap().remove(key);
            let ((report, calls), request) =
                observe(&response(&reply), |url| run(url, route.action, payload()));
            assert!(request.starts_with(&format!("POST {} HTTP/1.1", route.route)));
            assert_unknown(report, calls, route.action);
        }
    }
}

#[test]
fn corrupt_association_is_unknown_even_if_the_reply_claims_terminal_failure() {
    for key in ["project_id", "model_version_id"] {
        for status in ["queued", "completed", "failed", "cancelled"] {
            for value in [
                Value::Null,
                json!(7),
                json!(false),
                json!(""),
                json!("rejected-secret-other"),
                json!("owned-version "),
            ] {
                let mut reply = receipt();
                reply["job"]["status"] = json!(status);
                reply["job"][key] = value;
                let ((report, calls), _) =
                    observe(&response(&reply), |url| run(url, "solve_bar_1d", payload()));
                assert_unknown(report, calls, "solve_bar_1d");
            }
        }
    }
}

#[test]
fn repeated_association_fields_must_agree_even_without_request_constraints() {
    for key in ["project_id", "model_version_id"] {
        for value in [Value::Null, json!(7), json!("rejected-secret-other")] {
            let mut reply = receipt();
            reply[key] = value;
            let ((report, calls), _) = observe(&response(&reply), |url| {
                run(url, "solve_bar_1d", json!({"model":{}}))
            });
            assert_unknown(report, calls, "solve_bar_1d");
        }
    }
}

#[test]
fn composite_explicit_mesh_and_native_mesh_check_the_context_actually_sent() {
    for (action, path, mut request) in [
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
            "/api/v1/fem/heat-bar-1d/jobs",
            json!({"study_kind":"heat_bar_1d","input":{}}),
        ),
    ] {
        request["project_id"] = json!("owned-project");
        if action != "direct_mesh_solve" {
            request["model_version_id"] = json!("owned-version");
        }
        let mut reply = receipt();
        reply["job"]["project_id"] = json!("rejected-secret-other");
        let ((report, calls), sent) = observe(&response(&reply), |url| run(url, action, request));
        assert!(sent.starts_with(&format!("POST {path} HTTP/1.1")));
        let body: Value = serde_json::from_str(sent.split_once("\r\n\r\n").unwrap().1).unwrap();
        let context = if path == "/api/direct-mesh/solve" {
            &body["input"]
        } else {
            &body
        };
        assert_eq!(context["project_id"], "owned-project");
        if action != "direct_mesh_solve" {
            assert_eq!(context["model_version_id"], "owned-version");
        }
        assert_unknown(report, calls, action);
    }
}

#[test]
fn saved_reference_bad_submission_stops_before_polling_fetching_or_later_writes() {
    for (action, reference) in [
        ("direct_mesh_solve", "model"),
        ("solve_from_model_version", "version"),
        ("solve_and_wait_from_model_version", "version"),
    ] {
        for explicit in [false, true] {
            let mut request = json!({"timeout_ms":100,"interval_ms":10});
            let mut record = json!({"model_id":"owned-model","project_id":"owned-project",
                "kind":"heat_bar_1d","payload":{"nodes":[],"elements":[]}});
            let path = if reference == "version" {
                record["version_id"] = json!("owned-version");
                request["model_version_id"] = json!("owned-version");
                request["endpoints"] = json!(["127.0.0.1:7001"]);
                "/api/v1/model-versions/owned-version"
            } else {
                request["model_id"] = json!("owned-model");
                "/api/v1/models/owned-model"
            };
            if explicit {
                record["kind"] = json!("custom_study");
                request["study_kind"] = json!("heat_bar_1d");
                request["endpoints"] = json!(["127.0.0.1:7001"]);
            }
            let mut source = json!({});
            source[reference] = record;
            let mut reply = receipt();
            reply["job"]["project_id"] = json!("rejected-secret-other");
            let ((report, calls), sent) =
                observe_sequence(vec![response(&source), response(&reply)], |url| {
                    run(url, action, request)
                });
            assert_eq!(sent.len(), 2);
            assert!(sent[0].starts_with(&format!("GET {path} HTTP/1.1")));
            assert!(sent[1].starts_with(if explicit {
                "POST /api/direct-mesh/solve "
            } else {
                "POST /api/v1/fem/heat-bar-1d/jobs "
            }));
            assert_unknown(report, calls, action);
        }
    }
}

#[test]
fn matched_optional_context_and_inline_model_context_preserve_real_receipts() {
    for request in [
        json!({"model":{}}),
        json!({"model":{},"project_id":"owned-project"}),
        json!({"model":{},"model_version_id":"owned-version"}),
        payload(),
        json!({"model":{"project_id":"owned-project","model_version_id":"owned-version"}}),
    ] {
        let mut reply = receipt();
        reply["project_id"] = json!("owned-project");
        reply["model_version_id"] = json!("owned-version");
        let (outcome, _) = observe(&response(&reply), |url| {
            ServiceHeadlessExecutor::new(url).execute_step("solve_bar_1d", 1, &request)
        });
        assert_eq!(outcome.unwrap().result["raw"], reply);
    }
    let mut reply = receipt();
    reply["job"]
        .as_object_mut()
        .unwrap()
        .remove("model_version_id");
    let (outcome, _) = observe(&response(&reply), |url| {
        ServiceHeadlessExecutor::new(url).execute_step(
            "solve_bar_1d",
            1,
            &json!({"model":{},"project_id":"owned-project"}),
        )
    });
    outcome.unwrap();
    let mut reply = receipt();
    reply["job"]["model_version_id"] = Value::Null;
    let (outcome, _) = observe(&response(&reply), |url| {
        ServiceHeadlessExecutor::new(url).execute_step("solve_bar_1d", 1, &json!({"model":{}}))
    });
    outcome.unwrap();
}

#[test]
fn inline_context_is_not_discarded_or_unchecked() {
    let request =
        json!({"model":{"project_id":"owned-project","model_version_id":"owned-version"}});
    let mut reply = receipt();
    reply["job"]["model_version_id"] = json!("rejected-secret-other");
    let ((report, calls), _) = observe(&response(&reply), |url| run(url, "solve_bar_1d", request));
    assert_unknown(report, calls, "solve_bar_1d");
}

#[test]
fn matched_terminal_failure_is_known_and_never_replayed() {
    for status in ["failed", "cancelled"] {
        let mut reply = receipt();
        reply["job"]["status"] = json!(status);
        let ((report, calls), _) =
            observe(&response(&reply), |url| run(url, "solve_bar_1d", payload()));
        assert_eq!(calls, ["solve_bar_1d"]);
        let failure = report.execution_summary.failure.unwrap();
        assert_eq!(failure.stage, "execution");
        assert_eq!(
            failure.category,
            if status == "failed" {
                "runtime_failure"
            } else {
                "job_cancelled"
            }
        );
        assert!(!failure.retryable);
        assert_eq!(failure.retry_strategy, "none");
    }
}

#[test]
fn invalid_composite_or_inline_mesh_project_context_stops_before_io() {
    for action in ["solve_composite_thermo_electric_panel", "direct_mesh_solve"] {
        for value in [
            Value::Null,
            json!(7),
            json!(""),
            json!(" owned"),
            json!("../other"),
        ] {
            let mut request = json!({"study_kind":"heat_bar_1d","input":{"nodes":[],"elements":[]},
                "endpoints":["127.0.0.1:7001"]});
            request["project_id"] = value;
            let error = ServiceHeadlessExecutor::new("http://127.0.0.1:1")
                .execute_step(action, 1, &request)
                .unwrap_err();
            assert!(error.message.contains("project_id"));
            assert!(!error.message.contains("connect"));
            assert!(!error.message.contains("service_request_outcome_unknown"));
        }
    }
}
