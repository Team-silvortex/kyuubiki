use crate::service_executor_ack_loss_tests::{observe, observe_sequence};
use crate::{HeadlessExecutor, ServiceHeadlessExecutor, all_direct_fem_routes};
use serde_json::{Value, json};

fn response(value: Value) -> Vec<u8> {
    let body = value.to_string();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn request_body(request: &str) -> Value {
    serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
}

#[test]
fn every_direct_solver_keeps_explicit_project_and_model_version_context() {
    for route in all_direct_fem_routes() {
        let (outcome, request) = observe(
            &response(
                json!({"job":{"job_id":"job","status":"queued","project_id":"owned-project","model_version_id":"owned-version"}}),
            ),
            |url| {
                ServiceHeadlessExecutor::new(url).execute_step(route.action, 1,
                &json!({"model":{"tip_force":1000},"project_id":"owned-project","model_version_id":"owned-version"}))
            },
        );
        outcome.unwrap();
        assert!(request.starts_with(&format!("POST {} HTTP/1.1", route.route)));
        assert_eq!(
            request_body(&request),
            json!({"tip_force":1000,"project_id":"owned-project","model_version_id":"owned-version"})
        );
    }
}

#[test]
fn invalid_context_is_rejected_before_any_connection_or_artifact_upload() {
    for key in ["project_id", "model_version_id"] {
        for value in [
            Value::Null,
            json!(7),
            json!(""),
            json!(" owned"),
            json!("owned "),
            json!("../other"),
            json!("a%2fb"),
        ] {
            let mut payload = json!({"model":{"nodes":vec![json!({});250_000]}});
            payload[key] = value;
            let error = ServiceHeadlessExecutor::new("http://127.0.0.1:1")
                .execute_step("solve_bar_1d", 1, &payload)
                .unwrap_err();
            assert!(error.message.contains(key));
            assert!(!error.message.contains("connect"));
            assert!(!error.message.contains("service_request_outcome_unknown"));
        }
    }
}

#[test]
fn saved_version_solve_forwards_the_canonical_reference_not_only_a_local_label() {
    let version = json!({"version":{"version_id":"owned-version","model_id":"owned-model","project_id":"owned-project",
        "kind":"axial_bar_1d","payload":{"tip_force":2000}}});
    let (outcome, requests) = observe_sequence(
        vec![
            response(version),
            response(json!({"job":{"job_id":"job","status":"queued",
            "project_id":"owned-project","model_version_id":"owned-version"}})),
        ],
        |url| {
            ServiceHeadlessExecutor::new(url).execute_step(
                "solve_from_model_version",
                1,
                &json!({"model_version_id":"owned-version"}),
            )
        },
    );
    let outcome = outcome.unwrap();
    assert_eq!(outcome.result["model_version_id"], "owned-version");
    assert_eq!(outcome.result["job"]["model_version_id"], "owned-version");
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with("GET /api/v1/model-versions/owned-version HTTP/1.1"));
    assert_eq!(
        request_body(&requests[1]),
        json!({"tip_force":2000,"model_version_id":"owned-version","project_id":"owned-project"})
    );
}
