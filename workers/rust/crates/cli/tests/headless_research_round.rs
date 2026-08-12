use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kyuubiki-headless"))
        .args(args)
        .output()
        .expect("run headless CLI")
}

fn workspace() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "kyuubiki-headless-research-round-{}-{nanos}-{sequence}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("workspace");
    path
}

fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).expect("encode json")).expect("write json");
}

fn write_workflow(root: &Path) -> PathBuf {
    let path = root.join("workflow.json");
    write_json(
        &path,
        &json!({
            "schema_version": "kyuubiki.headless-workflow/v1",
            "exported_at": "2026-08-12T00:00:00Z",
            "language": "en",
            "workflow": {
                "id": "research.service-health",
                "steps": [{
                    "action": "service_health",
                    "payload": {"research_input": 1.0}
                }]
            }
        }),
    );
    path
}

fn write_spec(root: &Path, round_id: &str, iteration: u64) -> PathBuf {
    let path = root.join(format!("round-{iteration}.spec.json"));
    write_json(
        &path,
        &json!({
            "schema_version": "kyuubiki.headless-research-round-spec/v1",
            "round_id": round_id,
            "workflow_id": "research.service-health",
            "iteration": iteration,
            "primary_metric_ids": ["research_metric"],
            "metrics": [{
                "metric_id": "research_metric",
                "pointer": "/steps/0/result_preview/result/research_metric",
                "unit": "1",
                "objective": "minimize"
            }]
        }),
    );
    path
}

fn write_patch(root: &Path) -> PathBuf {
    let path = root.join("round-2.patch.json");
    write_json(
        &path,
        &json!({
            "schema_version": "kyuubiki.headless-parameter-patch/v1",
            "patch_id": "service-health-round-2",
            "workflow_id": "research.service-health",
            "changes": [{
                "path": "/steps/0/payload/research_input",
                "expected": 1.0,
                "value": 2.0
            }]
        }),
    );
    path
}

fn health_server(research_metric: f64) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind server");
    let port = listener.local_addr().expect("server address").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept health request");
        let mut request = [0_u8; 4096];
        let count = stream.read(&mut request).expect("read health request");
        assert!(String::from_utf8_lossy(&request[..count]).starts_with("GET /api/health "));
        let body = serde_json::to_string(&json!({
            "service": "research-fixture",
            "status": "ok",
            "result": {"research_metric": research_metric}
        }))
        .expect("response json");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write health response");
    });
    (format!("http://127.0.0.1:{port}"), handle)
}

fn path(path: &Path) -> &str {
    path.to_str().expect("utf8 path")
}

#[test]
fn service_research_runs_emit_a_contiguous_two_round_evidence_chain() {
    let root = workspace();
    let workflow = write_workflow(&root);
    let first_spec = write_spec(&root, "service-health-round-1", 1);
    let second_spec = write_spec(&root, "service-health-round-2", 2);
    let patch = write_patch(&root);
    let first_evidence = root.join("round-1.evidence.json");
    let second_evidence = root.join("round-2.evidence.json");
    let receipt = root.join("round-2.receipt.json");

    let (first_url, first_server) = health_server(8.0);
    let first = run(&[
        "run",
        path(&workflow),
        "--json",
        "--execute",
        "--executor",
        "service",
        "--execution-posture",
        "research",
        "--api-base-url",
        &first_url,
        "--research-round-spec",
        path(&first_spec),
        "--research-round-out",
        path(&first_evidence),
    ]);
    first_server.join().expect("first server");
    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let (second_url, second_server) = health_server(6.0);
    let second = run(&[
        "run",
        path(&workflow),
        "--json",
        "--execute",
        "--executor",
        "service",
        "--execution-posture",
        "research",
        "--api-base-url",
        &second_url,
        "--parameter-patch",
        path(&patch),
        "--parameter-patch-receipt-out",
        path(&receipt),
        "--research-round-spec",
        path(&second_spec),
        "--previous-round-evidence",
        path(&first_evidence),
        "--research-round-out",
        path(&second_evidence),
    ]);
    second_server.join().expect("second server");
    assert!(
        second.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    let first: Value = serde_json::from_slice(&fs::read(&first_evidence).expect("first evidence"))
        .expect("first evidence json");
    let second: Value =
        serde_json::from_slice(&fs::read(&second_evidence).expect("second evidence"))
            .expect("second evidence json");
    let receipt: Value =
        serde_json::from_slice(&fs::read(&receipt).expect("receipt")).expect("receipt json");
    assert_eq!(first["qualified"], true);
    assert_eq!(second["qualified"], true);
    assert_eq!(second["metrics"][0]["value"], 6.0);
    assert_eq!(
        second["previous_round"]["round_id"],
        "service-health-round-1"
    );
    assert_eq!(
        second["previous_round"]["batch_content_sha256"],
        receipt["before_sha256"]
    );
    assert_eq!(second["batch_content_sha256"], receipt["after_sha256"]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn later_round_without_previous_evidence_fails_before_service_connection() {
    let root = workspace();
    let workflow = write_workflow(&root);
    let second_spec = write_spec(&root, "service-health-round-2", 2);
    let patch = write_patch(&root);
    let output = run(&[
        "run",
        path(&workflow),
        "--json",
        "--execute",
        "--executor",
        "service",
        "--execution-posture",
        "research",
        "--api-base-url",
        "http://127.0.0.1:9",
        "--parameter-patch",
        path(&patch),
        "--research-round-spec",
        path(&second_spec),
        "--research-round-out",
        path(&root.join("round-2.evidence.json")),
    ]);

    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("preflight report");
    assert_eq!(
        report["execution_summary"]["failure"]["error_code"],
        "kyuubiki.headless.research_round_validation"
    );
    assert_eq!(
        report["execution_summary"]["failure"]["stage"],
        "research_round"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("--previous-round-evidence"));

    let wrong_spec = root.join("wrong-workflow.spec.json");
    write_json(
        &wrong_spec,
        &json!({
            "schema_version": "kyuubiki.headless-research-round-spec/v1",
            "round_id": "wrong-target-round-1",
            "workflow_id": "research.other-workflow",
            "iteration": 1,
            "primary_metric_ids": ["research_metric"],
            "metrics": [{
                "metric_id": "research_metric",
                "pointer": "/steps/0/result_preview/result/research_metric",
                "unit": "1",
                "objective": "observe"
            }]
        }),
    );
    let wrong_target = run(&[
        "run",
        path(&workflow),
        "--json",
        "--execute",
        "--executor",
        "service",
        "--execution-posture",
        "research",
        "--api-base-url",
        "http://127.0.0.1:9",
        "--research-round-spec",
        path(&wrong_spec),
        "--research-round-out",
        path(&root.join("wrong-workflow.evidence.json")),
    ]);
    assert!(!wrong_target.status.success());
    assert!(String::from_utf8_lossy(&wrong_target.stderr).contains("workflow mismatch"));
    let _ = fs::remove_dir_all(root);
}
