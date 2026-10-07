use crate::service_executor_ack_loss_tests::{observe, run};
use crate::{HeadlessExecutor, ServiceHeadlessExecutor};
use serde_json::{Value, json};

fn cases() -> Vec<(&'static str, &'static str, Value, Value)> {
    vec![
        (
            "project_create",
            "POST /api/v1/projects",
            json!({"name":"owned"}),
            json!({"project":{"project_id":"owned-project","name":"owned"}}),
        ),
        (
            "project_update",
            "PATCH /api/v1/projects/owned-project",
            json!({"project_id":"owned-project","name":"changed"}),
            json!({"project":{"project_id":"owned-project","name":"changed"}}),
        ),
        (
            "project_delete",
            "DELETE /api/v1/projects/owned-project",
            json!({"project_id":"owned-project"}),
            json!({"project":{"project_id":"owned-project"}}),
        ),
        (
            "model_create",
            "POST /api/v1/projects/owned-project/models",
            json!({"project_id":"owned-project","name":"owned","kind":"axial_bar_1d","payload":{}}),
            json!({"model":{"model_id":"owned-model","project_id":"owned-project",
                "latest_version_id":"owned-initial","kind":"axial_bar_1d"}}),
        ),
        (
            "model_version_create",
            "POST /api/v1/models/owned-model/versions",
            json!({"model_id":"owned-model","payload":{}}),
            json!({"version":{"version_id":"owned-version","model_id":"owned-model",
                "project_id":"owned-project","kind":"axial_bar_1d"}}),
        ),
    ]
}

fn envelope(action: &str) -> &'static str {
    match action {
        "model_create" => "model",
        "model_version_create" => "version",
        _ => "project",
    }
}

fn response(receipt: &Value) -> Vec<u8> {
    let body = serde_json::to_string(receipt).unwrap();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn rejects(action: &str, path: &str, payload: Value, receipt: Value) {
    let ((report, calls), request) = observe(&response(&receipt), |url| run(url, action, payload));
    assert!(request.starts_with(&format!("{path} HTTP/1.1")));
    assert_eq!(calls, [action]);
    assert_eq!(report.status, "failed");
    assert_eq!(report.executed_step_count, 0);
    assert_eq!(report.steps.len(), 1);
    for key in [
        "project_id",
        "model_id",
        "version_id",
        "model_version_id",
        "latest_version_id",
    ] {
        assert!(report.steps[0].result_preview.get(key).is_none());
    }
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "service_request_outcome_unknown");
    assert_eq!(failure.stage, "transport");
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
    assert!(failure.message.contains("library_receipt_invalid:"));
    assert!(!failure.message.contains("rejected-secret"));
}

#[test]
fn every_library_write_requires_an_explicit_record() {
    for (action, path, payload, _) in cases() {
        for value in [
            Value::Null,
            json!(false),
            json!(1),
            json!("ok"),
            json!([]),
            json!({}),
            json!({"project_id":"owned-project","model_id":"owned-model","version_id":"owned-version"}),
        ] {
            rejects(action, path, payload.clone(), value);
        }
        for value in [Value::Null, json!([]), json!(1), json!("ok")] {
            rejects(
                action,
                path,
                payload.clone(),
                json!({envelope(action):value}),
            );
        }
    }
}

#[test]
fn every_exposed_library_identity_is_required_safe_and_consistent() {
    for (action, path, payload, receipt) in cases() {
        let key = envelope(action);
        for field in receipt[key]
            .as_object()
            .unwrap()
            .keys()
            .filter(|field| field.ends_with("_id"))
        {
            let mut missing = receipt.clone();
            missing[key].as_object_mut().unwrap().remove(field);
            rejects(action, path, payload.clone(), missing);
            for id in [
                Value::Null,
                json!(false),
                json!(7),
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
                let mut malformed = receipt.clone();
                malformed[key][field] = id;
                rejects(action, path, payload.clone(), malformed);
            }
            for id in [Value::Null, json!(7), json!("rejected-secret-other")] {
                let mut conflicting = receipt.clone();
                conflicting[field] = id;
                rejects(action, path, payload.clone(), conflicting);
            }
        }
    }
}

#[test]
fn writes_must_acknowledge_the_requested_object_or_parent() {
    for (action, path, payload, mut receipt) in cases().into_iter().skip(1) {
        let field = if action == "model_version_create" {
            "model_id"
        } else {
            "project_id"
        };
        receipt[envelope(action)][field] = json!("rejected-secret-other");
        rejects(action, path, payload, receipt);
    }
}

#[test]
fn version_aliases_cannot_contradict_the_canonical_identity() {
    let (action, path, payload, receipt) = cases().pop().unwrap();
    for value in [Value::Null, json!(7), json!("rejected-secret-other")] {
        for nested in [false, true] {
            let mut conflicting = receipt.clone();
            if nested {
                conflicting["version"]["model_version_id"] = value.clone();
            } else {
                conflicting["model_version_id"] = value.clone();
            }
            rejects(action, path, payload.clone(), conflicting);
        }
    }
}

#[test]
fn valid_records_preserve_original_bindings_and_raw_receipts() {
    for (action, path, payload, mut receipt) in cases() {
        let record = receipt[envelope(action)].clone();
        for (key, value) in record.as_object().unwrap() {
            if key.ends_with("_id") {
                receipt[key] = value.clone();
            }
        }
        if action == "model_version_create" {
            receipt["model_version_id"] = record["version_id"].clone();
            receipt["version"]["model_version_id"] = record["version_id"].clone();
        }
        let (outcome, request) = observe(&response(&receipt), |url| {
            ServiceHeadlessExecutor::new(url).execute_step(action, 1, &payload)
        });
        assert!(request.starts_with(&format!("{path} HTTP/1.1")));
        let outcome = outcome.unwrap();
        assert_eq!(outcome.status, "executed");
        assert_eq!(outcome.result["raw"], receipt);
        assert_eq!(outcome.result[envelope(action)], receipt[envelope(action)]);
        for (key, value) in record.as_object().unwrap() {
            assert_eq!(outcome.result[key], *value);
        }
        if action == "model_version_create" {
            assert_eq!(outcome.result["model_version_id"], record["version_id"]);
        }
    }
}

#[test]
fn no_content_is_not_a_library_delete_receipt() {
    let ((report, calls), request) = observe(
        b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n",
        |url| run(url, "project_delete", json!({"project_id":"owned-project"})),
    );
    assert!(request.starts_with("DELETE /api/v1/projects/owned-project HTTP/1.1"));
    assert_eq!(calls, ["project_delete"]);
    assert_eq!(report.executed_step_count, 0);
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "service_request_outcome_unknown");
    assert!(!failure.retryable);
}
