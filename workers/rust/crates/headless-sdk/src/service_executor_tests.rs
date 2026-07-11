use super::*;
use std::net::TcpListener;

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

#[test]
fn direct_fem_submit_sends_solid_tetra_model_to_route() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let port = listener.local_addr().expect("local addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut buffer = [0_u8; 4096];
        let bytes_read = stream.read(&mut buffer).expect("read request");
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        assert!(request.starts_with("POST /api/v1/fem/solid-tetra-3d/jobs HTTP/1.1\r\n"));
        assert!(request.contains("\"id\":\"tet0\""));
        assert!(!request.contains("\"ignored\":true"));
        let body = r#"{"job":{"job_id":"solid_job","status":"queued","progress":0.0}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"));
    let outcome = executor
        .execute_step(
            "solve_solid_tetra_3d",
            1,
            &json!({
                "model": {
                    "nodes": [{ "id": "n0" }],
                    "elements": [{ "id": "tet0" }]
                },
                "ignored": true
            }),
        )
        .expect("solid tetra direct FEM request should succeed");

    handle.join().expect("server thread should finish");
    assert_eq!(outcome.status, "executed");
    assert_eq!(outcome.result["job_id"].as_str(), Some("solid_job"));
    assert_eq!(outcome.result["status"].as_str(), Some("queued"));
}

#[test]
fn operator_task_prepare_uses_control_plane_endpoint() {
    let payload = json!({
        "task": {
            "schema_version": "kyuubiki.operator-task-ir/v1"
        }
    });
    let request = build_request(
        "POST",
        "127.0.0.1",
        "/api/v1/operator-tasks/prepare",
        Some(&payload.to_string()),
        Some("secret-token"),
    );

    assert!(request.starts_with("POST /api/v1/operator-tasks/prepare HTTP/1.1\r\n"));
    assert!(request.contains("Authorization: Bearer secret-token\r\n"));
    assert!(request.contains("\"schema_version\":\"kyuubiki.operator-task-ir/v1\""));
}

#[test]
fn operator_task_execute_uses_control_plane_endpoint() {
    let payload = json!({
        "task": {
            "schema_version": "kyuubiki.operator-task-ir/v1"
        }
    });
    let request = build_request(
        "POST",
        "127.0.0.1",
        "/api/v1/operator-tasks/execute",
        Some(&payload.to_string()),
        Some("secret-token"),
    );

    assert!(request.starts_with("POST /api/v1/operator-tasks/execute HTTP/1.1\r\n"));
    assert!(request.contains("Authorization: Bearer secret-token\r\n"));
}

#[test]
fn operator_task_prepare_round_trips_against_local_http_server() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let port = listener.local_addr().expect("local addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut buffer = [0_u8; 4096];
        let bytes_read = stream.read(&mut buffer).expect("read request");
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        assert!(request.starts_with("POST /api/v1/operator-tasks/prepare HTTP/1.1\r\n"));
        assert!(request.contains("\"task\":"));
        let body = r#"{"status":"verified","task_digest":"abc","operator_id":"transform.demo"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"));
    let outcome = executor
        .execute_step(
            "operator_task_prepare",
            1,
            &json!({ "task": { "schema_version": "kyuubiki.operator-task-ir/v1" } }),
        )
        .expect("service request should succeed");

    handle.join().expect("server thread should finish");
    assert_eq!(outcome.status, "executed");
    assert_eq!(outcome.result["status"], "verified");
    assert_eq!(outcome.result["operator_id"], "transform.demo");
}

#[test]
fn operator_task_execute_preserves_readiness_from_control_plane() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let port = listener.local_addr().expect("local addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut buffer = [0_u8; 4096];
        let bytes_read = stream.read(&mut buffer).expect("read request");
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        assert!(request.starts_with("POST /api/v1/operator-tasks/execute HTTP/1.1\r\n"));
        assert!(request.contains("\"task\":"));
        let body = r#"{"status":"verified_pending_execution","execution_readiness":{"status":"blocked","current_stage":"fetch_package","required_action":"attach_operator_package_runtime"},"package_fetch_request":{"request_status":"blocked_runtime_not_attached"},"execution_plan":[{"stage":"fetch_package","gate":"blocked"}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"));
    let outcome = executor
        .execute_step(
            "operator_task_execute",
            1,
            &json!({ "task": { "schema_version": "kyuubiki.operator-task-ir/v1" } }),
        )
        .expect("service request should preserve readiness");

    handle.join().expect("server thread should finish");
    assert_eq!(outcome.status, "executed");
    assert_eq!(outcome.result["execution_readiness"]["status"], "blocked");
    assert_eq!(
        outcome.result["execution_readiness"]["required_action"],
        "attach_operator_package_runtime"
    );
    assert_eq!(
        outcome.result["package_fetch_request"]["request_status"],
        "blocked_runtime_not_attached"
    );
    assert_eq!(outcome.result["execution_plan"][0]["gate"], "blocked");
}

#[test]
fn non_success_response_includes_json_error_payload() {
    let response = "HTTP/1.1 422 Unprocessable Entity\r\nContent-Type: application/json\r\n\r\n{\"error\":\"operator_task_digest_mismatch\",\"error_code\":\"operator_task_digest_mismatch\"}";
    let error = parse_json_response(response, "/api/v1/operator-tasks/prepare")
        .expect_err("422 should be an error");

    assert!(error.message.contains("422"));
    assert!(error.message.contains("operator_task_digest_mismatch"));
}

#[test]
fn non_success_response_promotes_operator_task_error_code() {
    let response = "HTTP/1.1 422 Unprocessable Entity\r\nContent-Type: application/json\r\n\r\n{\"error\":\"{:operator_task_mirror_mismatch, %{}}\",\"error_code\":\"operator_task_mirror_mismatch\"}";
    let error = parse_json_response(response, "/api/v1/operator-tasks/prepare")
        .expect_err("422 should be an error");

    assert!(error.message.contains("operator_task_mirror_mismatch"));
    assert!(error.message.contains("/api/v1/operator-tasks/prepare"));
}
