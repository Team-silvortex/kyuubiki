use crate::service_executor_ack_loss_tests::{observe, observe_sequence, run};
use crate::{HeadlessExecutor, ServiceHeadlessExecutor};
use crate::{HeadlessModelSourceKind, headless_saved_model_source};
use serde_json::{Value, json};

fn response(value: &Value) -> Vec<u8> {
    let body = value.to_string();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn cases() -> Vec<(&'static str, &'static str, Value, Value)> {
    let version = json!({"version":{"version_id":"owned-version","model_id":"owned-model",
        "project_id":"owned-project","kind":"heat_bar_1d","payload":{"nodes":[],"elements":[]}}});
    let model = json!({"model":{"model_id":"owned-model","project_id":"owned-project",
        "kind":"heat_bar_1d","payload":{"nodes":[],"elements":[]}}});
    let mut cases = Vec::new();
    for action in [
        "solve_from_model_version",
        "solve_and_wait_from_model_version",
    ] {
        cases.push((action, "/api/v1/model-versions/owned-version",
            json!({"model_version_id":"owned-version","endpoints":["127.0.0.1:7001"],"timeout_ms":100,"interval_ms":10}), version.clone()));
        let mut mesh_version = version.clone();
        mesh_version["version"]["kind"] = json!("legacy_custom_kind");
        cases.push((
            action,
            "/api/v1/model-versions/owned-version",
            json!({"model_version_id":"owned-version","study_kind":"heat_bar_1d",
                "endpoints":["127.0.0.1:7001"],"timeout_ms":100,"interval_ms":10}),
            mesh_version,
        ));
    }
    for endpoints in [json!([]), json!(["127.0.0.1:7001"])] {
        cases.push((
            "direct_mesh_solve",
            "/api/v1/model-versions/owned-version",
            json!({"model_version_id":"owned-version","endpoints":endpoints}),
            version.clone(),
        ));
        cases.push((
            "direct_mesh_solve",
            "/api/v1/models/owned-model",
            json!({"model_id":"owned-model","endpoints":endpoints}),
            model.clone(),
        ));
    }
    cases
}

fn rejects(action: &str, path: &str, payload: Value, receipt: Value) {
    let ((report, calls), request) = observe(&response(&receipt), |url| run(url, action, payload));
    assert!(request.starts_with(&format!("GET {path} HTTP/1.1")));
    assert_eq!(calls, [action]);
    assert_eq!(report.status, "failed");
    assert_eq!(report.executed_step_count, 0);
    assert_eq!(report.steps.len(), 1);
    for key in ["job_id", "project_id", "model_id", "model_version_id"] {
        assert!(report.steps[0].result_preview.get(key).is_none());
    }
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "contract_failure");
    assert_eq!(failure.stage, "validation");
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
    assert!(failure.message.starts_with("model_reference_invalid:"));
    assert!(!failure.message.contains("rejected-secret"));
}

#[test]
fn saved_source_content_mismatch_stops_before_submission_or_downstream_writes() {
    for (action, path, mut payload, receipt) in cases() {
        let version = receipt.get("version").is_some();
        payload["expected_model_source"] = json!({
            "schema_version":"kyuubiki.headless-model-source/v1",
            "source_kind":if version {"model_version"} else {"model"},
            "project_id":"owned-project", "model_id":"owned-model",
            "model_version_id":if version {json!("owned-version")} else {Value::Null},
            "kind":receipt[if version {"version"} else {"model"}]["kind"],
            "sha256":"0".repeat(64)
        });
        rejects(action, path, payload, receipt);
    }
}

#[test]
fn malformed_or_redirected_model_source_pins_stop_before_network_io() {
    for (action, _, mut payload, receipt) in cases() {
        let version = receipt.get("version").is_some();
        let source = headless_saved_model_source(
            &receipt[if version { "version" } else { "model" }],
            if version {
                HeadlessModelSourceKind::ModelVersion
            } else {
                HeadlessModelSourceKind::Model
            },
        )
        .unwrap();
        let source = serde_json::to_value(source).unwrap();
        for (key, value) in [
            ("sha256", json!("invalid")),
            ("schema_version", json!("unknown")),
            ("unknown", json!("rejected-secret")),
            (
                "source_kind",
                json!(if version { "model" } else { "model_version" }),
            ),
            (
                if version {
                    "model_version_id"
                } else {
                    "model_id"
                },
                json!("other"),
            ),
        ] {
            let mut invalid = source.clone();
            invalid[key] = value;
            payload["expected_model_source"] = invalid;
            let (report, calls) = run("http://127.0.0.1:1", action, payload.clone());
            assert_eq!(calls, [action]);
            let failure = report.execution_summary.failure.unwrap();
            assert_eq!(failure.category, "contract_failure", "{failure:?}");
            assert!(!failure.retryable);
            assert!(!failure.message.contains("rejected-secret"));
        }
        for invalid in [Value::Null, json!([]), json!(false)] {
            payload["expected_model_source"] = invalid;
            let error = ServiceHeadlessExecutor::new("http://127.0.0.1:1")
                .execute_step(action, 1, &payload)
                .unwrap_err();
            assert!(error.message.starts_with("model_reference_invalid:"));
        }
    }
    let error = ServiceHeadlessExecutor::new("http://127.0.0.1:1")
        .execute_step(
            "direct_mesh_solve",
            1,
            &json!({"study_kind":"heat_bar_1d",
            "input":{},"expected_model_source":{}}),
        )
        .unwrap_err();
    assert!(error.message.starts_with("model_reference_invalid:"));
}

#[test]
fn saved_reads_require_matching_explicit_records_and_safe_parent_identities() {
    for (action, path, payload, receipt) in cases() {
        let key = if receipt.get("version").is_some() {
            "version"
        } else {
            "model"
        };
        for value in [
            Value::Null,
            json!([]),
            json!(false),
            json!(1),
            json!("ready"),
            json!({}),
        ] {
            rejects(action, path, payload.clone(), value.clone());
            rejects(action, path, payload.clone(), json!({key:value}));
        }
        for field in receipt[key]
            .as_object()
            .unwrap()
            .keys()
            .filter(|field| field.ends_with("_id"))
        {
            let mut missing = receipt.clone();
            missing[key].as_object_mut().unwrap().remove(field);
            rejects(action, path, payload.clone(), missing);
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
                let mut invalid = receipt.clone();
                invalid[key][field] = value;
                rejects(action, path, payload.clone(), invalid);
            }
            for value in [Value::Null, json!(7), json!("rejected-secret-other")] {
                let mut repeated = receipt.clone();
                repeated[field] = value;
                rejects(action, path, payload.clone(), repeated);
            }
        }
        let mut wrong = receipt;
        let id_key = if key == "version" {
            "version_id"
        } else {
            "model_id"
        };
        wrong[key][id_key] = json!("rejected-secret-other");
        rejects(action, path, payload, wrong);
    }
}

#[test]
fn saved_reads_require_usable_kind_and_object_payload() {
    for (action, path, payload, receipt) in cases() {
        let key = if receipt.get("version").is_some() {
            "version"
        } else {
            "model"
        };
        for field in ["kind", "payload"] {
            let mut missing = receipt.clone();
            missing[key].as_object_mut().unwrap().remove(field);
            rejects(action, path, payload.clone(), missing);
            let values = if field == "kind" {
                vec![
                    Value::Null,
                    json!(false),
                    json!(7),
                    json!(""),
                    json!(" "),
                    json!("heat_bar_1d "),
                    json!("heat\nbar"),
                ]
            } else {
                vec![
                    Value::Null,
                    json!(false),
                    json!(7),
                    json!("rejected-secret"),
                    json!([]),
                ]
            };
            for value in values {
                let mut invalid = receipt.clone();
                invalid[key][field] = value;
                rejects(action, path, payload.clone(), invalid);
            }
        }
    }
}

#[test]
fn saved_parent_hints_and_version_aliases_cannot_redirect_computation() {
    for (action, path, payload, receipt) in cases() {
        let version = receipt.get("version").is_some();
        let hints = if version {
            vec!["project_id", "projectId", "model_id", "modelId"]
        } else {
            vec!["project_id", "projectId"]
        };
        for key in hints {
            for value in [Value::Null, json!(7), json!("rejected-secret-other")] {
                let mut request = payload.clone();
                request[key] = value;
                rejects(action, path, request, receipt.clone());
            }
        }
        if version {
            for nested in [false, true] {
                let mut wrong = receipt.clone();
                if nested {
                    wrong["version"]["model_version_id"] = json!("rejected-secret-other");
                } else {
                    wrong["model_version_id"] = json!("rejected-secret-other");
                }
                rejects(action, path, payload.clone(), wrong);
            }
        }
    }
}

#[test]
fn ambiguous_inline_sources_and_invalid_reference_aliases_stop_before_network_io() {
    for (action, _, payload, _) in cases() {
        for key in ["input", "model_payload"] {
            for value in [json!({}), Value::Null, json!([])] {
                let mut mixed = payload.clone();
                mixed[key] = value;
                let (report, calls) = run("http://127.0.0.1:1", action, mixed);
                assert_eq!(calls, [action]);
                assert_eq!(report.executed_step_count, 0);
                let failure = report.execution_summary.failure.unwrap();
                assert_eq!(failure.category, "contract_failure");
                assert!(!failure.retryable);
                assert!(failure.message.starts_with("model_reference_invalid:"));
            }
        }
        let key = if payload.get("model_version_id").is_some() {
            "model_version_id"
        } else {
            "model_id"
        };
        let alias = if key == "model_id" {
            "modelId"
        } else {
            "modelVersionId"
        };
        for value in [
            Value::Null,
            json!(7),
            json!(""),
            json!(" owned"),
            json!("owned "),
            json!("../other"),
        ] {
            let mut request = payload.clone();
            request[key] = value;
            let error = ServiceHeadlessExecutor::new("http://127.0.0.1:1")
                .execute_step(action, 1, &request)
                .unwrap_err();
            assert!(error.message.starts_with("model_reference_invalid:"));
        }
        let mut request = payload;
        request[alias] = json!("other");
        let error = ServiceHeadlessExecutor::new("http://127.0.0.1:1")
            .execute_step(action, 1, &request)
            .unwrap_err();
        assert!(error.message.starts_with("model_reference_invalid:"));
    }
}

#[test]
fn valid_saved_reads_preserve_context_in_native_and_explicit_mesh_routes() {
    for (action, _, mut payload, mut receipt) in cases()
        .into_iter()
        .filter(|case| case.0 != "solve_and_wait_from_model_version")
    {
        let key = if receipt.get("version").is_some() {
            "version"
        } else {
            "model"
        };
        let record = receipt[key].clone();
        for (field, value) in record.as_object().unwrap() {
            if field.ends_with("_id") {
                receipt[field] = value.clone();
            }
        }
        payload["projectId"] = json!("owned-project");
        let expected = headless_saved_model_source(
            &receipt[key],
            if key == "version" {
                HeadlessModelSourceKind::ModelVersion
            } else {
                HeadlessModelSourceKind::Model
            },
        )
        .unwrap();
        payload["expected_model_source"] = serde_json::to_value(&expected).unwrap();
        if key == "version" {
            payload["modelId"] = json!("owned-model");
            receipt["model_version_id"] = json!("owned-version");
            receipt["version"]["model_version_id"] = json!("owned-version");
        }
        let (outcome, requests) = observe_sequence(
            vec![
                response(&receipt),
                response(
                    &json!({"job":{"job_id":"owned-job","status":"queued","project_id":"owned-project",
                    "model_version_id":if key == "version" {json!("owned-version")} else {Value::Null}}}),
                ),
            ],
            |url| ServiceHeadlessExecutor::new(url).execute_step(action, 1, &payload),
        );
        assert_eq!(
            outcome.unwrap().result["model_source"],
            serde_json::to_value(expected).unwrap()
        );
        assert_eq!(requests.len(), 2);
        let body: Value =
            serde_json::from_str(requests[1].split_once("\r\n\r\n").unwrap().1).unwrap();
        let input = if requests[1].starts_with("POST /api/direct-mesh/solve ") {
            &body["input"]
        } else {
            &body
        };
        assert_eq!(input["project_id"], "owned-project");
        assert_eq!(input["nodes"], json!([]));
        assert!(input.get("expected_model_source").is_none());
        assert!(input.get("model_source").is_none());
        if key == "version" {
            assert_eq!(input["model_version_id"], "owned-version");
        } else {
            assert!(
                input.get("model_version_id").is_none(),
                "mutable model is not its latest saved version"
            );
        }
    }
}
