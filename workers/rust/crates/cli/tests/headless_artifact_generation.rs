use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-artifact-generation-{}-{stamp}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kyuubiki-headless"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}

fn write_json(root: &Path, name: &str, value: &Value) {
    fs::write(root.join(name), serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn assert_generation_failure(output: &Output) -> Value {
    assert!(!output.status.success());
    assert!(
        !output.stdout.is_empty(),
        "completed receipt disappeared: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        diagnostic["error"]["code"], "report_generation_failure",
        "{diagnostic}"
    );
    assert_eq!(diagnostic["error"]["stage"], "artifact_output");
    assert_eq!(diagnostic["error"]["retryable"], false);
    assert!(
        diagnostic["error"]["recommended_action"]
            .as_str()
            .unwrap()
            .contains("do not replay")
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "ok");
    assert!(report["execution_summary"]["failure"].is_null());
    report
}

struct HealthService {
    url: String,
    calls: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl HealthService {
    fn start(result: Value) -> Self {
        Self::with_status(result, 200)
    }

    fn with_status(result: Value, status: u16) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let calls = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (call_counter, stopping) = (calls.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(20);
            let body = serde_json::to_string(
                &json!({"service":"generation-fixture","status":"ok","result":result}),
            )
            .unwrap();
            while !stopping.load(Ordering::Acquire) && Instant::now() < deadline {
                let mut stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("accept health request: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let request = read_health_request(&mut stream);
                assert!(request.starts_with(b"GET /api/health "));
                call_counter.fetch_add(1, Ordering::Release);
                write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        Self {
            url,
            calls,
            stop,
            thread: Some(thread),
        }
    }
}

fn read_health_request(stream: &mut TcpStream) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut request = Vec::with_capacity(256);
    while !request.windows(4).any(|part| part == b"\r\n\r\n") {
        assert!(
            request.len() < 4096,
            "health request headers exceed fixture budget"
        );
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "health request header deadline exceeded"
        );
        stream.set_read_timeout(Some(remaining)).unwrap();
        let mut chunk = [0; 256];
        let capacity = chunk.len().min(4096 - request.len());
        let count = match stream.read(&mut chunk[..capacity]) {
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => panic!("read health request headers: {error}"),
        };
        assert_ne!(count, 0, "health request ended before complete headers");
        request.extend_from_slice(&chunk[..count]);
    }
    request
}

impl Drop for HealthService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let joined = thread.join();
            if !std::thread::panicking() {
                joined.unwrap();
            }
        }
    }
}

#[test]
fn health_fixture_waits_for_delayed_fragmented_request_headers() {
    let service = HealthService::start(json!({"available_metric":8.0}));
    for _ in 0..3 {
        let mut client = TcpStream::connect(service.url.strip_prefix("http://").unwrap()).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        client
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        std::thread::sleep(Duration::from_millis(20));
        client.write_all(b"GE").unwrap();
        std::thread::sleep(Duration::from_millis(20));
        client
            .write_all(b"T /api/health HTTP/1.1\r\nHost: fixture\r\n")
            .unwrap();
        std::thread::sleep(Duration::from_millis(20));
        client.write_all(b"Connection: close\r\n\r\n").unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(
            response.starts_with("HTTP/1.1 200 Fixture\r\n"),
            "{response}"
        );
    }
    assert_eq!(service.calls.load(Ordering::Acquire), 3);
}

#[test]
fn incomplete_material_workflows_now_fail_preflight_and_retain_invalid_receipts() {
    for retained_steps in [2, 3] {
        for save_run in [false, true] {
            let scratch = Scratch::new();
            let initialized = run(
                &scratch.0,
                &[
                    "init",
                    "--template",
                    "material_heat_spreader_screening",
                    "--out",
                    "workflow.json",
                ],
            );
            assert!(
                initialized.status.success(),
                "{}",
                String::from_utf8_lossy(&initialized.stderr)
            );
            let mut workflow: Value =
                serde_json::from_slice(&fs::read(scratch.0.join("workflow.json")).unwrap())
                    .unwrap();
            workflow["workflow"]["steps"]
                .as_array_mut()
                .unwrap()
                .truncate(retained_steps);
            write_json(&scratch.0, "workflow.json", &workflow);
            let original = fs::read(scratch.0.join("workflow.json")).unwrap();
            let retained_material = b"{\"retained_material\":true}";
            fs::write(scratch.0.join("material.json"), retained_material).unwrap();
            let mut args = vec![
                "run",
                "workflow.json",
                "--json",
                "--execute",
                "--executor",
                "mock",
                "--material-report",
                "heat-spreader",
                "--material-report-out",
                "material.json",
            ];
            if save_run {
                args.extend(["--report-out", "completed-run.json"]);
            }
            let output = run(&scratch.0, &args);
            assert!(!output.status.success());
            let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(
                diagnostic["error"]["code"],
                "material_report_input_contract_mismatch"
            );
            assert_eq!(diagnostic["error"]["stage"], "material_report_validation");
            assert_eq!(diagnostic["error"]["retryable"], false);
            let report: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["status"], "invalid");
            assert_eq!(report["executed_step_count"], 0);
            assert_eq!(report["steps"], json!([]));
            assert_eq!(fs::read(scratch.0.join("workflow.json")).unwrap(), original);
            assert_eq!(
                fs::read(scratch.0.join("material.json")).unwrap(),
                retained_material
            );
            if save_run {
                assert_eq!(
                    report,
                    serde_json::from_slice::<Value>(
                        &fs::read(scratch.0.join("completed-run.json")).unwrap()
                    )
                    .unwrap()
                );
            }
            assert_eq!(
                fs::read_dir(&scratch.0).unwrap().count(),
                if save_run { 3 } else { 2 }
            );
        }
    }
}

fn health_workflow_and_spec(root: &Path, metric: &str) {
    write_json(
        root,
        "workflow.json",
        &json!({
            "schema_version":"kyuubiki.headless-workflow/v1","language":"en","exported_at":"2026-10-07T00:00:00Z",
            "workflow":{"id":"generation-health","steps":[{"action":"service_health","payload":{}}]}
        }),
    );
    write_json(
        root,
        "round.json",
        &json!({
            "schema_version":"kyuubiki.headless-research-round-spec/v1","round_id":"round-1",
            "workflow_id":"generation-health","iteration":1,"primary_metric_ids":["observed_metric"],
            "metrics":[{"metric_id":"observed_metric","pointer":format!("/steps/0/result_preview/result/{metric}"),
                "unit":"1","objective":"observe"}]
        }),
    );
}

#[test]
fn missing_research_metric_cannot_become_a_retryable_timeout_after_successful_execution() {
    let scratch = Scratch::new();
    write_json(
        &scratch.0,
        "workflow.json",
        &json!({
            "schema_version":"kyuubiki.headless-workflow/v1","language":"en","exported_at":"2026-10-07T00:00:00Z",
            "workflow":{"id":"generation-health","steps":[{"action":"service_health","payload":{}}]}
        }),
    );
    write_json(
        &scratch.0,
        "round.json",
        &json!({
            "schema_version":"kyuubiki.headless-research-round-spec/v1","round_id":"round-1",
            "workflow_id":"generation-health","iteration":1,"primary_metric_ids":["missing_metric"],
            "metrics":[{"metric_id":"missing_metric","pointer":"/steps/0/result_preview/result/timed out waiting for job",
                "unit":"1","objective":"observe"}]
        }),
    );
    let retained = b"{\"retained_evidence\":true}";
    fs::write(scratch.0.join("evidence.json"), retained).unwrap();
    let service = HealthService::start(json!({"available_metric":8.0}));
    let output = run(
        &scratch.0,
        &[
            "run",
            "workflow.json",
            "--json",
            "--execute",
            "--executor",
            "service",
            "--execution-posture",
            "research",
            "--api-base-url",
            &service.url,
            "--research-round-spec",
            "round.json",
            "--research-round-out",
            "evidence.json",
        ],
    );
    assert_eq!(service.calls.load(Ordering::Acquire), 1);
    let report = assert_generation_failure(&output);
    assert_eq!(report["executed_step_count"], 1);
    assert_eq!(
        report["steps"][0]["result_preview"]["result"]["available_metric"],
        8.0
    );
    assert_eq!(fs::read(scratch.0.join("evidence.json")).unwrap(), retained);
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 3);
}

#[test]
fn non_numeric_metrics_preserve_completed_reports_without_publishing_false_evidence() {
    for metric in [
        Value::Null,
        json!("8.0"),
        json!(true),
        json!([]),
        json!({"value":8.0}),
    ] {
        let scratch = Scratch::new();
        health_workflow_and_spec(&scratch.0, "observed_metric");
        let original = fs::read(scratch.0.join("workflow.json")).unwrap();
        let service = HealthService::start(json!({"observed_metric":metric}));
        let output = run(
            &scratch.0,
            &[
                "run",
                "workflow.json",
                "--json",
                "--execute",
                "--executor",
                "service",
                "--execution-posture",
                "research",
                "--api-base-url",
                &service.url,
                "--research-round-spec",
                "round.json",
                "--research-round-out",
                "evidence.json",
                "--report-out",
                "run.json",
            ],
        );
        let report = assert_generation_failure(&output);
        assert_eq!(report["executed_step_count"], 1);
        assert_eq!(service.calls.load(Ordering::Acquire), 1);
        assert_eq!(
            report,
            serde_json::from_slice::<Value>(&fs::read(scratch.0.join("run.json")).unwrap())
                .unwrap()
        );
        assert_eq!(fs::read(scratch.0.join("workflow.json")).unwrap(), original);
        assert!(!scratch.0.join("evidence.json").exists());
        assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 3);
    }
}

#[test]
fn non_json_incomplete_material_plan_never_prints_a_completed_run_or_winner() {
    let scratch = Scratch::new();
    let initialized = run(
        &scratch.0,
        &[
            "init",
            "--template",
            "material_heat_spreader_screening",
            "--out",
            "workflow.json",
        ],
    );
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    let mut workflow: Value =
        serde_json::from_slice(&fs::read(scratch.0.join("workflow.json")).unwrap()).unwrap();
    workflow["workflow"]["steps"]
        .as_array_mut()
        .unwrap()
        .truncate(3);
    write_json(&scratch.0, "workflow.json", &workflow);
    let output = run(
        &scratch.0,
        &[
            "run",
            "workflow.json",
            "--execute",
            "--executor",
            "mock",
            "--material-report",
            "heat-spreader",
        ],
    );
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains("Status: ok"));
    assert!(!stdout.contains("Executed steps:"));
    assert!(!stdout.contains("Material winner:"));
    assert!(stderr.starts_with("material-report input contract mismatch:"));
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
}

#[test]
fn actual_execution_failure_remains_execution_failure_without_entering_artifact_generation() {
    let scratch = Scratch::new();
    health_workflow_and_spec(&scratch.0, "observed_metric");
    let service = HealthService::with_status(json!({"observed_metric":8.0}), 500);
    let output = run(
        &scratch.0,
        &[
            "run",
            "workflow.json",
            "--json",
            "--execute",
            "--executor",
            "service",
            "--execution-posture",
            "research",
            "--api-base-url",
            &service.url,
            "--research-round-spec",
            "round.json",
            "--research-round-out",
            "evidence.json",
        ],
    );
    assert!(!output.status.success());
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostic["error"]["code"], "headless_execution_failed");
    assert_eq!(diagnostic["error"]["stage"], "execution");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "failed");
    assert_eq!(report["executed_step_count"], 0);
    assert!(!report["execution_summary"]["failure"].is_null());
    assert_eq!(service.calls.load(Ordering::Acquire), 1);
    assert!(!scratch.0.join("evidence.json").exists());
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 2);
}
