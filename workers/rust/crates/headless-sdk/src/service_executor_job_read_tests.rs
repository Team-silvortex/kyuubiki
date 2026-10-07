use crate::service_executor_ack_loss_tests::{observe, observe_sequence, run};
use crate::{HeadlessExecutor, ServiceHeadlessExecutor};
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
    json!({"job":{"job_id":"owned-job","status":"completed","project_id":"owned-project",
        "model_version_id":"owned-version","simulation_case_id":"owned-case"}})
}

fn assert_invalid(report: &crate::HeadlessRunReport) {
    assert_eq!(report.status, "failed", "{report:?}");
    assert_eq!(report.executed_step_count, 0);
    assert_eq!(report.steps.len(), 1);
    let failure = report.execution_summary.failure.as_ref().unwrap();
    assert_eq!(failure.category, "contract_failure");
    assert_eq!(failure.stage, "validation");
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
    assert!(failure.message.starts_with("job_read_invalid:"));
    assert!(!failure.message.contains("rejected-secret"));
    for key in ["job_id", "result", "project_id", "model_version_id"] {
        assert!(report.steps[0].result_preview.get(key).is_none());
    }
}

#[test]
fn task_read_identities_and_parent_aliases_are_exact_before_network_io() {
    for action in ["job_fetch", "job_wait", "result_fetch"] {
        for (key, alias) in [
            ("job_id", "jobId"),
            ("project_id", "projectId"),
            ("model_version_id", "modelVersionId"),
            ("simulation_case_id", "simulationCaseId"),
        ] {
            for value in [
                Value::Null,
                json!(false),
                json!(7),
                json!(""),
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
                if key == "model_version_id" && value.is_null() {
                    continue;
                }
                for name in [key, alias] {
                    let mut payload = json!({"job_id":"owned-job"});
                    payload[name] = value.clone();
                    let error = ServiceHeadlessExecutor::new("http://127.0.0.1:1")
                        .execute_step(action, 1, &payload)
                        .unwrap_err();
                    assert!(error.message.starts_with("job_read_invalid:"));
                    assert!(!error.message.contains("rejected-secret"));
                }
            }
            let mut payload = json!({"job_id":"owned-job"});
            payload[key] = json!("owned");
            payload[alias] = json!("rejected-secret-other");
            let (report, calls) = run("http://127.0.0.1:1", action, payload);
            assert_eq!(calls, [action]);
            assert_invalid(&report);
        }
    }
}

#[test]
fn malformed_result_preferences_stop_reads_and_combined_solves_before_io() {
    for action in ["result_fetch", "solve_and_wait_from_model_version"] {
        for key in ["prefer_job_result", "preferJobResult"] {
            for value in [Value::Null, json!(0), json!("false"), json!([]), json!({})] {
                let mut payload = json!({"job_id":"owned-job","model_version_id":"owned-version",
                    "endpoints":["127.0.0.1:7001"]});
                payload[key] = value;
                let (report, calls) = run("http://127.0.0.1:1", action, payload);
                assert_eq!(calls, [action]);
                assert_invalid(&report);
            }
        }
        let payload = json!({"job_id":"owned-job","model_version_id":"owned-version",
            "endpoints":["127.0.0.1:7001"],"prefer_job_result":false,"preferJobResult":true});
        let (report, calls) = run("http://127.0.0.1:1", action, payload);
        assert_eq!(calls, [action]);
        assert_invalid(&report);
    }
}

#[test]
fn requested_associations_and_repeated_context_gate_every_job_read() {
    for action in ["job_wait", "result_fetch"] {
        for prefer in [true, false] {
            for (key, alias) in [
                ("project_id", "projectId"),
                ("model_version_id", "modelVersionId"),
                ("simulation_case_id", "simulationCaseId"),
            ] {
                for name in [key, alias] {
                    for value in [
                        None,
                        Some(Value::Null),
                        Some(json!("rejected-secret-other")),
                    ] {
                        let mut receipt = job();
                        match value {
                            Some(value) => {
                                receipt["job"][key] = value;
                            }
                            None => {
                                receipt["job"].as_object_mut().unwrap().remove(key);
                            }
                        }
                        let mut payload = json!({"job_id":"owned-job","prefer_job_result":prefer});
                        payload[name] = job()["job"][key].clone();
                        let ((report, calls), request) =
                            observe(&response(&receipt), |url| run(url, action, payload));
                        assert_eq!(calls, [action]);
                        assert_invalid(&report);
                        let suffix = if action == "job_wait" || !prefer {
                            "/status"
                        } else {
                            ""
                        };
                        assert!(
                            request.starts_with(&format!(
                                "GET /api/v1/jobs/owned-job{suffix} HTTP/1.1"
                            ))
                        );
                    }
                }
                let mut receipt = job();
                receipt[key] = json!("rejected-secret-other");
                let ((report, calls), _) = observe(&response(&receipt), |url| {
                    run(
                        url,
                        action,
                        json!({"job_id":"owned-job","prefer_job_result":prefer}),
                    )
                });
                assert_eq!(calls, [action]);
                assert_invalid(&report);
            }
        }
    }
}

#[test]
fn every_poll_keeps_explicit_version_constraint_until_completion() {
    let mut solving = job();
    solving["job"]["status"] = json!("solving");
    let mut wrong = job();
    wrong["job"]["model_version_id"] = json!("rejected-secret-other");
    let ((report, calls), requests) =
        observe_sequence(vec![response(&solving), response(&wrong)], |url| {
            run(
                url,
                "job_wait",
                json!({"job_id":"owned-job","model_version_id":"owned-version",
            "timeout_ms":1000,"interval_ms":1}),
            )
        });
    assert_eq!(calls, ["job_wait"]);
    assert_eq!(requests.len(), 2);
    assert_invalid(&report);
}

#[test]
fn separate_result_context_must_agree_with_verified_job_metadata() {
    for prefer in [true, false] {
        for key in ["project_id", "model_version_id", "simulation_case_id"] {
            for nested in [true, false] {
                for value in [
                    None,
                    Some(Value::Null),
                    Some(json!("rejected-secret-other")),
                ] {
                    if !nested && value.is_none() {
                        continue;
                    }
                    let mut receipt = json!({"job_id":"owned-job","result":{"value":1}});
                    if nested {
                        receipt["job"] = job()["job"].clone();
                    }
                    let target = if nested {
                        &mut receipt["job"]
                    } else {
                        &mut receipt
                    };
                    match value {
                        Some(value) => {
                            target[key] = value;
                        }
                        None => {
                            target.as_object_mut().unwrap().remove(key);
                        }
                    }
                    let ((report, calls), requests) =
                        observe_sequence(vec![response(&job()), response(&receipt)], |url| {
                            run(
                                url,
                                "result_fetch",
                                json!({"job_id":"owned-job","prefer_job_result":prefer}),
                            )
                        });
                    assert_eq!(calls, ["result_fetch"]);
                    assert_eq!(requests.len(), 2);
                    assert_invalid(&report);
                }
            }
        }
    }
}

#[test]
fn valid_aliases_and_nullable_version_constraints_preserve_fetch_routes() {
    for version in [json!("owned-version"), Value::Null] {
        for prefer in [true, false] {
            for fallback in [true, false] {
                let result = json!({"nodes":[1,2],"literal":"{{steps.1.result.job_id}}"});
                let mut receipt = job();
                receipt["job"]["model_version_id"] = version.clone();
                if prefer && !fallback {
                    receipt["result"] = result.clone();
                }
                let mut replies = vec![response(&receipt)];
                if !prefer || fallback {
                    let second = json!({"job_id":"owned-job","job":receipt["job"],
                        "project_id":"owned-project","model_version_id":version,"result":result});
                    replies.push(response(&second));
                }
                let payload = json!({"job_id":"owned-job","jobId":"owned-job",
                    "projectId":"owned-project","modelVersionId":version,"simulationCaseId":"owned-case",
                    "prefer_job_result":prefer,"preferJobResult":prefer});
                let (outcome, requests) = observe_sequence(replies, |url| {
                    ServiceHeadlessExecutor::new(url).execute_step("result_fetch", 1, &payload)
                });
                let outcome = outcome.unwrap();
                assert_eq!(outcome.result["result"], result);
                assert!(outcome.result.get("raw").is_none());
                assert_eq!(requests.len(), if prefer && !fallback { 1 } else { 2 });
                let suffix = if prefer { "" } else { "/status" };
                assert!(
                    requests[0]
                        .starts_with(&format!("GET /api/v1/jobs/owned-job{suffix} HTTP/1.1"))
                );
            }
        }
    }
}

#[test]
fn saved_version_combined_solve_stops_before_fetching_a_misassociated_job() {
    let loaded = json!({"version":{"version_id":"owned-version","project_id":"owned-project",
        "model_id":"owned-model","kind":"heat_bar_1d","payload":{"nodes":[],"elements":[]}}});
    let submitted = json!({"job":{"job_id":"owned-job","status":"queued","project_id":"owned-project","model_version_id":"owned-version"}});
    for key in ["model_version_id", "project_id"] {
        let mut wrong = job();
        wrong["job"][key] = json!("rejected-secret-other");
        let ((report, calls), requests) = observe_sequence(
            vec![response(&loaded), response(&submitted), response(&wrong)],
            |url| {
                run(
                    url,
                    "solve_and_wait_from_model_version",
                    json!({"model_version_id":"owned-version","projectId":"owned-project",
                    "endpoints":["127.0.0.1:7001"],"timeout_ms":1000,"interval_ms":1}),
                )
            },
        );
        assert_eq!(calls, ["solve_and_wait_from_model_version"]);
        assert_eq!(requests.len(), 3);
        assert!(requests[1].starts_with("POST /api/v1/fem/heat-bar-1d/jobs HTTP/1.1"));
        assert!(requests[2].starts_with("GET /api/v1/jobs/owned-job/status HTTP/1.1"));
        assert_invalid(&report);
    }
}
