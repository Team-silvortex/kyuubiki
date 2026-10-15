use super::*;
#[path = "direct_mesh_owner_tests.rs"]
mod owner_tests;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn gateway(endpoints: Vec<String>) -> DirectMeshGateway {
    DirectMeshGateway {
        config: Arc::new(GatewayConfig {
            enabled: true,
            token: Some("test-token".into()),
            configured_endpoints: endpoints,
            allow_request_endpoints: false,
        }),
        results: Arc::new(Mutex::new(VecDeque::new())),
    }
}

fn request(path: &str, body: Value) -> HttpRequest {
    HttpRequest {
        method: "POST".into(),
        target: path.into(),
        path: path.into(),
        headers: [("authorization".into(), "Bearer test-token".into())].into(),
        body: serde_json::to_vec(&body).unwrap(),
    }
}

fn input() -> Value {
    json!({"execution_id":"a".repeat(32), "study_kind":"axial_bar_1d", "input":{"elements":2}})
}

fn report() -> Value {
    json!({"schema_version":"kyuubiki.agent-durable-execution/v1",
        "execution_id":"a".repeat(32), "automatic_replay_authorized":false,
        "status":"outcome_unknown", "request_id":"original", "process_instance_id":"process",
        "request_digest":"b".repeat(64), "method":"solve_bar_1d", "resumable":false,
        "response":null})
}

#[test]
fn discovery_exposes_transport_limits_even_when_the_agent_is_unavailable() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    drop(listener);
    let response = gateway(vec![endpoint]).handle(&request("/api/direct-mesh/agents", json!({})));
    assert_eq!(response.status, 200);
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        body["transport_policy"],
        crate::direct_mesh_rpc::transport_policy()
    );
    assert_eq!(
        body["transport_policy"]["control_request_timeout_ms"],
        15_000
    );
    assert_eq!(body["transport_policy"]["solver_total_deadline"], false);
    assert_eq!(body["transport_policy"]["automatic_retry"], false);
    assert_eq!(
        body["transport_policy"]["error_diagnostics"]["max_agent_code_bytes"],
        128
    );
    assert_eq!(
        body["transport_policy"]["error_diagnostics"]["agent_details_forwarded"],
        false
    );
    assert_eq!(
        body["selection_policy"],
        crate::direct_mesh_rpc::selection_policy()
    );
    assert!(body["agents"][0]["descriptor_error"].is_string());
}

#[test]
fn busy_or_draining_owner_still_allows_pinned_durable_recovery() {
    for draining in [false, true] {
        let mut descriptor =
            serde_json::to_value(kyuubiki_protocol::AgentDescriptor::solver_agent_default())
                .unwrap();
        descriptor["execution_admission"] = json!({
            "schema_version":"kyuubiki.agent-execution-admission/v1", "configured":true,
            "max_active_executions":1, "active_execution_count":if draining {0} else {1}
        });
        if draining {
            descriptor["lifecycle"]["state"] = json!("quiescent");
            descriptor["lifecycle"]["accepting_new_work"] = json!(false);
            descriptor["lifecycle"]["quiescent"] = json!(true);
            descriptor["lifecycle"]["safe_to_replace"] = json!(true);
            descriptor["lifecycle"]["drain_generation"] = json!(1);
            descriptor["lifecycle"]["drain_owner_id"] = json!("controller");
            descriptor["lifecycle"]["drain_reason"] = json!("replacement");
            descriptor["lifecycle"]["drain_started_unix_ms"] = json!(1);
        } else {
            descriptor["lifecycle"]["active_execution_count"] = json!(1);
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap().to_string();
        listener.set_nonblocking(true).unwrap();
        let peer = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut methods = Vec::new();
            while methods.len() < 3 {
                let (mut stream, _) = match listener.accept() {
                    Ok(socket) => socket,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "missing recovery request: {methods:?}"
                        );
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("recovery peer accept: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut header = [0; 4];
                stream.read_exact(&mut header).unwrap();
                let length = u32::from_be_bytes(header) as usize;
                assert!(length <= 4096);
                let mut bytes = vec![0; length];
                stream.read_exact(&mut bytes).unwrap();
                let request: Value = serde_json::from_slice(&bytes).unwrap();
                let method = request["method"].as_str().unwrap();
                methods.push(method.to_owned());
                let result = match method {
                    "describe_agent" => descriptor.clone(),
                    "fetch_durable_agent_execution" => {
                        assert_eq!(request["params"]["expected_store_id"], "c".repeat(32));
                        assert_eq!(request["params"]["execution_id"], "a".repeat(32));
                        let mut result = report();
                        result["store_id"] = json!("c".repeat(32));
                        result
                    }
                    other => panic!("owner must not receive a new task: {other}"),
                };
                let bytes = serde_json::to_vec(&json!({
                    "rpc_version":1, "id":request["id"], "ok":true, "result":result
                }))
                .unwrap();
                stream
                    .write_all(&(bytes.len() as u32).to_be_bytes())
                    .unwrap();
                stream.write_all(&bytes).unwrap();
            }
            methods
        });
        let gateway = gateway(vec![endpoint.clone()]);
        let discovered = gateway.handle(&request("/api/direct-mesh/agents", json!({})));
        assert_eq!(discovered.status, 200);
        let rejected = gateway.handle(&request("/api/direct-mesh/solve", input()));
        assert_eq!(rejected.status, 502);
        let rejected: Value = serde_json::from_slice(&rejected.body).unwrap();
        assert!(rejected["error"].as_str().unwrap().contains(if draining {
            "lifecycle_not_accepting"
        } else {
            "execution_at_capacity"
        }));
        assert_eq!(rejected["automatic_replay_authorized"], false);
        let response = gateway.handle(&request(
            "/api/direct-mesh/tasks/inspect",
            json!({
                "execution_id":"a".repeat(32), "expected_store_id":"c".repeat(32)
            }),
        ));
        let methods = peer.join().unwrap();
        assert_eq!(
            methods,
            [
                "describe_agent",
                "describe_agent",
                "fetch_durable_agent_execution"
            ]
        );
        assert_eq!(response.status, 200);
        let body: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["endpoint"], endpoint);
        assert_eq!(body["execution"]["status"], "outcome_unknown");
        assert_eq!(body["execution"]["automatic_replay_authorized"], false);
        assert!(gateway.results.lock().unwrap().is_empty());
    }
}

#[test]
fn durable_gateway_requires_authorization_stable_identity_and_one_pinned_endpoint() {
    let gateway = gateway(vec!["127.0.0.1:1".into(), "127.0.0.1:2".into()]);
    assert_eq!(
        gateway
            .handle(&request("/api/direct-mesh/tasks", input()))
            .status,
        400
    );
    let mut body = input();
    body["endpoints"] = json!(["127.0.0.1:3"]);
    assert_eq!(
        gateway
            .handle(&request("/api/direct-mesh/tasks", body))
            .status,
        400
    );
    let mut unauthorized = request("/api/direct-mesh/tasks/inspect", input());
    unauthorized.headers.clear();
    assert_eq!(gateway.handle(&unauthorized).status, 401);
    let mut invalid = input();
    invalid["execution_id"] = json!("../arbitrary");
    assert_eq!(
        gateway
            .handle(&request("/api/direct-mesh/tasks", invalid))
            .status,
        400
    );
}

#[test]
fn durable_submit_and_inspection_bypass_the_gateway_result_cache() {
    for (path, method) in [
        ("/api/direct-mesh/tasks", "submit_durable_agent_task"),
        (
            "/api/direct-mesh/tasks/inspect",
            "fetch_durable_agent_execution",
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap().to_string();
        let peer = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut header = [0; 4];
            stream.read_exact(&mut header).unwrap();
            let mut payload = vec![0; u32::from_be_bytes(header) as usize];
            stream.read_exact(&mut payload).unwrap();
            let request: Value = serde_json::from_slice(&payload).unwrap();
            assert_eq!(request["method"], method);
            assert_eq!(request["params"]["execution_id"], "a".repeat(32));
            if method == "submit_durable_agent_task" {
                assert_eq!(request["params"]["method"], "solve_bar_1d");
                assert_eq!(request["params"]["params"]["elements"], 2);
            }
            let response =
                serde_json::to_vec(&json!({"rpc_version":1, "id":request["id"], "ok":true,
                "result":report()}))
                .unwrap();
            stream
                .write_all(&(response.len() as u32).to_be_bytes())
                .unwrap();
            stream.write_all(&response).unwrap();
        });
        let gateway = gateway(vec![endpoint.clone()]);
        let response = gateway.handle(&request(path, input()));
        peer.join().unwrap();
        assert_eq!(response.status, 200);
        let body: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["endpoint"], endpoint);
        assert_eq!(body["execution"]["status"], "outcome_unknown");
        assert!(gateway.results.lock().unwrap().is_empty());
    }
}

#[test]
fn lost_agent_response_preserves_the_original_recovery_handle() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut header = [0; 4];
        stream.read_exact(&mut header).unwrap();
        let mut payload = vec![0; u32::from_be_bytes(header) as usize];
        stream.read_exact(&mut payload).unwrap();
        // The request was delivered but no terminal response is returned.
    });
    let response =
        gateway(vec![endpoint.clone()]).handle(&request("/api/direct-mesh/tasks", input()));
    peer.join().unwrap();
    assert_eq!(response.status, 502);
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body["execution_id"], "a".repeat(32));
    assert_eq!(body["endpoint"], endpoint);
    assert_eq!(body["automatic_replay_authorized"], false);
}

#[test]
fn durable_reports_bind_identity_schema_and_terminal_evidence() {
    let id = "a".repeat(32);
    let report = report();
    assert!(validate_durable_report(&report, &id).is_ok());
    for (field, value) in [
        ("execution_id", json!("b".repeat(32))),
        ("schema_version", json!("unknown")),
        ("status", json!("completed")),
        ("automatic_replay_authorized", json!(true)),
        ("request_digest", json!("not-a-digest")),
        ("request_id", json!(null)),
        ("resumable", json!(true)),
    ] {
        let mut invalid = report.clone();
        invalid[field] = value;
        assert!(validate_durable_report(&invalid, &id).is_err());
    }
    let mut completed = report;
    completed["status"] = json!("completed");
    completed["response"] = json!({"rpc_version":1, "id":"original", "ok":true,
        "result":{"metric":1.0000000000000002, "tiny":1.2345678901234567e-280}, "error":null});
    assert!(validate_durable_report(&completed, &id).is_ok());
    let wire = serde_json::to_vec(&completed).unwrap();
    let decoded: Value = serde_json::from_slice(&wire).unwrap();
    assert_eq!(completed, decoded);
    completed["response"]["id"] = json!("foreign-receipt");
    assert!(validate_durable_report(&completed, &id).is_err());
    completed["response"]["id"] = json!("original");
    completed["response"]["result"] = Value::Null;
    assert!(validate_durable_report(&completed, &id).is_err());
}

#[test]
fn durable_gateway_rejects_blank_failure_evidence_without_retry() {
    for field in ["code", "message"] {
        for blank in ["", " ", "\t\r\n", "\u{2003}"] {
            let mut failed = report();
            failed["status"] = json!("failed");
            failed["response"] = json!({"rpc_version":1, "id":"original", "ok":false,
                "error":{"code":"solve_failed", "message":"solver failed"}});
            failed["response"]["error"][field] = json!(blank);
            let response = recovery_exchange(
                "/api/direct-mesh/tasks/inspect",
                "fetch_durable_agent_execution",
                json!({"execution_id":"a".repeat(32)}),
                failed,
            );
            assert_eq!(response.status, 502, "{field}: {blank:?}");
            let body: Value = serde_json::from_slice(&response.body).unwrap();
            assert_eq!(body["automatic_replay_authorized"], false);
            assert_eq!(body["execution_id"], "a".repeat(32));
        }
    }
}

fn recovery_exchange(
    path: &str,
    method: &'static str,
    params: Value,
    report: Value,
) -> HttpResponse {
    recovery_rpc_exchange(path, method, params, json!({"ok":true, "result":report}))
}

fn recovery_rpc_exchange(
    path: &str,
    method: &'static str,
    params: Value,
    mut response: Value,
) -> HttpResponse {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let expected = params.clone();
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut header = [0; 4];
        stream.read_exact(&mut header).unwrap();
        let mut bytes = vec![0; u32::from_be_bytes(header) as usize];
        stream.read_exact(&mut bytes).unwrap();
        let request: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(request["method"], method);
        assert_eq!(request["params"], expected);
        response["rpc_version"] = json!(1);
        response["id"] = request["id"].clone();
        let response = serde_json::to_vec(&response).unwrap();
        stream
            .write_all(&(response.len() as u32).to_be_bytes())
            .unwrap();
        stream.write_all(&response).unwrap();
    });
    let gateway = gateway(vec![endpoint]);
    let response = gateway.handle(&request(path, params));
    peer.join().unwrap();
    assert!(gateway.results.lock().unwrap().is_empty());
    response
}

#[test]
fn busy_recovery_is_not_an_empty_index_missing_task_or_successful_cancel() {
    let identity = json!({"execution_id":"a".repeat(32), "expected_store_id":"c".repeat(32)});
    let mut cancellation = identity.clone();
    cancellation["process_instance_id"] = json!("process");
    cancellation["request_digest"] = json!("b".repeat(64));
    for (path, method, params) in [
        (
            "/api/direct-mesh/tasks/inspect",
            "fetch_durable_agent_execution",
            identity,
        ),
        (
            "/api/direct-mesh/tasks/list",
            "list_durable_agent_executions",
            json!({"limit":24, "after":null, "expected_store_id":"c".repeat(32)}),
        ),
        (
            "/api/direct-mesh/tasks/cancel",
            "cancel_durable_agent_execution",
            cancellation,
        ),
    ] {
        let response = recovery_rpc_exchange(
            path,
            method,
            params,
            json!({"ok":false,
            "error":{"code":"durable_execution_busy", "message":"execution store is busy",
                "details":{"operation_started":false,"automatic_replay_authorized":false}}}),
        );
        assert_eq!(response.status, 502, "{path}");
        let body: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["error"], "execution store is busy", "{path}");
        assert_eq!(body["agent_error_code"], "durable_execution_busy", "{path}");
        assert_eq!(body["automatic_retry"], false);
        assert_eq!(body["automatic_replay_authorized"], false);
        assert!(body.get("execution").is_none());
        assert!(body.get("executions").is_none());
        assert!(body.get("cancellation").is_none());
    }
}

#[test]
fn remote_error_diagnostics_cannot_forward_replay_advice_or_become_terminal_evidence() {
    for code in [
        "store_unavailable".to_owned(),
        "x".repeat(129),
        "retry\nnow".into(),
    ] {
        let response = recovery_rpc_exchange(
            "/api/direct-mesh/tasks/inspect",
            "fetch_durable_agent_execution",
            json!({"execution_id":"a".repeat(32), "expected_store_id":"c".repeat(32)}),
            json!({"ok":false,"error":{"code":code,"message":"failure",
                "details":{"automatic_replay_authorized":true,"automatic_retry":true,
                    "execution_started":false,"required_action":"submit_to_another_agent"}}}),
        );
        assert_eq!(response.status, 502);
        let body: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(
            body["agent_error_code"],
            if code == "store_unavailable" {
                json!(code)
            } else {
                Value::Null
            }
        );
        assert_eq!(body["automatic_retry"], false);
        assert_eq!(body["automatic_replay_authorized"], false);
        assert_eq!(
            body["required_action"],
            "inspect_execution_on_original_agent"
        );
        assert!(body.get("details").is_none());
        assert!(body.get("execution_started").is_none());
        assert!(body.get("execution").is_none());
    }
}

#[test]
fn invalid_remote_envelopes_and_transport_failures_have_no_agent_error_code() {
    for reply in [
        json!({"ok":false, "result":{"status":"not_found"},
            "error":{"code":"durable_execution_busy","message":"busy"}}),
        json!({"ok":true, "result":{"agent_error_code":"durable_execution_busy"}}),
    ] {
        let response = recovery_rpc_exchange(
            "/api/direct-mesh/tasks/inspect",
            "fetch_durable_agent_execution",
            json!({"execution_id":"a".repeat(32)}),
            reply,
        );
        assert_eq!(response.status, 502);
        let body: Value = serde_json::from_slice(&response.body).unwrap();
        assert!(body["agent_error_code"].is_null());
        assert_eq!(body["automatic_retry"], false);
        assert_eq!(body["automatic_replay_authorized"], false);
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    drop(listener);
    let response = gateway(vec![endpoint]).handle(&request(
        "/api/direct-mesh/tasks/inspect",
        json!({"execution_id":"a".repeat(32)}),
    ));
    assert_eq!(response.status, 502);
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    assert!(body["agent_error_code"].is_null());
    assert_eq!(body["automatic_replay_authorized"], false);
}

#[test]
fn durable_recovery_routes_validate_index_and_exact_cancel_receipts() {
    let row = json!({"schema_version":"kyuubiki.agent-durable-execution/v1",
        "execution_id":"a".repeat(32), "request_id":"original", "process_instance_id":"process",
        "request_digest":"b".repeat(64), "method":"solve_bar_1d"});
    let index = json!({"schema_version":"kyuubiki.agent-durable-execution-index/v1",
        "executions":[row], "next_cursor":null, "automatic_replay_authorized":false});
    assert_eq!(
        recovery_exchange(
            "/api/direct-mesh/tasks/list",
            "list_durable_agent_executions",
            json!({"limit":24, "after":null}),
            index.clone()
        )
        .status,
        200
    );
    let mut invalid = index;
    invalid["executions"][0]["response"] = json!({"result": "unexpected large result"});
    assert_eq!(
        recovery_exchange(
            "/api/direct-mesh/tasks/list",
            "list_durable_agent_executions",
            json!({"limit":24, "after":null}),
            invalid
        )
        .status,
        502
    );
    let params = json!({"execution_id":"a".repeat(32),"process_instance_id":"process","request_digest":"b".repeat(64)});
    let cancellation = json!({"schema_version":"kyuubiki.agent-durable-cancellation/v1",
        "execution_id":"a".repeat(32),"process_instance_id":"process","request_digest":"b".repeat(64),
        "status":"requested","cancel_registered":true,"execution_terminal_confirmed":false,"automatic_replay_authorized":false});
    assert_eq!(
        recovery_exchange(
            "/api/direct-mesh/tasks/cancel",
            "cancel_durable_agent_execution",
            params.clone(),
            cancellation.clone()
        )
        .status,
        200
    );
    let mut invalid = cancellation;
    invalid["execution_terminal_confirmed"] = json!(true);
    assert_eq!(
        recovery_exchange(
            "/api/direct-mesh/tasks/cancel",
            "cancel_durable_agent_execution",
            params,
            invalid
        )
        .status,
        502
    );
}

#[test]
fn durable_recovery_rejects_invalid_requests_and_unauthorized_peers_before_connect() {
    let gateway = gateway(vec!["127.0.0.1:1".into()]);
    for (path, body) in [
        ("/api/direct-mesh/tasks/list", json!({"limit":0})),
        ("/api/direct-mesh/tasks/list", json!({"limit":65})),
        (
            "/api/direct-mesh/tasks/list",
            json!({"limit":24,"after":"../foreign"}),
        ),
        (
            "/api/direct-mesh/tasks/list",
            json!({"limit":24,"endpoints":["127.0.0.1:2"]}),
        ),
        (
            "/api/direct-mesh/tasks/cancel",
            json!({"execution_id":"a".repeat(32)}),
        ),
    ] {
        assert_eq!(gateway.handle(&request(path, body)).status, 400);
    }
    for path in [
        "/api/direct-mesh/tasks/list",
        "/api/direct-mesh/tasks/cancel",
    ] {
        let mut req = request(path, json!({"limit":24}));
        req.headers.clear();
        assert_eq!(gateway.handle(&req).status, 401);
    }
}

fn rejection_exchange(
    path: &str,
    change: impl FnOnce(&mut Value) + Send + 'static,
) -> HttpResponse {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut header = [0; 4];
        stream.read_exact(&mut header).unwrap();
        let mut payload = vec![0; u32::from_be_bytes(header) as usize];
        stream.read_exact(&mut payload).unwrap();
        let request: Value = serde_json::from_slice(&payload).unwrap();
        let mut response = json!({"rpc_version":1, "id":request["id"], "ok":true, "result":{
            "schema_version":"kyuubiki.agent-durable-admission-rejection/v1",
            "status":"not_admitted", "execution_id":"a".repeat(32), "method":"solve_bar_1d",
            "request_id":request["id"], "process_instance_id":"producer", "request_digest":"b".repeat(64),
            "reason":"execution_unavailable", "message":"agent_at_capacity", "execution_started":false,
            "automatic_replay_authorized":false
        }});
        change(&mut response);
        let payload = serde_json::to_vec(&response).unwrap();
        stream
            .write_all(&(payload.len() as u32).to_be_bytes())
            .unwrap();
        stream.write_all(&payload).unwrap();
    });
    let gateway = gateway(vec![endpoint]);
    let response = gateway.handle(&request(path, input()));
    peer.join().unwrap();
    assert!(gateway.results.lock().unwrap().is_empty());
    response
}

#[test]
fn rejection_is_bound_to_the_live_submit_exchange_not_an_execution_lookup() {
    let response = rejection_exchange("/api/direct-mesh/tasks", |_| {});
    assert_eq!(response.status, 200);
    let value: Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(value["execution"]["status"], "not_admitted");
    assert_eq!(
        rejection_exchange("/api/direct-mesh/tasks/inspect", |_| {}).status,
        502
    );
}

#[test]
fn malformed_or_unbound_rejection_never_authorizes_caller_cleanup() {
    for (field, value) in [
        ("request_id", json!("foreign-request")),
        ("execution_id", json!("b".repeat(32))),
        ("method", json!("solve_beam_1d")),
        ("schema_version", json!("unknown")),
        ("execution_started", json!(true)),
        ("automatic_replay_authorized", json!(true)),
        ("request_digest", json!("bad")),
        ("process_instance_id", json!("")),
        ("reason", json!("corrupt_store")),
        ("message", json!("")),
        ("response", json!({"ok":true})),
    ] {
        let response = rejection_exchange("/api/direct-mesh/tasks", move |v| {
            v["result"][field] = value
        });
        assert_eq!(response.status, 502, "{field}");
        let body: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["execution_id"], "a".repeat(32));
        assert_eq!(body["automatic_replay_authorized"], false);
    }
    for (field, value) in [
        ("id", json!("other")),
        ("rpc_version", json!(2)),
        ("error", json!({"code":"failed"})),
    ] {
        assert_eq!(
            rejection_exchange("/api/direct-mesh/tasks", move |v| v[field] = value).status,
            502
        );
    }
}
