use crate::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessRisk, ServiceHeadlessExecutor,
    execute_batch_with_executor,
};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

struct Server {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: mpsc::Sender<()>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    fn new(responses: Vec<(&str, Value)>) -> Self {
        let mut responses = responses
            .into_iter()
            .map(|(path, body)| (path.to_owned(), body.to_string()))
            .collect::<VecDeque<_>>();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let requests = Arc::new(Mutex::new(vec![]));
        let captured = Arc::clone(&requests);
        let (stop, stopped) = mpsc::channel();
        let worker = thread::spawn(move || {
            while stopped.try_recv().is_err() {
                let (mut stream, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    Err(error) => panic!("accept fixture: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = vec![];
                let mut buffer = [0; 4096];
                while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                    let size = stream.read(&mut buffer).unwrap();
                    assert!(size > 0 && request.len() + size <= 64 * 1024);
                    request.extend_from_slice(&buffer[..size]);
                }
                let first = std::str::from_utf8(&request)
                    .unwrap()
                    .lines()
                    .next()
                    .unwrap()
                    .to_owned();
                captured.lock().unwrap().push(first.clone());
                let response = responses.pop_front();
                let (status, body) = match response {
                    Some((expected, body)) if first == expected => ("200 OK", body),
                    Some(_) => (
                        "500 Internal Server Error",
                        "unexpected request route".into(),
                    ),
                    None => (
                        "500 Internal Server Error",
                        "unexpected downstream request".into(),
                    ),
                };
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        Self {
            url,
            requests,
            stop,
            worker: Some(worker),
        }
    }

    fn run(&self, action: &str, payload: Value) -> crate::HeadlessRunReport {
        let document = HeadlessExecutionBatch {
            schema_version: "kyuubiki.headless-execution-batch/v1".into(),
            exported_at: "2026-10-07T00:00:00Z".into(),
            language: "en".into(),
            workflow_id: "job-result-gate".into(),
            template_id: None,
            warnings: vec![],
            steps: vec![
                HeadlessExecutionBatchStep {
                    index: 1,
                    action: action.into(),
                    risk: HeadlessRisk::Normal,
                    payload,
                },
                HeadlessExecutionBatchStep {
                    index: 2,
                    action: "project_create".into(),
                    risk: HeadlessRisk::Normal,
                    payload: json!({"name":"must-not-exist-after-incomplete-result"}),
                },
            ],
        };
        execute_batch_with_executor(
            &document,
            &mut ServiceHeadlessExecutor::new(&self.url),
            false,
            false,
        )
    }

    fn assert_halted(&self, report: &crate::HeadlessRunReport, reads: usize, code: &str) {
        assert_eq!(report.status, "failed", "{report:?}");
        assert_eq!(report.executed_step_count, 0);
        assert_eq!(report.steps.len(), 1);
        assert_eq!(self.requests.lock().unwrap().len(), reads);
        let failure = report.execution_summary.failure.as_ref().unwrap();
        assert_eq!(failure.error_code, format!("kyuubiki.headless.{code}"));
        assert!(!failure.retryable);
        assert_eq!(failure.retry_strategy, "none");
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

const JOB: &str = "GET /api/v1/jobs/owned-job HTTP/1.1";
const STATUS: &str = "GET /api/v1/jobs/owned-job/status HTTP/1.1";
const RESULT: &str = "GET /api/v1/results/owned-job HTTP/1.1";

#[test]
fn receipt_states_follow_the_public_job_contract_and_only_completed_unlocks_results() {
    use crate::service_executor_job_receipt::{
        JOB_STATUSES, require_completed_job, validate_job_receipt,
    };

    let schema: Value =
        serde_json::from_str(include_str!("../../../../../schemas/job.schema.json")).unwrap();
    let mut declared = schema["properties"]["status"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|status| status.as_str().unwrap())
        .collect::<Vec<_>>();
    let mut accepted = JOB_STATUSES.to_vec();
    declared.sort_unstable();
    accepted.sort_unstable();
    assert_eq!(accepted, declared);
    for status in schema["properties"]["status"]["enum"].as_array().unwrap() {
        let receipt = json!({"job":{"job_id":"owned-job","status":status}});
        assert!(validate_job_receipt("owned-job", &receipt).is_ok());
        assert_eq!(
            require_completed_job("owned-job", &receipt).is_ok(),
            status == "completed"
        );
    }
    for status in [
        json!("running"),
        json!("unknown"),
        json!(""),
        json!(3),
        Value::Null,
    ] {
        let receipt = json!({"job":{"job_id":"owned-job","status":status}});
        assert!(validate_job_receipt("owned-job", &receipt).is_err());
    }
}

#[test]
fn retained_partial_results_do_not_authorize_downstream_work_for_either_fetch_mode() {
    for status in [
        "queued",
        "preprocessing",
        "partitioning",
        "solving",
        "postprocessing",
        "failed",
        "cancelled",
    ] {
        for prefer in [true, false] {
            let server = Server::new(vec![(
                if prefer { JOB } else { STATUS },
                json!({"job":{"job_id":"owned-job","status":status},"result":{"runtime":"retained-only"}}),
            )]);
            let report = server.run(
                "result_fetch",
                json!({"job_id":"owned-job","prefer_job_result":prefer}),
            );
            server.assert_halted(&report, 1, "job_result_unavailable");
        }
    }
}

#[test]
fn completed_job_requires_an_object_result_not_null_or_scalar_success() {
    for result in [
        Value::Null,
        json!(false),
        json!(1),
        json!("ready"),
        json!([]),
    ] {
        let server = Server::new(vec![(
            JOB,
            json!({"job":{"job_id":"owned-job","status":"completed"},"result":result}),
        )]);
        let report = server.run("result_fetch", json!({"job_id":"owned-job"}));
        server.assert_halted(&report, 1, "job_result_unavailable");
    }
}

#[test]
fn job_identity_and_status_are_validated_before_accepting_or_polling_a_receipt() {
    for job in [
        json!({"job_id":"foreign-secret-id","status":"completed"}),
        json!({"status":"completed"}),
        json!({"job_id":"owned-job"}),
        json!({"job_id":"owned-job","status":"unknown"}),
        Value::Null,
    ] {
        for action in ["result_fetch", "job_wait"] {
            let server = Server::new(vec![(
                if action == "job_wait" { STATUS } else { JOB },
                json!({"job":job,"result":{"value":123}}),
            )]);
            let report = server.run(
                action,
                json!({"job_id":"owned-job","timeout_ms":100,"interval_ms":10}),
            );
            server.assert_halted(&report, 1, "job_receipt_invalid");
            assert!(
                !report
                    .execution_summary
                    .failure
                    .unwrap()
                    .message
                    .contains("foreign-secret-id")
            );
        }
    }
    for receipt in [
        json!({"job_id":"conflicting-top-id",
        "job":{"job_id":"owned-job","status":"completed"},"result":{}}),
        json!({"status":"completed",
        "job":{"job_id":"owned-job","status":"failed"},"result":{}}),
        json!({"status":"failed",
        "job":{"job_id":"owned-job","status":"completed"},"result":{}}),
    ] {
        for action in ["result_fetch", "job_wait"] {
            let server = Server::new(vec![(
                if action == "job_wait" { STATUS } else { JOB },
                receipt.clone(),
            )]);
            let report = server.run(action, json!({"job_id":"owned-job"}));
            server.assert_halted(&report, 1, "job_receipt_invalid");
        }
    }
}

#[test]
fn result_endpoint_requires_matching_identity_and_explicit_object_payload() {
    for (result, code) in [
        (
            json!({"job_id":"foreign-secret-id","result":{"value":1}}),
            "job_receipt_invalid",
        ),
        (json!({"result":{}}), "job_receipt_invalid"),
        (
            json!({"job_id":"owned-job","value":1}),
            "job_result_unavailable",
        ),
        (
            json!({"job_id":"owned-job","result":null}),
            "job_result_unavailable",
        ),
        (
            json!({"job_id":"owned-job","status":"failed","result":{}}),
            "job_receipt_invalid",
        ),
        (
            json!({"job_id":"owned-job","status":null,"result":{}}),
            "job_receipt_invalid",
        ),
        (
            json!({"job_id":"owned-job","job":null,"result":{}}),
            "job_receipt_invalid",
        ),
        (
            json!({"job_id":"owned-job","job":{"job_id":"foreign-secret-id","status":"completed"},"result":{}}),
            "job_receipt_invalid",
        ),
        (
            json!({"job_id":"owned-job","job":{"job_id":"owned-job","status":"failed"},"result":{}}),
            "job_result_unavailable",
        ),
        (
            json!({"job_id":"owned-job","status":"failed","job":{"job_id":"owned-job","status":"completed"},"result":{}}),
            "job_receipt_invalid",
        ),
    ] {
        let server = Server::new(vec![
            (
                STATUS,
                json!({"job":{"job_id":"owned-job","status":"completed"}}),
            ),
            (RESULT, result),
        ]);
        let report = server.run(
            "result_fetch",
            json!({"job_id":"owned-job","prefer_job_result":false}),
        );
        server.assert_halted(&report, 2, code);
    }
}

#[test]
fn complete_inline_and_reference_results_preserve_full_values_without_raw_mirror() {
    let inline =
        json!({"nodes":(0..256).collect::<Vec<_>>(),"literal":"{{steps.1.result.job_id}}"});
    let reference = json!({"schema_version":"kyuubiki.solver-result-reference/v1",
        "solver_method":"solve_bar_1d", "storage_mode":"orchestra_content_addressed",
        "result_artifact_ref":{"schema_version":"kyuubiki.result-artifact-ref/v1",
            "artifact_id":"a".repeat(64),"sha256":"a".repeat(64),"size_bytes":12345,
            "media_type":"application/vnd.kyuubiki.result+json","immutable":true}});
    for result in [inline, reference, json!({})] {
        for mode in 0..3 {
            let first = if mode == 1 { STATUS } else { JOB };
            let mut job = json!({"job":{"job_id":"owned-job","status":"completed"}});
            if mode == 0 {
                job["result"] = result.clone();
            }
            let mut replies = vec![(first, job)];
            if mode != 0 {
                replies.push((RESULT, json!({"job_id":"owned-job","result":result})));
            }
            replies.push((
                "POST /api/v1/projects HTTP/1.1",
                json!({"project":{"project_id":"new-project"}}),
            ));
            let server = Server::new(replies);
            let report = server.run(
                "result_fetch",
                json!({"job_id":"owned-job","prefer_job_result": mode != 1,"resolve_result_artifact":false}),
            );
            assert_eq!(report.status, "ok", "{report:?}");
            assert_eq!(report.executed_step_count, 2);
            assert_eq!(
                server.requests.lock().unwrap().len(),
                if mode == 0 { 2 } else { 3 }
            );
            assert!(report.steps[0].result_preview.get("raw").is_none());
            if result.get("nodes").is_some() {
                assert_eq!(
                    report.steps[0].result_preview["result"]["nodes"]["item_count"],
                    256
                );
                assert_eq!(
                    report.steps[0].result_preview["result"]["literal"],
                    result["literal"]
                );
            } else {
                assert_eq!(report.steps[0].result_preview["result"], result);
            }
        }
    }
}
