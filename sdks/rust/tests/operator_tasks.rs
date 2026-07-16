use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn control_plane_executes_operator_task_batch() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let request = read_http_request(&mut stream);
        let mut parts = request.split("\r\n\r\n");
        let headers = parts.next().unwrap_or_default();
        let body = parts.next().unwrap_or_default();
        let path = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/");

        assert_eq!(path, "/api/v1/operator-tasks/execute-batch");
        let payload: serde_json::Value = serde_json::from_str(body).expect("request json");
        assert_eq!(
            payload["batch"]["quality_execution_batch_contract"].as_str(),
            Some("kyuubiki.quality_execution_batch/v1")
        );
        assert_eq!(
            payload["batch"]["tasks"][0]["case_id"].as_str(),
            Some("case-a")
        );

        let response_body = serde_json::json!({
            "status": "executed",
            "operator_task_batch_execution_contract": "kyuubiki.operator_task_batch_execution/v1",
            "task_count": 1,
            "ok_count": 1,
            "error_count": 0,
            "error_codes": [],
            "error_code_counts": {},
            "results": [
                {
                    "case_id": "case-a",
                    "task_id": "task-a",
                    "status": "ok",
                    "result": {"material_thermal_shock_status": "pass"}
                }
            ]
        })
        .to_string();

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response_body}",
            response_body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let client = kyuubiki_headless_sdk::ControlPlaneClient::new(&format!("http://{}", addr))
        .expect("control plane client");
    let batch = serde_json::json!({
        "quality_execution_batch_contract": "kyuubiki.quality_execution_batch/v1",
        "tasks": [
            {
                "case_id": "case-a",
                "task_ir": {
                    "schema_version": "kyuubiki.operator-task-ir/v1",
                    "task_id": "task-a"
                }
            }
        ]
    });

    let result = client
        .execute_operator_task_batch(&batch)
        .expect("execute task batch");

    assert_eq!(result["status"].as_str(), Some("executed"));
    assert_eq!(result["ok_count"].as_u64(), Some(1));
    assert_eq!(result["error_codes"].as_array().map(Vec::len), Some(0));
    assert_eq!(
        result["error_code_counts"]
            .as_object()
            .map(|counts| counts.len()),
        Some(0)
    );
    assert_eq!(
        result["results"][0]["result"]["material_thermal_shock_status"].as_str(),
        Some("pass")
    );

    server.join().expect("server thread");
}

#[test]
fn control_plane_prepares_operator_task_batch() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let request = read_http_request(&mut stream);
        let mut parts = request.split("\r\n\r\n");
        let headers = parts.next().unwrap_or_default();
        let body = parts.next().unwrap_or_default();
        let path = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/");

        assert_eq!(path, "/api/v1/operator-tasks/prepare-batch");
        let payload: serde_json::Value = serde_json::from_str(body).expect("request json");
        assert_eq!(
            payload["batch"]["quality_execution_batch_contract"].as_str(),
            Some("kyuubiki.quality_execution_batch/v1")
        );

        let response_body = serde_json::json!({
            "status": "verified",
            "operator_task_batch_preparation_contract": "kyuubiki.operator_task_batch_preparation/v1",
            "task_count": 1,
            "verified_count": 1,
            "error_count": 0,
            "error_codes": [],
            "error_code_counts": {},
            "summaries": [{"case_id": "case-a", "status": "verified"}]
        })
        .to_string();

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response_body}",
            response_body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let client = kyuubiki_headless_sdk::ControlPlaneClient::new(&format!("http://{}", addr))
        .expect("control plane client");
    let batch = serde_json::json!({
        "quality_execution_batch_contract": "kyuubiki.quality_execution_batch/v1",
        "tasks": [{"case_id": "case-a", "task_ir": {"task_id": "task-a"}}]
    });

    let result = client
        .prepare_operator_task_batch(&batch)
        .expect("prepare task batch");

    assert_eq!(result["status"].as_str(), Some("verified"));
    assert_eq!(result["verified_count"].as_u64(), Some(1));
    assert_eq!(result["error_codes"].as_array().map(Vec::len), Some(0));
    assert_eq!(
        result["error_code_counts"]
            .as_object()
            .map(|counts| counts.len()),
        Some(0)
    );
    assert_eq!(result["summaries"][0]["case_id"].as_str(), Some("case-a"));

    server.join().expect("server thread");
}

#[test]
fn control_plane_checkpoints_operator_task_batch() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let request = read_http_request(&mut stream);
        let mut parts = request.split("\r\n\r\n");
        let headers = parts.next().unwrap_or_default();
        let body = parts.next().unwrap_or_default();
        let path = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/");

        assert_eq!(path, "/api/v1/operator-tasks/checkpoint-batch");
        let payload: serde_json::Value = serde_json::from_str(body).expect("request json");
        assert_eq!(
            payload["batch"]["quality_execution_batch_contract"].as_str(),
            Some("kyuubiki.quality_execution_batch/v1")
        );
        assert_eq!(
            payload["preparation"]["run_id"].as_str(),
            Some("prepare-run")
        );

        let response_body = serde_json::json!({
            "operator_task_batch_checkpoint_contract": "kyuubiki.operator_task_batch_checkpoint/v1",
            "batch_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "checkpoint_digest": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "resume_policy": {"status": "prepared", "next_action": "execute"},
            "case_index": [{"case_id": "case-a"}]
        })
        .to_string();

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response_body}",
            response_body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let client = kyuubiki_headless_sdk::ControlPlaneClient::new(&format!("http://{}", addr))
        .expect("control plane client");
    let batch = serde_json::json!({
        "quality_execution_batch_contract": "kyuubiki.quality_execution_batch/v1",
        "tasks": [{"case_id": "case-a", "task_ir": {"task_id": "task-a"}}]
    });
    let preparation = serde_json::json!({"run_id": "prepare-run", "batch_digest": "a".repeat(64)});

    let result = client
        .checkpoint_operator_task_batch(&batch, Some(&preparation), None)
        .expect("checkpoint task batch");

    assert_eq!(
        result["operator_task_batch_checkpoint_contract"].as_str(),
        Some("kyuubiki.operator_task_batch_checkpoint/v1")
    );
    assert_eq!(
        result["resume_policy"]["next_action"].as_str(),
        Some("execute")
    );

    server.join().expect("server thread");
}

#[test]
fn control_plane_verifies_operator_task_batch_checkpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let request = read_http_request(&mut stream);
        let mut parts = request.split("\r\n\r\n");
        let headers = parts.next().unwrap_or_default();
        let body = parts.next().unwrap_or_default();
        let path = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/");

        assert_eq!(path, "/api/v1/operator-tasks/verify-checkpoint-batch");
        let payload: serde_json::Value = serde_json::from_str(body).expect("request json");
        assert_eq!(
            payload["checkpoint"]["operator_task_batch_checkpoint_contract"].as_str(),
            Some("kyuubiki.operator_task_batch_checkpoint/v1")
        );

        let response_body = serde_json::json!({
            "operator_task_batch_checkpoint_verification_contract": "kyuubiki.operator_task_batch_checkpoint_verification/v1",
            "status": "verified",
            "batch_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "checkpoint_digest": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "resume_policy": {"status": "prepared", "next_action": "execute"}
        })
        .to_string();

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response_body}",
            response_body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let client = kyuubiki_headless_sdk::ControlPlaneClient::new(&format!("http://{}", addr))
        .expect("control plane client");
    let batch = serde_json::json!({
        "quality_execution_batch_contract": "kyuubiki.quality_execution_batch/v1",
        "tasks": [{"case_id": "case-a", "task_ir": {"task_id": "task-a"}}]
    });
    let checkpoint = serde_json::json!({
        "operator_task_batch_checkpoint_contract": "kyuubiki.operator_task_batch_checkpoint/v1",
        "batch_digest": "a".repeat(64),
        "checkpoint_digest": "b".repeat(64)
    });

    let result = client
        .verify_operator_task_batch_checkpoint(&batch, &checkpoint)
        .expect("verify task batch checkpoint");

    assert_eq!(result["status"].as_str(), Some("verified"));
    assert_eq!(
        result["operator_task_batch_checkpoint_verification_contract"].as_str(),
        Some("kyuubiki.operator_task_batch_checkpoint_verification/v1")
    );

    server.join().expect("server thread");
}

#[test]
fn control_plane_plans_operator_task_batch_resume() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let request = read_http_request(&mut stream);
        let mut parts = request.split("\r\n\r\n");
        let headers = parts.next().unwrap_or_default();
        let body = parts.next().unwrap_or_default();
        let path = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/");

        assert_eq!(path, "/api/v1/operator-tasks/resume-plan-batch");
        let payload: serde_json::Value = serde_json::from_str(body).expect("request json");
        assert_eq!(
            payload["checkpoint"]["operator_task_batch_checkpoint_contract"].as_str(),
            Some("kyuubiki.operator_task_batch_checkpoint/v1")
        );

        let response_body = serde_json::json!({
            "operator_task_batch_resume_plan_contract": "kyuubiki.operator_task_batch_resume_plan/v1",
            "next_action": "execute",
            "target_case_ids": ["case-a"],
            "blocked_case_ids": []
        })
        .to_string();

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response_body}",
            response_body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let client = kyuubiki_headless_sdk::ControlPlaneClient::new(&format!("http://{}", addr))
        .expect("control plane client");
    let batch = serde_json::json!({
        "quality_execution_batch_contract": "kyuubiki.quality_execution_batch/v1",
        "tasks": [{"case_id": "case-a", "task_ir": {"task_id": "task-a"}}]
    });
    let checkpoint = serde_json::json!({
        "operator_task_batch_checkpoint_contract": "kyuubiki.operator_task_batch_checkpoint/v1",
        "batch_digest": "a".repeat(64),
        "checkpoint_digest": "b".repeat(64)
    });

    let result = client
        .plan_operator_task_batch_resume(&batch, &checkpoint)
        .expect("plan task batch resume");

    assert_eq!(
        result["operator_task_batch_resume_plan_contract"].as_str(),
        Some("kyuubiki.operator_task_batch_resume_plan/v1")
    );
    assert_eq!(result["target_case_ids"][0].as_str(), Some("case-a"));

    server.join().expect("server thread");
}

#[test]
fn extracts_operator_task_recovery_summary() {
    let payload = serde_json::json!({
        "status": "failed",
        "results": [
            {
                "case_id": "case-a",
                "failure_receipt": {
                    "schema_version": "kyuubiki.headless-operator-task-failure/v1",
                    "failure_stage": "summarize_execution_program",
                    "recovery": {"required_action": "fix_task_ir_contract_mirror_fields"}
                }
            },
            {
                "case_id": "case-b",
                "error": {
                    "details": {
                        "operator_task_failure_receipt": {
                            "schema_version": "kyuubiki.agent-operator-task-failure/v1",
                            "failure_stage": "verify_digest",
                            "recovery": {
                                "required_action": "rebuild_task_ir_and_recompute_digest"
                            }
                        }
                    }
                }
            },
            {
                "case_id": "case-c",
                "failure_receipt": {
                    "schema_version": "kyuubiki.control-plane-operator-task-failure/v1",
                    "failure_stage": "validate_batch_entry",
                    "recovery": {"required_action": "fix_quality_execution_batch_entry"}
                }
            }
        ],
        "resume_plan": {
            "next_action": "retry_failed_cases",
            "target_case_ids": ["case-a", "case-c"],
            "blocked_case_ids": ["case-b"],
            "recovery_actions": [
                "fix_quality_execution_batch_entry",
                "inspect_operator_task_batch_checkpoint"
            ]
        }
    });

    let receipts = kyuubiki_headless_sdk::operator_task_failure_receipts(&payload);
    assert_eq!(receipts.len(), 3);
    assert_eq!(
        kyuubiki_headless_sdk::operator_task_failure_actions(&payload),
        vec![
            "fix_task_ir_contract_mirror_fields",
            "rebuild_task_ir_and_recompute_digest",
            "fix_quality_execution_batch_entry",
            "inspect_operator_task_batch_checkpoint"
        ]
    );

    let summary = kyuubiki_headless_sdk::operator_task_recovery_summary(&payload);
    assert_eq!(summary["next_action"].as_str(), Some("retry_failed_cases"));
    assert_eq!(summary["target_case_ids"][0].as_str(), Some("case-a"));
    assert_eq!(summary["blocked_case_ids"][0].as_str(), Some("case-b"));
    assert_eq!(summary["failure_receipt_count"].as_u64(), Some(3));
    assert_eq!(
        summary["failure_receipts"].as_array().map(Vec::len),
        Some(3)
    );
}

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut buf = [0_u8; 4096];
    let mut request = Vec::new();

    loop {
        let read = stream.read(&mut buf).expect("read request");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buf[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            let text = String::from_utf8_lossy(&request).to_string();
            let content_length = text
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length: "))
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let header_len = text.find("\r\n\r\n").map(|index| index + 4).unwrap_or(0);
            if request.len() >= header_len + content_length {
                break;
            }
        }
    }

    String::from_utf8_lossy(&request).to_string()
}
