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
fn rejects_oversized_inline_json_before_socket_submission() {
    let error = validate_inline_json_size(
        "/api/v1/fem/heat-plane-quad-2d/jobs",
        MAX_INLINE_JSON_BYTES + 1,
    )
    .expect_err("oversized inline JSON must use a reference transport");

    assert!(error.message.contains("inline JSON transport limit"));
    assert!(error.message.contains("size_bytes=8000001"));
    assert!(error.message.contains("limit_bytes=8000000"));
    assert!(error.message.contains("model or artifact reference"));
}

#[test]
fn large_artifact_errors_point_away_from_frontend_proxies() {
    let message = service_error_message(500, "/api/v1/model-artifacts", &json!("truncated"));

    assert!(message.contains("frontend_proxy_artifact_limit"));
    assert!(message.contains("runtime control-plane endpoint"));
    assert!(message.contains("not a frontend proxy"));

    let validation = service_error_message(400, "/api/v1/model-artifacts", &json!("invalid"));
    assert!(!validation.contains("frontend_proxy_artifact_limit"));
}

#[test]
fn rejects_path_and_header_injection_inputs() {
    assert!(sanitize_request_path("/api/health").is_ok());
    assert!(sanitize_request_path("/api/v1/jobs/../secrets").is_err());
    assert!(sanitize_request_path("/api/v1/jobs/job_1\r\nX-Bad: yes").is_err());
    assert!(sanitize_header_value(Some("secret-token"), "api token").is_ok());
    assert!(sanitize_header_value(Some("secret\r\nX-Bad: yes"), "api token").is_err());
}

#[test]
fn rejects_unsafe_dynamic_path_segments() {
    let payload = json!({ "job_id": "../other" });
    let error = required_path_segment(&payload, &["job_id"]).expect_err("unsafe job id");
    assert!(error.message.contains("safe path segment"));

    let payload = json!({ "workflow_id": "catalog/demo" });
    let error = required_path_segment(&payload, &["workflow_id"]).expect_err("unsafe workflow id");
    assert!(error.message.contains("safe path segment"));
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
fn strict_constructor_rejects_ambiguous_base_urls() {
    for base_url in [
        "http://127.0.0.1:3000/not-api",
        "http://127.0.0.1:3000?mode=test",
        "http://127.0.0.1:3000#fragment",
        "http://user@127.0.0.1:3000",
        "http://127.0.0.1:0",
        "",
    ] {
        assert!(
            ServiceHeadlessExecutor::try_new(base_url).is_err(),
            "base URL should fail: {base_url}"
        );
    }
}

#[test]
fn strict_constructor_accepts_plain_authority_with_trailing_slash() {
    ServiceHeadlessExecutor::try_new("http://127.0.0.1:3000/")
        .expect("one trailing slash should normalize safely");
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
fn rejects_failed_terminal_jobs_with_the_service_failure_reason() {
    let failed = json!({
        "status": "failed",
        "job": {"message": "missing field id"}
    });
    let error = reject_unsuccessful_terminal_job("job-failed", &failed)
        .expect_err("failed job must fail the headless run");
    assert!(error.message.contains("terminal status failed"));
    assert!(error.message.contains("missing field id"));

    assert!(
        reject_unsuccessful_terminal_job("job-complete", &json!({"status": "completed"})).is_ok()
    );
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
fn large_direct_fem_submit_uploads_a_model_artifact_then_sends_its_reference() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let port = listener.local_addr().expect("local addr").port();
    let handle = std::thread::spawn(move || {
        let (mut upload, _) = listener.accept().expect("accept artifact upload");
        let upload_request = read_complete_http_request(&mut upload);
        let upload_text = String::from_utf8_lossy(&upload_request);
        assert!(upload_text.starts_with("POST /api/v1/model-artifacts HTTP/1.1\r\n"));
        assert!(upload_text.contains("Content-Type: application/vnd.kyuubiki.model+json\r\n"));
        assert!(upload_request.len() > MAX_INLINE_JSON_BYTES);
        let artifact_id = "a".repeat(64);
        write_test_response(
            &mut upload,
            "201 Created",
            &format!(
                r#"{{"artifact":{{"artifact_id":"{artifact_id}","sha256":"{artifact_id}"}}}}"#
            ),
        );
        drop(upload);

        let (mut submit, _) = listener.accept().expect("accept solve submission");
        let submit_request = read_complete_http_request(&mut submit);
        let submit_text = String::from_utf8_lossy(&submit_request);
        assert!(submit_text.starts_with("POST /api/v1/fem/heat-plane-quad-2d/jobs HTTP/1.1\r\n"));
        assert!(submit_text.contains("\"model_artifact_ref\""));
        assert!(submit_text.contains(&artifact_id));
        assert!(!submit_text.contains("large-model-padding"));
        write_test_response(
            &mut submit,
            "202 Accepted",
            r#"{"job":{"job_id":"artifact-job","status":"queued","progress":0.0}}"#,
        );
    });

    let model = json!({
        "nodes": [],
        "elements": [],
        "large-model-padding": "x".repeat(MAX_INLINE_JSON_BYTES)
    });
    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"));
    let outcome = executor
        .execute_step("solve_heat_plane_quad_2d", 1, &json!({"model": model}))
        .expect("large FEM request should use artifact transport");

    handle.join().expect("server thread should finish");
    assert_eq!(outcome.result["job_id"], "artifact-job");
}

#[test]
fn composite_submit_preserves_the_full_coupled_payload() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let port = listener.local_addr().expect("local addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut buffer = [0_u8; 16_384];
        let bytes_read = stream.read(&mut buffer).expect("read request");
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        assert!(
            request
                .starts_with("POST /api/v1/fem/composite-thermo-electric-panel/jobs HTTP/1.1\r\n")
        );
        assert!(request.contains("\"electrostatic_model\""));
        assert!(request.contains("\"electric_conduction_model\""));
        assert!(request.contains("\"thermal_expansion_feedback\""));
        let body = r#"{"job":{"job_id":"composite-job","status":"queued","progress":0.0}}"#;
        write_test_response(&mut stream, "200 OK", body);
    });

    let step = crate::build_composite_panel_steps()
        .into_iter()
        .next()
        .expect("composite study should include a candidate");
    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"));
    let outcome = executor
        .execute_step(&step.action, 1, &step.payload)
        .expect("composite submission should succeed");

    handle.join().expect("server thread should finish");
    assert_eq!(outcome.result["job_id"], "composite-job");
    assert_eq!(outcome.result["status"], "queued");
}

#[test]
fn service_executor_covers_every_service_action_contract() {
    let missing = crate::all_action_contracts()
        .iter()
        .filter(|contract| contract.engine == crate::HeadlessEngine::Service)
        .filter(|contract| !service_executor_supports_action(contract.id))
        .map(|contract| contract.id)
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "service contracts without native executor support: {missing:?}"
    );
}

#[test]
fn direct_mesh_solve_posts_normalized_study_request() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let port = listener.local_addr().expect("local addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut buffer = [0_u8; 8192];
        let bytes_read = stream.read(&mut buffer).expect("read request");
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        assert!(request.starts_with("POST /api/direct-mesh/solve HTTP/1.1\r\n"));
        assert!(request.contains("\"study_kind\":\"heat_bar_1d\""));
        assert!(request.contains("\"endpoints\":[\"127.0.0.1:7001\"]"));
        assert!(request.contains("\"nodes\":[{\"id\":\"n0\"}]"));
        let body = r#"{"job":{"job_id":"mesh-job","status":"queued","progress":0.0},"direct_mesh":{"endpoint":"127.0.0.1:7001"}}"#;
        write_test_response(&mut stream, "200 OK", body);
    });

    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"));
    let outcome = executor
        .execute_step(
            "direct_mesh_solve",
            1,
            &json!({
                "study_kind": "heat_bar_1d",
                "input": {
                    "nodes": [{ "id": "n0" }],
                    "elements": [{ "id": "e0" }]
                },
                "endpoints": ["127.0.0.1:7001"]
            }),
        )
        .expect("direct mesh request should succeed");

    handle.join().expect("server thread should finish");
    assert_eq!(outcome.result["job_id"], "mesh-job");
    assert_eq!(outcome.result["endpoint"], "127.0.0.1:7001");
}

#[test]
fn direct_mesh_solve_uses_native_fem_route_without_explicit_endpoints() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let port = listener.local_addr().expect("local addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut buffer = [0_u8; 8192];
        let bytes_read = stream.read(&mut buffer).expect("read request");
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        assert!(request.starts_with("POST /api/v1/fem/axial-bar/jobs HTTP/1.1\r\n"));
        assert!(request.contains("\"nodes\":[{\"id\":\"n0\"}]"));
        write_test_response(
            &mut stream,
            "200 OK",
            r#"{"job":{"job_id":"native-job","status":"queued","progress":0.0}}"#,
        );
    });

    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"));
    let outcome = executor
        .execute_step(
            "direct_mesh_solve",
            1,
            &json!({
                "study_kind": "axial_bar_1d",
                "input": {
                    "nodes": [{ "id": "n0" }],
                    "elements": [{ "id": "e0" }]
                }
            }),
        )
        .expect("native direct mesh request should succeed");

    handle.join().expect("server thread should finish");
    assert_eq!(outcome.result["job_id"], "native-job");
}

#[test]
fn direct_mesh_solve_resolves_model_and_version_references() {
    assert_direct_mesh_reference(
        "model_id",
        "model_native",
        "GET /api/v1/models/model_native HTTP/1.1",
        r#"{"model":{"model_id":"model_native","kind":"heat_bar_1d","project_id":"project-native","payload":{"nodes":[{"id":"n0"}],"elements":[{"id":"e0"}]}}}"#,
    );
    assert_direct_mesh_reference(
        "model_version_id",
        "version_native",
        "GET /api/v1/model-versions/version_native HTTP/1.1",
        r#"{"version":{"version_id":"version_native","kind":"heat_bar_1d","project_id":"project-native","payload":{"nodes":[{"id":"n0"}],"elements":[{"id":"e0"}]}}}"#,
    );
}

fn assert_direct_mesh_reference(
    reference_key: &str,
    reference_id: &str,
    expected_get: &'static str,
    envelope: &'static str,
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let port = listener.local_addr().expect("local addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept model request");
        let mut buffer = [0_u8; 8192];
        let bytes_read = stream.read(&mut buffer).expect("read model request");
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        assert!(request.starts_with(expected_get), "request: {request}");
        write_test_response(&mut stream, "200 OK", envelope);
        drop(stream);

        let (mut stream, _) = listener.accept().expect("accept solve request");
        let bytes_read = stream.read(&mut buffer).expect("read solve request");
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        assert!(request.starts_with("POST /api/direct-mesh/solve HTTP/1.1\r\n"));
        assert!(request.contains("\"study_kind\":\"heat_bar_1d\""));
        assert!(request.contains("\"nodes\":[{\"id\":\"n0\"}]"));
        write_test_response(
            &mut stream,
            "200 OK",
            r#"{"job":{"job_id":"referenced-job","status":"queued","progress":0.0}}"#,
        );
    });

    let mut payload = json!({ "endpoints": ["127.0.0.1:7001"] });
    payload[reference_key] = Value::String(reference_id.to_string());
    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"));
    let outcome = executor
        .execute_step("direct_mesh_solve", 1, &payload)
        .expect("referenced direct mesh request should succeed");

    handle.join().expect("server thread should finish");
    assert_eq!(outcome.result["job_id"], "referenced-job");
}

#[test]
fn solve_and_wait_from_model_version_runs_native_service_chain() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let port = listener.local_addr().expect("local addr").port();
    let handle = std::thread::spawn(move || {
        let responses = [
            (
                "GET /api/v1/model-versions/ver_native HTTP/1.1",
                r#"{"version":{"version_id":"ver_native","kind":"heat_bar_1d","project_id":"project-native","payload":{"nodes":[{"id":"n0"}],"elements":[{"id":"e0"}]}}}"#,
            ),
            (
                "POST /api/v1/fem/heat-bar-1d/jobs HTTP/1.1",
                r#"{"job":{"job_id":"job-native","status":"queued","progress":0.0}}"#,
            ),
            (
                "GET /api/v1/jobs/job-native HTTP/1.1",
                r#"{"job":{"job_id":"job-native","status":"completed","progress":1.0},"result":{"field":"ready"}}"#,
            ),
            (
                "GET /api/v1/jobs/job-native HTTP/1.1",
                r#"{"job":{"job_id":"job-native","status":"completed","progress":1.0},"result":{"field":"ready"}}"#,
            ),
        ];
        for (expected, body) in responses {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut buffer = [0_u8; 8192];
            let bytes_read = stream.read(&mut buffer).expect("read request");
            let request = String::from_utf8_lossy(&buffer[..bytes_read]);
            assert!(request.starts_with(expected), "request: {request}");
            write_test_response(&mut stream, "200 OK", body);
        }
    });

    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"));
    let outcome = executor
        .execute_step(
            "solve_and_wait_from_model_version",
            1,
            &json!({
                "model_version_id": "ver_native",
                "endpoints": ["127.0.0.1:7001"],
                "interval_ms": 1,
                "timeout_ms": 1000
            }),
        )
        .expect("model-version solve chain should succeed");

    handle.join().expect("server thread should finish");
    assert_eq!(outcome.result["job_id"], "job-native");
    assert_eq!(outcome.result["status"], "completed");
    assert_eq!(outcome.result["model_version_id"], "ver_native");
    assert!(outcome.result["endpoint"].is_null());
    assert_eq!(outcome.result["solve"]["model_version_id"], "ver_native");
    assert_eq!(outcome.result["result"]["result"]["field"], "ready");
}

fn write_test_response(stream: &mut impl Write, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .expect("write response");
}

fn read_complete_http_request(stream: &mut impl Read) -> Vec<u8> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 65_536];
    let mut expected_len = None;
    loop {
        let bytes_read = stream.read(&mut buffer).expect("read HTTP request");
        assert!(bytes_read > 0, "request ended before declared body length");
        request.extend_from_slice(&buffer[..bytes_read]);
        if expected_len.is_none() {
            if let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&request[..header_end]);
                let content_length = head
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(0);
                expected_len = Some(header_end + 4 + content_length);
            }
        }
        if expected_len.is_some_and(|length| request.len() >= length) {
            return request;
        }
    }
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
fn project_create_round_trips_against_local_http_server() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let port = listener.local_addr().expect("local addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut buffer = [0_u8; 4096];
        let bytes_read = stream.read(&mut buffer).expect("read request");
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        assert!(request.starts_with("POST /api/v1/projects HTTP/1.1\r\n"));
        assert!(request.contains("\"name\":\"Native project\""));
        assert!(!request.contains("\"project_id\""));
        let body = r#"{"project":{"project_id":"project-native","name":"Native project"}}"#;
        let response = format!(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
            "project_create",
            1,
            &json!({
                "project_id": "must-not-leak",
                "name": "Native project",
                "description": "created by Rust"
            }),
        )
        .expect("project create request should succeed");

    handle.join().expect("server thread should finish");
    assert_eq!(outcome.status, "executed");
    assert_eq!(outcome.result["project_id"], "project-native");
    assert_eq!(outcome.result["project"]["project_id"], "project-native");
}

#[test]
fn service_health_exposes_discovered_solver_endpoints_for_bindings() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let port = listener.local_addr().expect("local addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut buffer = [0_u8; 4096];
        let bytes_read = stream.read(&mut buffer).expect("read request");
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        assert!(request.starts_with("GET /api/health HTTP/1.1\r\n"));
        write_test_response(
            &mut stream,
            "200 OK",
            r#"{"service":"kyuubiki-orchestrator","status":"ok","solver_agents":[{"host":"127.0.0.1","port":5001},{"host":"::1","port":5002}]}"#,
        );
    });

    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"));
    let outcome = executor
        .execute_step("service_health", 1, &json!({}))
        .expect("service health should succeed");

    handle.join().expect("server thread should finish");
    assert_eq!(
        outcome.result["solver_endpoints"],
        json!(["127.0.0.1:5001", "[::1]:5002"])
    );
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
fn non_success_plain_text_response_preserves_http_root_cause() {
    let response = "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nnot found";
    let error = parse_json_response(response, "/api/v1/fem/composite-thermo-electric-panel/jobs")
        .expect_err("404 should be an error");

    assert!(error.message.contains("404"));
    assert!(error.message.contains("not found"));
    assert!(
        error
            .message
            .contains("service action endpoint not deployed")
    );
    assert!(
        error
            .message
            .contains("/api/v1/fem/composite-thermo-electric-panel/jobs")
    );
    assert!(!error.message.contains("failed to parse JSON"));
}

#[test]
fn non_success_response_promotes_operator_task_error_code() {
    let response = "HTTP/1.1 422 Unprocessable Entity\r\nContent-Type: application/json\r\n\r\n{\"error\":\"{:operator_task_mirror_mismatch, %{}}\",\"error_code\":\"operator_task_mirror_mismatch\"}";
    let error = parse_json_response(response, "/api/v1/operator-tasks/prepare")
        .expect_err("422 should be an error");

    assert!(error.message.contains("operator_task_mirror_mismatch"));
    assert!(error.message.contains("/api/v1/operator-tasks/prepare"));
}
