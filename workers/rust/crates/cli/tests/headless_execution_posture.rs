use serde_json::{Value, json};
use std::fs;
use std::net::TcpListener;
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

fn unique_path(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("kyuubiki-headless-{label}-{unique}.json"))
}

fn init_template(template: &str) -> PathBuf {
    let path = unique_path(template);
    let output = run(&[
        "init",
        "--template",
        template,
        "--out",
        path.to_str().expect("template path"),
    ]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    path
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

#[test]
fn service_executor_rejects_base_url_paths_before_execution() {
    let path = workflow_path();
    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        "http://127.0.0.1:3000/not-api",
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid --api-base-url"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("paths, queries, and fragments"),
        "stderr: {stderr}"
    );
    assert!(output.stdout.is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn service_execution_failure_returns_nonzero_with_root_cause() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve local port");
    let port = listener.local_addr().expect("local address").port();
    drop(listener);
    let path = workflow_path();
    let base_url = format!("http://127.0.0.1:{port}");
    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        &base_url,
    ]);

    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("failed run report");
    assert_eq!(report["status"], "failed");
    assert_eq!(report["steps"][0]["status"], "failed");
    assert!(
        report["steps"][0]["result_preview"]["error"]
            .as_str()
            .is_some_and(|error| error.contains("failed to connect"))
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("headless execution failed at step 1 (service_health)"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("failed to connect"), "stderr: {stderr}");
    let _ = fs::remove_file(path);
}

#[test]
fn unknown_material_study_fails_before_service_connection() {
    let path = workflow_path();
    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--material-report",
        "unknown-study",
        "--material-report-out",
        "/tmp/kyuubiki-unknown-study.json",
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        "http://127.0.0.1:19999",
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unsupported material report study: unknown-study"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("failed to connect"), "stderr: {stderr}");
    let _ = fs::remove_file(path);
}

#[test]
fn missing_material_output_fails_before_service_connection() {
    let path = workflow_path();
    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--material-report",
        "dielectric-screening",
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        "http://127.0.0.1:19999",
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--material-report with --json requires --material-report-out"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("failed to connect"), "stderr: {stderr}");
    let _ = fs::remove_file(path);
}

#[test]
fn material_report_rejects_incompatible_template_before_execution() {
    let path = init_template("direct_plane_quad");
    let material_path = unique_path("incompatible-material-report");
    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--material-report",
        "dielectric-screening",
        "--material-report-out",
        material_path.to_str().expect("material path"),
        "--execute",
        "--executor",
        "mock",
    ]);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).expect("structured CLI error");
    assert_eq!(error["schema_version"], "kyuubiki.headless-cli-error/v1");
    assert_eq!(error["ok"], false);
    assert_eq!(error["error"]["code"], "material_report_template_mismatch");
    assert_eq!(error["error"]["stage"], "material_report_validation");
    assert_eq!(error["error"]["retryable"], false);
    assert!(error["error"]["recommended_action"].is_string());
    assert!(
        error["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("direct_plane_quad"))
    );
    assert!(!material_path.exists());
    let _ = fs::remove_file(path);
}

#[test]
fn material_report_accepts_matching_template() {
    let path = init_template("material_dielectric_screening");
    let material_path = unique_path("matching-material-report");
    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--material-report",
        "dielectric-screening",
        "--material-report-out",
        material_path.to_str().expect("material path"),
        "--execute",
        "--executor",
        "mock",
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("run report");
    assert_eq!(report["status"], "ok");
    let material_report: Value = serde_json::from_slice(
        &fs::read(&material_path).expect("matching material report should be written"),
    )
    .expect("material report json");
    assert_eq!(
        material_report["schema_version"],
        "kyuubiki.dielectric-material-report/v1"
    );
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(material_path);
}

#[test]
fn composite_material_report_accepts_matching_template() {
    let path = init_template("material_composite_thermo_electric_panel_screening");
    let material_path = unique_path("matching-composite-material-report");
    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--material-report",
        "composite-thermo-electric-panel",
        "--material-report-out",
        material_path.to_str().expect("material path"),
        "--execute",
        "--executor",
        "mock",
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("run report");
    assert_eq!(report["status"], "ok");
    let material_report: Value = serde_json::from_slice(
        &fs::read(&material_path).expect("composite material report should be written"),
    )
    .expect("material report json");
    assert_eq!(
        material_report["schema_version"],
        "kyuubiki.composite-panel-report/v1"
    );
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(material_path);
}

#[test]
fn material_report_rejects_workflow_without_template_provenance() {
    let path = workflow_path();
    let material_path = unique_path("missing-template-provenance");
    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--material-report",
        "dielectric-screening",
        "--material-report-out",
        material_path.to_str().expect("material path"),
        "--execute",
        "--executor",
        "mock",
    ]);

    assert!(!output.status.success());
    let error: Value = serde_json::from_slice(&output.stderr).expect("structured CLI error");
    assert_eq!(
        error["error"]["code"],
        "material_report_template_provenance_missing"
    );
    assert!(!material_path.exists());
    let _ = fs::remove_file(path);
}
