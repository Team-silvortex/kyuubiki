use super::*;

#[test]
fn parses_http_url_with_port() {
    let parsed = parse_http_url("http://127.0.0.1:3000").expect("parse base url");
    assert_eq!(parsed.host, "127.0.0.1");
    assert_eq!(parsed.port, 3000);
}

#[test]
fn builds_post_request_with_json_body() {
    let request = build_request(
        "POST",
        "127.0.0.1",
        "/api/v1/workflows/catalog/wf_demo/jobs",
        Some("{\"input_artifacts\":{}}"),
        Some("secret-token"),
    );
    assert!(request.starts_with("POST /api/v1/workflows/catalog/wf_demo/jobs HTTP/1.1\r\n"));
    assert!(request.contains("Authorization: Bearer secret-token\r\n"));
    assert!(request.contains("Content-Type: application/json\r\n"));
    assert!(request.contains("\r\n\r\n{\"input_artifacts\":{}}"));
}

#[test]
fn parses_json_response_payload() {
    let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"job\":{\"status\":\"completed\"}}";
    let payload =
        parse_json_response(response, "/api/v1/jobs/job_123").expect("parse response payload");
    assert_eq!(payload["job"]["status"].as_str(), Some("completed"));
}

#[test]
fn rejects_non_http_base_url() {
    let error = parse_http_url("https://example.com").expect_err("https should fail");
    assert!(error.message.contains("only http:// is supported"));
}

#[test]
fn picks_string_and_u64_values() {
    let payload = json!({
        "job_id": "job_123",
        "interval_ms": "25"
    });
    assert_eq!(pick_string(&payload, &["job_id"]), Some("job_123"));
    assert_eq!(pick_u64(&payload, &["interval_ms"]), Some(25));
}

#[test]
fn normalizes_job_submission_for_bindings() {
    let normalized = normalize_job_submission_result(json!({
        "job": {
            "job_id": "job_123",
            "status": "queued",
            "progress": 0.0
        }
    }));
    assert_eq!(normalized["job_id"].as_str(), Some("job_123"));
    assert_eq!(normalized["status"].as_str(), Some("queued"));
}

#[test]
fn normalizes_job_state_for_bindings() {
    let normalized = normalize_job_state_result(json!({
        "job": {
            "job_id": "job_123",
            "status": "completed",
            "progress": 1.0
        },
        "result": {
            "artifact": "ok"
        }
    }));
    assert_eq!(normalized["job_id"].as_str(), Some("job_123"));
    assert_eq!(normalized["status"].as_str(), Some("completed"));
    assert_eq!(normalized["result"]["artifact"].as_str(), Some("ok"));
}

#[test]
fn direct_fem_submit_uses_model_payload_when_present() {
    let payload = json!({
        "model": {
            "nodes": [{ "id": "q0" }],
            "elements": [{ "id": "e0" }]
        },
        "ignored": true
    });
    let request = build_request(
        "POST",
        "127.0.0.1",
        direct_fem_submit_route("solve_plane_quad_2d").expect("route"),
        Some(&payload["model"].to_string()),
        Some("secret-token"),
    );
    assert!(request.starts_with("POST /api/v1/fem/plane-quad-2d/jobs HTTP/1.1\r\n"));
    assert!(request.contains("\"nodes\":[{\"id\":\"q0\"}]"));
    assert!(!request.contains("\"ignored\":true"));
}
