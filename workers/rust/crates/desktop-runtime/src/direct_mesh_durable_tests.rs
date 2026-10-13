use super::*;
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

fn recovery_exchange(
    path: &str,
    method: &'static str,
    params: Value,
    report: Value,
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
        let response = serde_json::to_vec(
            &json!({"rpc_version":1, "id":request["id"], "ok":true,"result":report}),
        )
        .unwrap();
        stream
            .write_all(&(response.len() as u32).to_be_bytes())
            .unwrap();
        stream.write_all(&response).unwrap();
    });
    let response = gateway(vec![endpoint]).handle(&request(path, params));
    peer.join().unwrap();
    response
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
