use crate::service_executor_result_artifact::{
    READBACK_FAILURE, RESULT_MEDIA_TYPE, ResultReadPolicy,
};
use crate::service_executor_result_download::ResultSpool;
use crate::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessRisk,
    ServiceHeadlessExecutor, execute_batch_with_executor,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

struct Reply {
    parts: Vec<Vec<u8>>,
    delay: Duration,
}

impl Reply {
    fn raw(bytes: Vec<u8>) -> Self {
        Self {
            parts: vec![bytes],
            delay: Duration::ZERO,
        }
    }
    fn json(value: Value) -> Self {
        let body = serde_json::to_vec(&value).unwrap();
        Self::raw(response(
            "200 OK",
            "application/json",
            &format!("Content-Length: {}\r\n", body.len()),
            &body,
        ))
    }
}

struct Server {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: mpsc::Sender<()>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    fn new(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let requests = Arc::new(Mutex::new(vec![]));
        let captured = Arc::clone(&requests);
        let (stop, stopped) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut replies = VecDeque::from(replies);
            while stopped.try_recv().is_err() {
                let (mut stream, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(cause) if cause.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    Err(cause) => panic!("fixture accept: {cause}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                let mut byte = [0; 1];
                while !request.ends_with(b"\r\n\r\n") {
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                    assert!(request.len() <= 64 * 1024);
                }
                let request = String::from_utf8(request).unwrap();
                let length = request
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                assert!(length <= 1024 * 1024);
                stream.read_exact(&mut vec![0; length]).unwrap();
                captured.lock().unwrap().push(request);
                let reply = replies
                    .pop_front()
                    .unwrap_or_else(|| Reply::json(json!({"project":{"project_id":"unexpected"}})));
                for part in reply.parts {
                    thread::sleep(reply.delay);
                    if stream.write_all(&part).is_err() {
                        break;
                    }
                }
            }
        });
        Self {
            url,
            requests,
            stop,
            worker: Some(worker),
        }
    }

    fn fetch(&self, payload: Value) -> Result<Value, crate::HeadlessExecutorError> {
        ServiceHeadlessExecutor::with_token(&self.url, Some("test-only-token"))
            .execute_step("result_fetch", 1, &payload)
            .map(|outcome| outcome.result)
    }

    fn halt(&self, payload: Value, reads: usize) {
        let batch = HeadlessExecutionBatch {
            schema_version: "kyuubiki.headless-execution-batch/v1".into(),
            exported_at: "2026-10-08T00:00:00Z".into(),
            language: "en".into(),
            workflow_id: "result-readback".into(),
            template_id: None,
            warnings: vec![],
            steps: vec![
                HeadlessExecutionBatchStep {
                    index: 1,
                    action: "result_fetch".into(),
                    risk: HeadlessRisk::Normal,
                    payload,
                },
                HeadlessExecutionBatchStep {
                    index: 2,
                    action: "project_create".into(),
                    risk: HeadlessRisk::Normal,
                    payload: json!({"name":"must-not-be-written"}),
                },
            ],
        };
        let report = execute_batch_with_executor(
            &batch,
            &mut ServiceHeadlessExecutor::new(&self.url),
            false,
            false,
        );
        assert_eq!(report.status, "failed", "{report:?}");
        assert_eq!(report.steps.len(), 1);
        assert_eq!(report.executed_step_count, 0);
        let failure = report.execution_summary.failure.unwrap();
        assert_eq!(failure.category, "result_artifact_readback_failed");
        assert_eq!(failure.stage, "result_fetch");
        assert!(!failure.retryable);
        assert_eq!(failure.retry_strategy, "none");
        assert!(failure.message.starts_with(READBACK_FAILURE));
        assert_eq!(self.requests.lock().unwrap().len(), reads);
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        let joined = self.worker.take().unwrap().join();
        if !thread::panicking() {
            joined.unwrap();
        }
    }
}

fn reference(body: &[u8]) -> Value {
    let hash = format!("{:x}", Sha256::digest(body));
    json!({"schema_version":"kyuubiki.solver-result-reference/v1","solver_method":"solve_bar_1d",
    "storage_mode":"orchestra_content_addressed","result_artifact_ref":{
        "schema_version":"kyuubiki.result-artifact-ref/v1","artifact_id":hash,"sha256":hash,
        "size_bytes":body.len(),"media_type":RESULT_MEDIA_TYPE,"immutable":true
    }})
}

fn response(status: &str, media: &str, headers: &str, body: &[u8]) -> Vec<u8> {
    let mut bytes =
        format!("HTTP/1.1 {status}\r\nContent-Type: {media}\r\n{headers}Connection: close\r\n\r\n")
            .into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

fn content(body: &[u8]) -> Reply {
    Reply::raw(response(
        "200 OK",
        RESULT_MEDIA_TYPE,
        &format!("Content-Length: {}\r\n", body.len()),
        body,
    ))
}

fn receipts(prefer: bool, result: Value) -> Vec<Reply> {
    let job = json!({"job_id":"owned-job","status":"completed","project_id":"owned-project"});
    if prefer {
        vec![Reply::json(json!({"job":job,"result":result}))]
    } else {
        vec![
            Reply::json(json!({"job":job})),
            Reply::json(json!({"job_id":"owned-job","project_id":"owned-project","result":result})),
        ]
    }
}

#[test]
fn verified_readback_preserves_physical_values_and_original_byte_identity_in_both_fetch_modes() {
    let body =
        br#"{ "tip_displacement": 1.2345678901234567e-10, "note": "\u03b1", "stress": -0.0 }"#;
    for prefer in [true, false] {
        let mut wrapper = reference(body);
        wrapper["result_artifact_ref"]["url"] = json!("http://invalid-host/steal-token");
        let mut replies = receipts(prefer, wrapper.clone());
        replies.push(content(body));
        let server = Server::new(replies);
        let result = server
            .fetch(json!({"job_id":"owned-job","prefer_job_result":prefer}))
            .unwrap();
        assert_eq!(
            result["result"],
            serde_json::from_slice::<Value>(body).unwrap()
        );
        assert_eq!(result["result_artifact_readback"]["verified"], true);
        assert_eq!(
            result["result_artifact_readback"]["artifact"]["sha256"],
            wrapper["result_artifact_ref"]["sha256"]
        );
        assert!(
            result["result_artifact_readback"]["artifact"]
                .get("url")
                .is_none()
        );
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), if prefer { 2 } else { 3 });
        let request = requests.last().unwrap();
        assert!(request.starts_with(&format!(
                "GET /api/v1/result-artifacts/{}/content HTTP/1.1",
                wrapper["result_artifact_ref"]["artifact_id"]
                    .as_str()
                    .unwrap()
            )));
        assert!(request.contains("Authorization: Bearer test-only-token\r\n"));
        assert!(request.contains("Accept-Encoding: identity\r\n"));
    }
}

#[test]
fn invalid_readback_options_stop_before_any_io_including_combined_solve_submission() {
    for (key, value) in [
        ("resolve_result_artifact", json!("false")),
        ("resolve_result_artifact", Value::Null),
        ("result_artifact_max_bytes", json!(0)),
        ("result_artifact_max_bytes", json!(536_870_913u64)),
        ("result_artifact_max_bytes", json!(1.5)),
        ("result_artifact_max_bytes", json!("12")),
        ("result_artifact_timeout_ms", json!(0)),
        ("result_artifact_timeout_ms", json!(600_001)),
        ("result_artifact_timeout_ms", Value::Null),
    ] {
        let server = Server::new(vec![]);
        let mut payload = json!({"job_id":"owned-job","model_version_id":"owned-version","endpoints":["127.0.0.1:5001"]});
        payload[key] = value;
        server.halt(payload.clone(), 0);
        let error = ServiceHeadlessExecutor::new(&server.url)
            .execute_step("solve_and_wait_from_model_version", 1, &payload)
            .unwrap_err();
        assert!(error.message.starts_with(READBACK_FAILURE));
        assert!(server.requests.lock().unwrap().is_empty());
    }
    assert_eq!(
        ResultReadPolicy::parse(&json!({})).unwrap().max_bytes,
        64 * 1024 * 1024
    );
}

#[test]
fn malformed_result_descriptors_never_trigger_download_or_downstream_writes() {
    let good = reference(b"{}");
    for (pointer, value) in [
        ("/schema_version", json!("other/v1")),
        ("/storage_mode", json!("external")),
        ("/solver_method", json!("../../secret")),
        ("/solver_method", Value::Null),
        ("/result_artifact_ref", Value::Null),
        ("/result_artifact_ref/schema_version", json!("other/v1")),
        ("/result_artifact_ref/artifact_id", json!("../outside")),
        ("/result_artifact_ref/artifact_id", json!("A".repeat(64))),
        ("/result_artifact_ref/sha256", json!("a".repeat(64))),
        ("/result_artifact_ref/media_type", json!("application/json")),
        ("/result_artifact_ref/immutable", json!(false)),
        ("/result_artifact_ref/immutable", json!("true")),
        ("/result_artifact_ref/size_bytes", json!(0)),
        ("/result_artifact_ref/size_bytes", json!("2")),
        ("/result_artifact_ref/size_bytes", json!(1.5)),
    ] {
        let mut wrapper = good.clone();
        *wrapper.pointer_mut(pointer).unwrap() = value;
        for prefer in [true, false] {
            let server = Server::new(receipts(prefer, wrapper.clone()));
            server.halt(
                json!({"job_id":"owned-job","prefer_job_result":prefer}),
                if prefer { 1 } else { 2 },
            );
        }
    }
    let mut missing = good;
    missing
        .as_object_mut()
        .unwrap()
        .remove("result_artifact_ref");
    Server::new(receipts(true, missing)).halt(json!({"job_id":"owned-job"}), 1);
}

#[test]
fn reference_only_mode_validates_without_download_and_byte_budget_blocks_before_content_io() {
    let mut wrapper = reference(b"{}");
    wrapper["result_artifact_ref"]["size_bytes"] = json!(600_000_000u64);
    let server = Server::new(receipts(true, wrapper.clone()));
    let result = server
        .fetch(json!({"job_id":"owned-job","resolve_result_artifact":false}))
        .unwrap();
    assert_eq!(result["result"], wrapper);
    assert!(result.get("result_artifact_readback").is_none());
    assert_eq!(server.requests.lock().unwrap().len(), 1);
    Server::new(receipts(true, wrapper)).halt(json!({"job_id":"owned-job"}), 1);
    Server::new(receipts(true, reference(b"{}"))).halt(
        json!({"job_id":"owned-job","result_artifact_max_bytes":1}),
        1,
    );
}

#[test]
fn inline_result_remains_unchanged_and_does_not_download() {
    for prefer in [true, false] {
        let server = Server::new(receipts(prefer, json!({"tip_displacement":1e-6})));
        let result = server
            .fetch(json!({"job_id":"owned-job","prefer_job_result":prefer}))
            .unwrap();
        assert_eq!(result["result"], json!({"tip_displacement":1e-6}));
        assert!(result.get("result_artifact_readback").is_none());
        assert_eq!(
            server.requests.lock().unwrap().len(),
            if prefer { 1 } else { 2 }
        );
    }
}

#[test]
fn hash_mismatch_in_unsampled_bytes_cannot_publish_a_result_or_retry_computation() {
    let body = serde_json::to_vec(&json!({"value":1,"padding":"a".repeat(140_000)})).unwrap();
    let mut corrupt = body.clone();
    let index = corrupt.iter().rposition(|byte| *byte == b'a').unwrap();
    corrupt[index] = b'b';
    for prefer in [true, false] {
        let mut replies = receipts(prefer, reference(&body));
        replies.push(content(&corrupt));
        Server::new(replies).halt(
            json!({"job_id":"owned-job","prefer_job_result":prefer}),
            if prefer { 2 } else { 3 },
        );
    }
}

#[test]
fn wrong_status_media_encoding_length_or_framing_never_unlocks_downstream_steps() {
    let body = b"{}";
    for reply in [
        response(
            "302 Found",
            RESULT_MEDIA_TYPE,
            "Location: http://invalid-host/steal\r\nContent-Length: 2\r\n",
            body,
        ),
        response(
            "401 Unauthorized",
            RESULT_MEDIA_TYPE,
            "Content-Length: 2\r\n",
            body,
        ),
        response(
            "404 Not Found",
            RESULT_MEDIA_TYPE,
            "Content-Length: 2\r\n",
            body,
        ),
        response(
            "500 Internal Server Error",
            RESULT_MEDIA_TYPE,
            "Content-Length: 2\r\n",
            body,
        ),
        response("200 OK", "application/json", "Content-Length: 2\r\n", body),
        response(
            "200 OK",
            RESULT_MEDIA_TYPE,
            "Content-Length: 2\r\nContent-Encoding: gzip\r\n",
            body,
        ),
        response(
            "200 OK",
            RESULT_MEDIA_TYPE,
            "Content-Length: 2\r\nContent-Length: 2\r\n",
            body,
        ),
        response(
            "200 OK",
            RESULT_MEDIA_TYPE,
            "Content-Length: 2\r\nTransfer-Encoding: chunked\r\n",
            body,
        ),
        response(
            "200 OK",
            RESULT_MEDIA_TYPE,
            "Content-Length: 999999999\r\n",
            body,
        ),
        response("200 OK", RESULT_MEDIA_TYPE, "Content-Length: 2\r\n", b"{"),
        response("200 OK", RESULT_MEDIA_TYPE, "Content-Length: 2\r\n", b"{}x"),
        response(
            "200 OK",
            RESULT_MEDIA_TYPE,
            "Transfer-Encoding: gzip\r\n",
            body,
        ),
        response("200 OK", RESULT_MEDIA_TYPE, "", body),
        response(
            "200 OK",
            RESULT_MEDIA_TYPE,
            "Broken-Header\r\nContent-Length: 2\r\n",
            body,
        ),
        response(
            "200 OK",
            RESULT_MEDIA_TYPE,
            &format!(
                "X-Padding: {}\r\nContent-Length: 2\r\n",
                "x".repeat(64 * 1024)
            ),
            body,
        ),
    ] {
        let mut replies = receipts(true, reference(body));
        replies.push(Reply::raw(reply));
        Server::new(replies).halt(json!({"job_id":"owned-job"}), 2);
    }
}

#[test]
fn fragmented_chunked_readback_has_bounded_exact_decoding_and_rejects_bad_terminators() {
    let body = br#"{"value":1}"#;
    let chunked = b"3;extension=yes\r\n{\"v\r\n8\r\nalue\":1}\r\n0\r\n\r\n";
    let bytes = response(
        "200 OK",
        &format!("{RESULT_MEDIA_TYPE}; charset=utf-8"),
        "Transfer-Encoding: chunked\r\n",
        chunked,
    );
    let mut replies = receipts(true, reference(body));
    replies.push(Reply {
        parts: bytes.chunks(3).map(<[u8]>::to_vec).collect(),
        delay: Duration::from_millis(1),
    });
    assert_eq!(
        Server::new(replies)
            .fetch(json!({"job_id":"owned-job"}))
            .unwrap()["result"],
        json!({"value":1})
    );
    for framed in [
        b"c\r\n{\"value\":1}x\r\n0\r\n\r\n".as_slice(),
        b"b\r\n{\"value\":1}\n\n0\r\n\r\n",
        b"0\r\n\r\n",
        b"FFFFFFFFFFFFFFFFF\r\n",
        b"b\r\n{\"value\":1}\r\n0\r\nTrailer: rejected\r\n\r\n",
        b"b\r\n{\"value\":1}\r\n0\r\n\r\nx",
    ] {
        let mut replies = receipts(true, reference(body));
        replies.push(Reply::raw(response(
            "200 OK",
            RESULT_MEDIA_TYPE,
            "Transfer-Encoding: chunked\r\n",
            framed,
        )));
        Server::new(replies).halt(json!({"job_id":"owned-job"}), 2);
    }
}

#[test]
fn verified_bytes_still_require_json_object_and_never_follow_nested_references() {
    let nested = serde_json::to_vec(&reference(b"{}")).unwrap();
    for body in [
        b"not-json".as_slice(),
        b"null",
        b"[]",
        b"3",
        b"{} trailing",
        &nested,
    ] {
        let mut replies = receipts(true, reference(body));
        replies.push(content(body));
        Server::new(replies).halt(json!({"job_id":"owned-job"}), 2);
    }
}

#[test]
fn trickled_result_bytes_exhaust_one_total_deadline_instead_of_resetting_it() {
    let body = serde_json::to_vec(&json!({"padding":"x".repeat(100)})).unwrap();
    let bytes = response(
        "200 OK",
        RESULT_MEDIA_TYPE,
        &format!("Content-Length: {}\r\n", body.len()),
        &body,
    );
    let mut replies = receipts(true, reference(&body));
    let split = bytes
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .unwrap()
        + 4;
    let mut parts = vec![bytes[..split].to_vec()];
    parts.extend(bytes[split..].chunks(1).map(<[u8]>::to_vec));
    replies.push(Reply {
        parts,
        delay: Duration::from_millis(5),
    });
    let server = Server::new(replies);
    let start = Instant::now();
    server.halt(
        json!({"job_id":"owned-job","result_artifact_timeout_ms":45}),
        2,
    );
    assert!(start.elapsed() < Duration::from_millis(500));
}

#[test]
fn result_spool_is_exclusive_private_and_removed_on_success_or_early_return() {
    let mut first = ResultSpool::create().unwrap();
    let second = ResultSpool::create().unwrap();
    assert_ne!(first.path, second.path);
    first.file_mut().write_all(b"owned").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&first.path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let path = first.path.clone();
    drop(first);
    assert!(!path.exists());
    let path = second.path.clone();
    let fail = || -> Result<(), &'static str> {
        let _owner = second;
        Err("intentional early return")
    };
    assert!(fail().is_err());
    assert!(!path.exists());
}

#[test]
fn combined_saved_solve_forwards_read_policy_to_actual_result_fetch() {
    let body = br#"{"value":1}"#;
    for (resolve, prefer) in [(false, true), (true, true), (true, false)] {
        let wrapper = reference(body);
        let mut replies = vec![
            Reply::json(
                json!({"version":{"version_id":"owned-version","model_id":"owned-model","kind":"axial_bar_1d","project_id":"owned-project",
                "payload":{"length":1,"area":0.01,"young_modulus":210e9,"force":1000,"elements":4}}}),
            ),
            Reply::json(
                json!({"job":{"job_id":"owned-job","status":"queued","project_id":"owned-project","model_version_id":"owned-version"}}),
            ),
            Reply::json(
                json!({"job":{"job_id":"owned-job","status":"completed","model_version_id":"owned-version"}}),
            ),
        ];
        let receipt = json!({"job":{"job_id":"owned-job","status":"completed","model_version_id":"owned-version"},"result":wrapper});
        if prefer {
            replies.push(Reply::json(receipt));
        } else {
            replies.push(Reply::json(json!({"job":{"job_id":"owned-job","status":"completed","model_version_id":"owned-version"}})));
            replies.push(Reply::json(
                json!({"job_id":"owned-job","model_version_id":"owned-version","result":wrapper}),
            ));
        }
        if resolve {
            replies.push(content(body));
        }
        let server = Server::new(replies);
        let outcome = ServiceHeadlessExecutor::new(&server.url).execute_step("solve_and_wait_from_model_version", 1,
            &json!({"model_version_id":"owned-version","endpoints":["127.0.0.1:5001"],"interval_ms":1,"timeout_ms":2000,
                "resolve_result_artifact":resolve,"prefer_job_result":prefer,"result_artifact_max_bytes":if resolve {32} else {1},"result_artifact_timeout_ms":1500})).unwrap();
        if resolve {
            assert_eq!(outcome.result["result"]["result"], json!({"value":1}));
            let receipt = &outcome.result["result"]["result_artifact_readback"];
            assert_eq!(receipt["max_bytes"], 32);
            assert_eq!(receipt["timeout_ms"], 1500);
        } else {
            assert_eq!(outcome.result["result"]["result"], wrapper);
        }
        let requests = server.requests.lock().unwrap();
        assert_eq!(
            requests.len(),
            4 + usize::from(resolve) + usize::from(!prefer)
        );
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.starts_with("POST "))
                .count(),
            1
        );
    }
}

#[test]
fn readback_receipt_and_normalized_descriptor_match_the_public_schema_fields() {
    let body = b"{}";
    let mut replies = receipts(true, reference(body));
    replies.push(content(body));
    let result = Server::new(replies)
        .fetch(json!({"job_id":"owned-job"}))
        .unwrap();
    for (value, schema) in [
        (
            &result["result_artifact_readback"],
            include_str!("../../../../../schemas/headless-result-artifact-readback.schema.json"),
        ),
        (
            &result["result_artifact_readback"]["artifact"],
            include_str!("../../../../../schemas/result-artifact-ref.schema.json"),
        ),
    ] {
        let schema: Value = serde_json::from_str(schema).unwrap();
        let actual = value.as_object().unwrap();
        assert_eq!(actual.len(), schema["required"].as_array().unwrap().len());
        for key in schema["required"].as_array().unwrap() {
            assert!(actual.contains_key(key.as_str().unwrap()));
        }
        for (key, rule) in schema["properties"].as_object().unwrap() {
            if let Some(constant) = rule.get("const") {
                assert_eq!(&value[key], constant);
            }
        }
    }
}
