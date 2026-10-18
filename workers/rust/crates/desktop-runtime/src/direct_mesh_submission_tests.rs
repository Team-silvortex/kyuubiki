use super::*;

fn exchange(
    path: &str,
    body: Value,
    change: impl FnOnce(&mut Value) + Send + 'static,
) -> (HttpResponse, String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let peer = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "missing pinned request");
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("accept: {error}"),
            }
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
        let size = u32::from_be_bytes(header) as usize;
        assert!(size <= 4096);
        let mut payload = vec![0; size];
        stream.read_exact(&mut payload).unwrap();
        let rpc: Value = serde_json::from_slice(&payload).unwrap();
        assert!(matches!(
            rpc["method"].as_str(),
            Some("submit_durable_agent_task" | "fetch_durable_agent_execution")
        ));
        let mut result = report();
        result["store_id"] = json!("c".repeat(32));
        if rpc["method"] == "submit_durable_agent_task" {
            let task: kyuubiki_protocol::DurableAgentTask =
                serde_json::from_value(rpc["params"].clone()).unwrap();
            result["request_digest"] = json!(task.request_digest().unwrap());
        }
        change(&mut result);
        // A replay of a stored receipt must keep the old execution RPC identity.
        assert_ne!(rpc["id"], result["request_id"]);
        let response = serde_json::to_vec(&json!({
            "rpc_version":1, "id":rpc["id"], "ok":true, "result":result
        }))
        .unwrap();
        stream
            .write_all(&(response.len() as u32).to_be_bytes())
            .unwrap();
        stream.write_all(&response).unwrap();
        listener
    });
    let gateway = gateway(vec![endpoint.clone()]);
    let response = gateway.handle(&request(path, body));
    let listener = peer.join().unwrap();
    assert!(
        matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock),
        "must not retry or rediscover after receiving an unbound receipt"
    );
    assert!(gateway.results.lock().unwrap().is_empty());
    (response, endpoint)
}

fn set_status(report: &mut Value, status: &str) {
    report["status"] = json!(status);
    report["response"] = match status {
        "completed" => json!({"rpc_version":1,"id":"original","ok":true,
            "result":{"displacement":0.125},"error":null}),
        "failed" => json!({"rpc_version":1,"id":"original","ok":false,
            "result":null,"error":{"code":"solver_failed","message":"did not converge"}}),
        _ => Value::Null,
    };
}

fn assert_ambiguous(response: HttpResponse, endpoint: &str) {
    assert_eq!(
        response.status,
        502,
        "{}",
        String::from_utf8_lossy(&response.body)
    );
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body["execution_id"], "a".repeat(32));
    assert_eq!(body["endpoint"], endpoint);
    assert_eq!(body["automatic_replay_authorized"], false);
    assert_eq!(body["automatic_retry"], false);
    assert_eq!(
        body["required_action"],
        "inspect_execution_on_original_agent"
    );
    assert!(body.get("execution").is_none());
}

#[test]
fn submit_rejects_same_id_receipts_for_another_method_or_input() {
    for status in ["running", "outcome_unknown", "completed", "failed"] {
        for (field, value) in [
            ("method", json!("solve_beam_1d")),
            (
                "request_digest",
                json!(task_digest("solve_bar_1d", json!({"elements":3}))),
            ),
        ] {
            let (response, endpoint) = exchange("/api/direct-mesh/tasks", input(), move |v| {
                set_status(v, status);
                v[field] = value;
            });
            assert_ambiguous(response, &endpoint);
        }
    }
}

#[test]
fn submit_rejects_a_well_formed_not_found_lookup_in_place_of_admission() {
    for with_identity in [false, true] {
        let (response, endpoint) = exchange("/api/direct-mesh/tasks", input(), move |v| {
            if with_identity {
                v["status"] = json!("not_found");
            } else {
                *v = json!({"schema_version":"kyuubiki.agent-durable-execution/v1",
                    "execution_id":"a".repeat(32), "store_id":"c".repeat(32),
                    "status":"not_found", "automatic_replay_authorized":false, "response":null});
            }
        });
        assert_ambiguous(response, &endpoint);
    }
}

#[test]
fn submit_digest_distinguishes_adjacent_binary64_inputs() {
    let mut body = input();
    body["input"] = json!({"tip_force":1.0000000000000002, "tiny":1.2345678901234567e-280});
    let mut other = body["input"].clone();
    other["tip_force"] = json!(1.0000000000000004);
    let (response, endpoint) = exchange("/api/direct-mesh/tasks", body, move |v| {
        v["request_digest"] = json!(task_digest("solve_bar_1d", other));
    });
    assert_ambiguous(response, &endpoint);
}

#[test]
fn matching_historical_receipts_remain_valid_without_a_new_transport_identity() {
    for status in ["running", "outcome_unknown", "completed", "failed"] {
        let mut body = input();
        body["expected_store_id"] = json!("c".repeat(32));
        body["input"] = json!({"tip_force":1.0000000000000002, "tiny":1.2345678901234567e-280});
        let expected_digest = task_digest("solve_bar_1d", body["input"].clone());
        let (response, _) = exchange("/api/direct-mesh/tasks", body, move |v| {
            set_status(v, status)
        });
        assert_eq!(response.status, 200);
        let body: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["execution"]["request_id"], "original");
        assert_eq!(body["execution"]["request_digest"], expected_digest);
        assert_eq!(body["execution"]["status"], status);
    }
}

#[test]
fn lookup_does_not_bind_a_stored_receipt_to_the_current_editor_input() {
    let mut body = input();
    body["input"] = json!({"elements":999});
    let (response, _) = exchange("/api/direct-mesh/tasks/inspect", body, |v| {
        set_status(v, "completed")
    });
    assert_eq!(response.status, 200);
}

#[test]
fn valid_hex_but_foreign_rejection_digest_never_authorizes_cleanup() {
    let response = rejection_exchange("/api/direct-mesh/tasks", |v| {
        v["result"]["request_digest"] = json!(task_digest("solve_bar_1d", json!({"elements":3})));
    });
    assert_eq!(response.status, 502);
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body["automatic_replay_authorized"], false);
    assert_eq!(body["automatic_retry"], false);
    assert!(body.get("execution").is_none());
}

#[test]
fn invalid_submission_identity_or_payload_fails_before_connecting() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let valid = json!({"execution_id":"a".repeat(32), "method":"solve_bar_1d",
        "params":{"elements":2}});
    for (field, value) in [
        ("execution_id", Value::Null),
        ("execution_id", json!("A".repeat(32))),
        ("method", json!("unknown_method")),
        ("method", json!("ping")),
        ("method", json!("run_operator_task_ir")),
        ("params", Value::Null),
        ("params", json!([])),
    ] {
        let mut invalid = valid.clone();
        invalid[field] = value;
        assert!(
            crate::direct_mesh_rpc::request_detailed(
                &endpoint,
                "submit_durable_agent_task",
                invalid
            )
            .is_err()
        );
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
}
