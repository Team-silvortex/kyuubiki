use super::*;

#[test]
fn admission_rejection_must_echo_the_requested_store_before_cleanup() {
    let mut report = json!({
        "schema_version":"kyuubiki.agent-durable-admission-rejection/v1",
        "status":"not_admitted", "execution_id":"a".repeat(32), "method":"solve_bar_1d",
        "request_id":"request", "process_instance_id":"producer", "request_digest":"b".repeat(64),
        "reason":"store_capacity", "message":"full", "execution_started":false,
        "automatic_replay_authorized":false, "store_id":"c".repeat(32)
    });
    let params = json!({"execution_id":"a".repeat(32), "method":"solve_bar_1d", "expected_store_id":"c".repeat(32)});
    let check = |report: &Value| {
        crate::direct_mesh_admission::validate_rejection(
            report,
            "submit_durable_agent_task",
            &params,
            "request",
        )
    };
    assert!(check(&report).is_ok());
    report["store_id"] = json!("d".repeat(32));
    assert!(check(&report).is_err());
    report.as_object_mut().unwrap().remove("store_id");
    assert!(check(&report).is_err());
}

#[test]
fn every_gateway_durable_route_forwards_and_checks_the_store_fence() {
    for (route, method) in [
        ("/api/direct-mesh/tasks", "submit_durable_agent_task"),
        (
            "/api/direct-mesh/tasks/inspect",
            "fetch_durable_agent_execution",
        ),
        (
            "/api/direct-mesh/tasks/list",
            "list_durable_agent_executions",
        ),
        (
            "/api/direct-mesh/tasks/cancel",
            "cancel_durable_agent_execution",
        ),
    ] {
        for owner in [Some("c".repeat(32)), Some("d".repeat(32)), None] {
            let valid = owner.as_deref() == Some(&"c".repeat(32));
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = listener.local_addr().unwrap().to_string();
            let peer = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut header = [0; 4];
                stream.read_exact(&mut header).unwrap();
                let mut bytes = vec![0; u32::from_be_bytes(header) as usize];
                stream.read_exact(&mut bytes).unwrap();
                let rpc: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(rpc["method"], method);
                assert_eq!(rpc["params"]["expected_store_id"], "c".repeat(32));
                let mut value = match method {
                    "list_durable_agent_executions" => {
                        json!({"schema_version":"kyuubiki.agent-durable-execution-index/v1",
                        "executions":[], "next_cursor":null, "automatic_replay_authorized":false})
                    }
                    "cancel_durable_agent_execution" => {
                        json!({"schema_version":"kyuubiki.agent-durable-cancellation/v1",
                        "execution_id":"a".repeat(32), "process_instance_id":"process", "request_digest":"b".repeat(64),
                        "status":"not_running", "cancel_registered":false, "execution_terminal_confirmed":false,
                        "automatic_replay_authorized":false})
                    }
                    _ => report(),
                };
                if let Some(owner) = owner {
                    value["store_id"] = json!(owner);
                }
                let reply = serde_json::to_vec(
                    &json!({"rpc_version":1,"id":rpc["id"],"ok":true,"result":value}),
                )
                .unwrap();
                stream
                    .write_all(&(reply.len() as u32).to_be_bytes())
                    .unwrap();
                stream.write_all(&reply).unwrap();
            });
            let mut body = input();
            body["expected_store_id"] = json!("c".repeat(32));
            body["limit"] = json!(24);
            body["process_instance_id"] = json!("process");
            body["request_digest"] = json!("b".repeat(64));
            let response = gateway(vec![endpoint]).handle(&request(route, body));
            peer.join().unwrap();
            assert_eq!(response.status, if valid { 200 } else { 502 }, "{route}");
            if !valid {
                let body: Value = serde_json::from_slice(&response.body).unwrap();
                assert_eq!(body["automatic_replay_authorized"], false);
            }
        }
    }
}

#[test]
fn malformed_owner_fence_is_rejected_without_connecting() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let gateway = gateway(vec![listener.local_addr().unwrap().to_string()]);
    for route in [
        "/api/direct-mesh/tasks",
        "/api/direct-mesh/tasks/inspect",
        "/api/direct-mesh/tasks/list",
        "/api/direct-mesh/tasks/cancel",
    ] {
        for invalid in [
            Value::Null,
            json!(""),
            json!("../owner"),
            json!("A".repeat(32)),
            json!(7),
        ] {
            let mut body = input();
            body["expected_store_id"] = invalid;
            assert_eq!(gateway.handle(&request(route, body)).status, 400);
            assert!(listener.accept().is_err());
        }
    }
}
