use crate::service_executor_ack_loss_tests::{observe, observe_sequence, run};
use crate::service_executor_job_receipt::JOB_STATUSES;
use crate::{HeadlessExecutor, HeadlessRunReport, ServiceHeadlessExecutor};
use serde_json::{Value, json};

fn response(value: &Value) -> Vec<u8> {
    let body = value.to_string();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn job() -> Value {
    json!({"job":{"job_id":"owned-job","status":"completed","progress":1.0,
        "project_id":"owned-project","model_version_id":"owned-version","simulation_case_id":"owned-case"},
        "result":{"retained":"inspection only"}})
}

fn assert_failure(report: HeadlessRunReport, calls: Vec<String>, context: bool) {
    assert_eq!(calls, ["job_fetch"]);
    assert_eq!(report.status, "failed");
    assert_eq!(report.executed_step_count, 0);
    assert_eq!(report.steps.len(), 1);
    for key in [
        "job_id",
        "job",
        "result",
        "raw",
        "project_id",
        "model_version_id",
    ] {
        assert!(report.steps[0].result_preview.get(key).is_none());
    }
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(
        failure.category,
        if context {
            "contract_failure"
        } else {
            "job_receipt_invalid"
        }
    );
    assert_eq!(
        failure.stage,
        if context {
            "validation"
        } else {
            "job_observation"
        }
    );
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
    assert!(failure.message.starts_with(if context {
        "job_read_invalid:"
    } else {
        "job_receipt_invalid:"
    }));
    assert!(!failure.message.contains("rejected-secret"));
    assert!(!failure.message.contains("service_request_outcome_unknown"));
}

fn rejects(reply: Value, payload: Value, context: bool) {
    let ((report, calls), request) =
        observe(&response(&reply), |url| run(url, "job_fetch", payload));
    assert!(request.starts_with("GET /api/v1/jobs/owned-job HTTP/1.1"));
    assert_failure(report, calls, context);
}

#[test]
fn observation_requires_an_explicit_job_and_matching_unmodified_identity() {
    for reply in [
        Value::Null,
        json!(false),
        json!(7),
        json!("completed"),
        json!([]),
        json!({}),
        json!({"job_id":"owned-job","status":"completed"}),
        json!({"job":null}),
        json!({"job":[]}),
        json!({"job":{"status":"completed"}}),
    ] {
        rejects(reply, json!({"job_id":"owned-job"}), false);
    }
    for id in [
        Value::Null,
        json!(7),
        json!(false),
        json!(""),
        json!("owned-job "),
        json!(" owned-job"),
        json!("rejected-secret-other"),
    ] {
        let mut reply = job();
        reply["job"]["job_id"] = id.clone();
        rejects(reply, json!({"job_id":"owned-job"}), false);
        let mut reply = job();
        reply["job_id"] = id;
        rejects(reply, json!({"job_id":"owned-job"}), false);
    }
}

#[test]
fn observation_requires_a_public_status_and_consistent_repeated_status() {
    let mut reply = job();
    reply["job"].as_object_mut().unwrap().remove("status");
    rejects(reply, json!({"job_id":"owned-job"}), false);
    for status in [
        Value::Null,
        json!(7),
        json!(false),
        json!(""),
        json!("running"),
        json!("COMPLETED"),
        json!("completed "),
        json!("rejected-secret-status"),
    ] {
        let mut reply = job();
        reply["job"]["status"] = status.clone();
        rejects(reply, json!({"job_id":"owned-job"}), false);
        let mut reply = job();
        reply["status"] = status;
        rejects(reply, json!({"job_id":"owned-job"}), false);
    }
    let mut reply = job();
    reply["status"] = json!("failed");
    rejects(reply, json!({"job_id":"owned-job"}), false);
}

#[test]
fn observation_enforces_optional_associations_and_repeated_context() {
    for (key, alias) in [
        ("project_id", "projectId"),
        ("model_version_id", "modelVersionId"),
        ("simulation_case_id", "simulationCaseId"),
    ] {
        for name in [key, alias] {
            for value in [
                None,
                Some(Value::Null),
                Some(json!(7)),
                Some(json!(false)),
                Some(json!("")),
                Some(json!("rejected-secret-other")),
            ] {
                let mut reply = job();
                match value {
                    Some(value) => {
                        reply["job"][key] = value;
                    }
                    None => {
                        reply["job"].as_object_mut().unwrap().remove(key);
                    }
                }
                let mut request = json!({"job_id":"owned-job"});
                request[name] = job()["job"][key].clone();
                rejects(reply, request, true);
            }
        }
        for value in [Value::Null, json!(7), json!("rejected-secret-other")] {
            let mut reply = job();
            reply[key] = value;
            rejects(reply, json!({"job_id":"owned-job"}), true);
        }
    }
}

#[test]
fn observation_accepts_every_public_state_without_claiming_computation_success() {
    for &status in JOB_STATUSES {
        let mut reply = job();
        reply["job"]["status"] = json!(status);
        reply["job"]["message"] =
            json!("failed to connect; cancelled; service_request_outcome_unknown:");
        reply["status"] = json!(status);
        reply["job_id"] = json!("owned-job");
        let (outcome, _) = observe(&response(&reply), |url| {
            ServiceHeadlessExecutor::new(url).execute_step(
                "job_fetch",
                1,
                &json!({"job_id":"owned-job"}),
            )
        });
        let outcome = outcome.unwrap();
        assert_eq!(
            outcome.status, "executed",
            "the read executed, not the solver"
        );
        assert_eq!(outcome.result["status"], status);
        assert_eq!(outcome.result["job"], reply["job"]);
        assert_eq!(outcome.result["raw"], reply);
        assert_eq!(outcome.result["result"], reply["result"]);
    }
}

#[test]
fn matching_aliases_and_explicit_unversioned_observation_keep_the_detail_route() {
    for version in [json!("owned-version"), Value::Null] {
        let mut reply = job();
        reply["job"]["model_version_id"] = version.clone();
        reply["project_id"] = json!("owned-project");
        reply["model_version_id"] = version.clone();
        reply["simulation_case_id"] = json!("owned-case");
        let (outcome, request) = observe(&response(&reply), |url| {
            ServiceHeadlessExecutor::new(url).execute_step(
                "job_fetch",
                1,
                &json!({
                "job_id":"owned-job","jobId":"owned-job","projectId":"owned-project",
                "modelVersionId":version,"simulationCaseId":"owned-case"}),
            )
        });
        assert!(request.starts_with("GET /api/v1/jobs/owned-job HTTP/1.1"));
        assert_eq!(outcome.unwrap().result["raw"], reply);
        reply["job"]
            .as_object_mut()
            .unwrap()
            .remove("model_version_id");
        reply.as_object_mut().unwrap().remove("model_version_id");
        rejects(
            reply,
            json!({"job_id":"owned-job","model_version_id":version}),
            true,
        );
    }
    let reply = json!({"job":{"job_id":"owned-job","status":"queued"}});
    let (outcome, _) = observe(&response(&reply), |url| {
        ServiceHeadlessExecutor::new(url).execute_step(
            "job_fetch",
            1,
            &json!({"jobId":"owned-job"}),
        )
    });
    outcome.unwrap();
}

#[test]
fn valid_failed_job_observation_can_continue_an_explicit_diagnostic_chain() {
    let mut reply = job();
    reply["job"]["status"] = json!("failed");
    let ((report, calls), requests) = observe_sequence(
        vec![
            response(&reply),
            response(&json!({"project":{"project_id":"diagnostic-project"}})),
        ],
        |url| run(url, "job_fetch", json!({"job_id":"owned-job"})),
    );
    assert_eq!(report.status, "ok");
    assert_eq!(report.executed_step_count, 2);
    assert_eq!(calls, ["job_fetch", "project_create"]);
    assert_eq!(requests.len(), 2);
    assert_eq!(report.steps[0].result_preview["status"], "failed");
    assert_eq!(report.steps[0].result_preview["job_id"], "owned-job");
}

#[test]
fn job_fetch_keeps_complete_http_rejections_distinct_from_receipt_faults() {
    let body = json!({"error":"job_not_found"}).to_string();
    let response = format!(
        "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let ((report, calls), request) = observe(response.as_bytes(), |url| {
        run(url, "job_fetch", json!({"job_id":"owned-job"}))
    });
    assert_eq!(calls, ["job_fetch"]);
    assert!(request.starts_with("GET /api/v1/jobs/owned-job HTTP/1.1"));
    assert_eq!(report.status, "failed");
    let failure = report.execution_summary.failure.unwrap();
    assert!(failure.message.contains("404"));
    assert!(!failure.message.contains("job_receipt_invalid:"));
    assert!(!failure.message.contains("service_request_outcome_unknown"));
    assert!(!failure.retryable);
}
