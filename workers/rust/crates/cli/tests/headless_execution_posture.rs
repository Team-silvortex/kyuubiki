use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn workflow_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kyuubiki-headless-posture-{unique}.json"));
    let workflow = json!({
        "schema_version": "kyuubiki.headless-workflow/v1",
        "exported_at": "2026-07-28T00:00:00Z",
        "language": "en",
        "workflow": {
            "id": "workflow.execution-posture",
            "steps": [{ "action": "service_health", "payload": {} }]
        }
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&workflow).expect("workflow json"),
    )
    .expect("write workflow");
    path
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kyuubiki-headless"))
        .args(args)
        .output()
        .expect("run headless CLI")
}

#[test]
fn execute_rejects_implicit_mock_executor() {
    let path = workflow_path();
    let output = run(&["run", path.to_str().expect("path"), "--execute"]);

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("explicit --executor"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = fs::remove_file(path);
}

#[test]
fn research_posture_rejects_mock_and_preview_accepts_explicit_mock() {
    let path = workflow_path();
    let path_text = path.to_str().expect("path");
    let rejected = run(&[
        "run",
        path_text,
        "--execute",
        "--executor",
        "mock",
        "--execution-posture",
        "research",
    ]);
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("no-mock execution guarantee"),
        "stderr: {}",
        String::from_utf8_lossy(&rejected.stderr)
    );

    let preview = run(&[
        "run",
        path_text,
        "--json",
        "--execute",
        "--executor",
        "mock",
        "--execution-posture",
        "preview",
    ]);
    assert!(
        preview.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let report: Value = serde_json::from_slice(&preview.stdout).expect("run report");
    assert_eq!(report["mode"], "execute:mock");
    let _ = fs::remove_file(path);
}
