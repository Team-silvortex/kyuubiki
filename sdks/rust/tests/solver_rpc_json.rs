#[path = "../examples/durable_modal_research/checks.rs"]
mod checks;
#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::{DurableExecutionIntent, SdkError, SolverRpcClient};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

fn peer(
    payload: impl FnOnce(Value) -> String + Send + 'static,
) -> (SolverRpcClient, thread::JoinHandle<()>) {
    peer_frames(move |request| vec![payload(request)])
}

fn peer_frames(
    payload: impl FnOnce(Value) -> Vec<String> + Send + 'static,
) -> (SolverRpcClient, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let worker = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "test request did not arrive");
                    thread::sleep(Duration::from_millis(1));
                }
                Err(e) => panic!("test accept: {e}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut header = [0; 4];
        stream.read_exact(&mut header).unwrap();
        let length = u32::from_be_bytes(header) as usize;
        assert!(length < 65536);
        let mut bytes = vec![0; length];
        stream.read_exact(&mut bytes).unwrap();
        let request: Value = serde_json::from_slice(&bytes).unwrap();
        let mut frames = Vec::new();
        for response in payload(request) {
            frames.extend_from_slice(&(response.len() as u32).to_be_bytes());
            frames.extend_from_slice(response.as_bytes());
        }
        stream.write_all(&frames).unwrap();
    });
    (SolverRpcClient::new("127.0.0.1", port), worker)
}

fn assert_ambiguous(error: SdkError) {
    assert!(
        matches!(error, SdkError::Json(_)),
        "ambiguous bytes became a decision: {error}"
    );
    assert!(error.to_string().contains("duplicate"), "{error}");
}

#[test]
fn duplicate_nested_result_keys_never_silently_choose_a_value() {
    for payload in [
        r#"{"status":"failed","status":"completed"}"#,
        r#"{"modes":[{"frequency_hz":1,"frequency_hz":2}]}"#,
        r#"{"metadata":{"x":1,"\u0078":1}}"#,
        r#"{"x":null,"x":null}"#,
    ] {
        let (client, worker) = peer(move |request| {
            format!(
                r#"{{"rpc_version":1,"id":{},"ok":true,"result":{payload}}}"#,
                request["id"]
            )
        });
        let result = client.ping();
        worker.join().unwrap();
        assert_ambiguous(result.err().expect("duplicate result was accepted"));
    }
}

#[test]
fn duplicate_execution_flags_cannot_become_known_non_admission() {
    let intent =
        DurableExecutionIntent::new(&"b".repeat(32), &"a".repeat(32), "solve_bar_1d", &json!({}))
            .unwrap();
    let expected = intent.clone();
    let (client, worker) = peer(move |request| {
        let report = json!({
            "schema_version":"kyuubiki.agent-durable-admission-rejection/v1",
            "store_id":expected.store_id, "execution_id":expected.execution_id,
            "request_digest":expected.request_digest, "method":expected.method,
            "request_id":request["id"], "process_instance_id":"process-a",
            "status":"not_admitted", "reason":"store_capacity",
            "message":"execution store admission blocked by capacity",
            "execution_started":false, "automatic_replay_authorized":false
        });
        let bytes =
            json!({"rpc_version":1,"id":request["id"],"ok":true,"result":report}).to_string();
        assert!(bytes.contains(r#""execution_started":false"#));
        bytes.replace(
            r#""execution_started":false"#,
            r#""execution_started":true,"execution_started":false"#,
        )
    });
    let result = client.submit_prepared_durable_agent_task(&intent, json!({}));
    worker.join().unwrap();
    assert_ambiguous(result.unwrap_err());
}

#[test]
fn duplicate_error_details_are_not_retained_as_a_definite_failure() {
    let (client, worker) = peer(|request| {
        format!(
            r#"{{"rpc_version":1,"id":{},"ok":false,"error":{{"code":"failure","message":"failed","details":{{"nested":[{{"stage":"a","stage":"b"}}]}}}}}}"#,
            request["id"]
        )
    });
    let result = client.ping();
    worker.join().unwrap();
    assert_ambiguous(result.err().unwrap());
}

#[test]
fn duplicate_optional_progress_data_never_enters_retained_frames() {
    let (client, worker) = peer_frames(|request| {
        vec![
            format!(
                r#"{{"rpc_version":1,"id":{},"event":"progress","progress":{{"job_id":"job","stage":"solving","progress":0.5,"extra":{{"n":1,"n":2}}}}}}"#,
                request["id"]
            ),
            json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{"value":42}}).to_string(),
        ]
    });
    let result = client.ping();
    worker.join().unwrap();
    assert_ambiguous(result.err().unwrap());
}

const EXACT_PAYLOAD: &str = r#"{"values":[-0.0,0.0,1,1.0,18446744073709551615,-9223372036854775808,1.0000000000000002,5e-324,1.7976931348623157e308],"left":{"n":1},"right":{"n":2},"items":[{"n":3},{"n":4}],"text":"\u6a21\u6001","other":[true,false,null,[],{}]}"#;

fn assert_exact(actual: &Value) {
    let expected: Value = serde_json::from_str(EXACT_PAYLOAD).unwrap();
    let digest = |value| {
        DurableExecutionIntent::new(&"b".repeat(32), &"a".repeat(32), "solve_bar_1d", value)
            .unwrap()
            .request_digest
    };
    assert_eq!(digest(actual), digest(&expected));
    assert_eq!(
        actual["values"][0].as_f64().unwrap().to_bits(),
        (-0.0_f64).to_bits()
    );
    assert_eq!(actual["values"][4].as_u64(), Some(u64::MAX));
    assert_eq!(actual["values"][5].as_i64(), Some(i64::MIN));
    assert_eq!(actual["values"][7].as_f64().unwrap().to_bits(), 1);
}

#[test]
fn unique_rpc_json_preserves_numeric_bits_and_sibling_key_scopes() {
    let (client, worker) = peer(|request| {
        format!(
            r#"{{"rpc_version":1,"id":{},"ok":true,"result":{EXACT_PAYLOAD}}}"#,
            request["id"]
        )
    });
    let result = client.ping();
    worker.join().unwrap();
    assert_exact(&result.unwrap().result);
}

#[test]
fn unique_progress_and_error_diagnostics_preserve_original_numeric_types() {
    let (client, worker) = peer_frames(|request| {
        vec![
            format!(
                r#"{{"rpc_version":1,"id":{},"event":"progress","progress":{{"job_id":"job","stage":"solving","progress":0.5,"extra":{EXACT_PAYLOAD}}}}}"#,
                request["id"]
            ),
            json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{"value":42}}).to_string(),
        ]
    });
    let outcome = client.ping();
    worker.join().unwrap();
    let outcome = outcome.unwrap();
    assert_eq!(outcome.progress_frames.len(), 1);
    assert_exact(&outcome.progress_frames[0]["progress"]["extra"]);
    assert_eq!(outcome.result["value"], 42);
    let (client, worker) = peer(|request| {
        format!(
            r#"{{"rpc_version":1,"id":{},"ok":false,"error":{{"code":"failure","message":"failed","details":{EXACT_PAYLOAD}}}}}"#,
            request["id"]
        )
    });
    let error = client.ping().err().unwrap();
    worker.join().unwrap();
    let SdkError::Rpc {
        details: Some(details),
        ..
    } = error
    else {
        panic!("valid diagnostics were lost: {error}");
    };
    assert_exact(&details);
}

#[test]
fn unique_rpc_json_keeps_parser_depth_and_finite_number_limits() {
    for payload in [
        format!("{}0{}", "[".repeat(130), "]".repeat(130)),
        "[1e400]".into(),
    ] {
        let (client, worker) = peer(move |request| {
            format!(
                r#"{{"rpc_version":1,"id":{},"ok":true,"result":{payload}}}"#,
                request["id"]
            )
        });
        let result = client.ping();
        worker.join().unwrap();
        assert!(matches!(result, Err(SdkError::Json(_))));
    }
}

#[test]
#[ignore = "requires built native Agent; run make test-sdk-modal-recovery"]
fn ambiguous_real_modal_receipt_is_rejected_then_original_recovers_after_restart()
-> support::Result<()> {
    let mut agent = support::Agent::start()?;
    let input = model::model(false);
    let intent = DurableExecutionIntent::new(
        &agent.store_id()?,
        &"a".repeat(32),
        "solve_modal_frame_2d",
        &input,
    )?;
    let admitted = agent
        .client()
        .submit_prepared_durable_agent_task(&intent, input.clone())?;
    let handle = intent.handle_from_report(&admitted)?;
    let original = agent.wait(&handle, "completed")?;
    checks::verify(&input, handle.completed_result(&original)?)?;

    // Fault fixture keeps the real receipt's final values. A last-key-wins decoder
    // would pass even the history digest, while hiding the contradictory raw bytes.
    for (field, wrong) in [
        ("status", json!("failed")),
        ("request_digest", json!("c".repeat(64))),
    ] {
        let expected = original.clone();
        let (client, worker) = peer(move |request| {
            assert_eq!(request["method"], "fetch_durable_agent_execution");
            assert_eq!(request["params"]["execution_id"], expected["execution_id"]);
            let report = serde_json::to_string(&expected).unwrap();
            let needle = format!("\"{field}\":{}", expected[field]);
            assert_eq!(report.matches(&needle).count(), 1);
            let ambiguous = report.replacen(&needle, &format!("\"{field}\":{wrong},{needle}"), 1);
            format!(
                r#"{{"rpc_version":1,"id":{},"ok":true,"result":{ambiguous}}}"#,
                request["id"]
            )
        });
        let outcome = client.fetch_durable_agent_execution_after(&original);
        worker.join().unwrap();
        assert_ambiguous(outcome.unwrap_err());
    }
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&original)?,
        original
    );
    agent.kill()?;
    agent.restart()?;
    let recovered = agent
        .client()
        .fetch_durable_agent_execution_after(&original)?;
    assert_eq!(recovered, original);
    checks::verify(&input, handle.completed_result(&recovered)?)?;
    let state = agent.client().describe_agent()?.result;
    assert_eq!(state["durable_execution"]["stored_entries"], 1);
    assert_eq!(state["lifecycle"]["active_execution_count"], 0);
    Ok(())
}
