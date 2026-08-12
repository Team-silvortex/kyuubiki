use serde_json::{Value, json};
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn workflow_path() -> PathBuf {
    let unique = unique_suffix();
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
    std::env::temp_dir().join(format!(
        "kyuubiki-headless-{label}-{}.json",
        unique_suffix()
    ))
}

fn unique_suffix() -> String {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{}-{unique}-{sequence}", std::process::id())
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
    let report: Value = serde_json::from_slice(&output.stdout).expect("preflight run report");
    assert_eq!(
        report["schema_version"],
        "kyuubiki.headless-execution-run/v1"
    );
    assert_eq!(report["status"], "invalid");
    assert_eq!(
        report["execution_summary"]["failure"]["error_code"],
        "kyuubiki.headless.endpoint_configuration"
    );
    let _ = fs::remove_file(path);
}

#[test]
fn service_execution_failure_returns_nonzero_with_root_cause() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve local port");
    let port = listener.local_addr().expect("local address").port();
    let closer = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept health request");
        drop(stream);
    });
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
    closer.join().expect("health failure server should exit");
    let report: Value = serde_json::from_slice(&output.stdout).expect("failed run report");
    assert_eq!(report["status"], "failed");
    assert_eq!(report["steps"][0]["status"], "failed");
    let root_cause = report["steps"][0]["result_preview"]["error"]
        .as_str()
        .expect("failed service step should retain its root cause");
    assert!(!root_cause.trim().is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("headless execution failed at step 1 (service_health)"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains(root_cause), "stderr: {stderr}");
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
    let report: Value = serde_json::from_slice(&output.stdout).expect("preflight run report");
    assert_eq!(report["status"], "invalid");
    assert_eq!(
        report["execution_summary"]["failure"]["error_code"],
        "kyuubiki.headless.material_report_template_mismatch"
    );
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
fn material_report_rejects_dielectric_unit_drift_before_execution() {
    let path = init_template("material_dielectric_screening");
    let material_path = unique_path("drifted-dielectric-material-report");
    let mut workflow: Value = serde_json::from_slice(&fs::read(&path).expect("template workflow"))
        .expect("template workflow json");
    workflow["workflow"]["steps"][0]["payload"]["model"]["elements"][0]["permittivity"] =
        json!(4.7);
    fs::write(
        &path,
        serde_json::to_vec_pretty(&workflow).expect("drifted workflow json"),
    )
    .expect("write drifted workflow");

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
    let report: Value = serde_json::from_slice(&output.stdout).expect("preflight report");
    assert_eq!(report["status"], "invalid");
    assert_eq!(report["executed_step_count"], 0);
    let error: Value = serde_json::from_slice(&output.stderr).expect("structured CLI error");
    assert_eq!(
        error["error"]["code"],
        "material_report_input_contract_mismatch"
    );
    assert!(!material_path.exists());
    let _ = fs::remove_file(path);
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
    let report: Value = serde_json::from_slice(&output.stdout).expect("preflight run report");
    assert_eq!(report["status"], "invalid");
    assert_eq!(
        report["execution_summary"]["failure"]["error_code"],
        "kyuubiki.headless.material_report_template_provenance_missing"
    );
    let error: Value = serde_json::from_slice(&output.stderr).expect("structured CLI error");
    assert_eq!(
        error["error"]["code"],
        "material_report_template_provenance_missing"
    );
    assert!(!material_path.exists());
    let _ = fs::remove_file(path);
}

#[test]
fn executor_compatibility_failure_writes_a_standard_run_report() {
    let path = init_template("browser_capture_review");
    let report_path = unique_path("executor-compatibility-report");
    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--report-out",
        report_path.to_str().expect("report path"),
        "--execute",
        "--executor",
        "service",
    ]);

    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("preflight run report");
    assert_eq!(
        report["schema_version"],
        "kyuubiki.headless-execution-run/v1"
    );
    assert_eq!(report["mode"], "execute:service");
    assert_eq!(report["status"], "invalid");
    assert_eq!(
        report["execution_summary"]["failure"]["error_code"],
        "kyuubiki.headless.executor_compatibility"
    );
    assert!(
        report["validation"]["issues"]
            .as_array()
            .is_some_and(|issues| issues.iter().any(|issue| issue
                .as_str()
                .is_some_and(|text| text.contains("not compatible with executor service"))))
    );
    let persisted: Value =
        serde_json::from_slice(&fs::read(&report_path).expect("persisted preflight report"))
            .expect("persisted report json");
    assert_eq!(persisted, report);
    let error: Value = serde_json::from_slice(&output.stderr).expect("structured CLI error");
    assert_eq!(error["error"]["code"], "executor_compatibility");
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(report_path);
}

#[test]
fn contract_invalid_batch_fails_before_mock_execution() {
    let path = unique_path("contract-invalid-batch");
    let batch = json!({
        "schema_version": "kyuubiki.headless-execution-batch/v1",
        "exported_at": "2026-08-10T00:00:00Z",
        "language": "en",
        "workflow_id": "workflow.contract-invalid",
        "steps": [{
            "index": 1,
            "action": "project_create",
            "risk": "normal",
            "payload": {}
        }]
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&batch).expect("batch json"),
    )
    .expect("write invalid batch");

    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--execute",
        "--executor",
        "mock",
    ]);

    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("run report");
    assert_eq!(report["status"], "invalid");
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(report["steps"], json!([]));
    assert_eq!(
        report["execution_summary"]["failure"]["stage"],
        "batch_validation"
    );
    let _ = fs::remove_file(path);
}

#[test]
fn run_applies_auditable_job_wait_timeout_override() {
    let path = init_template("direct_mesh_pipeline");
    let original = fs::read(&path).expect("source workflow");
    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--job-wait-timeout-ms",
        "1200000",
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
    let wait = report["steps"]
        .as_array()
        .and_then(|steps| steps.iter().find(|step| step["action"] == "job_wait"))
        .expect("job_wait report");
    assert_eq!(wait["payload"]["timeout_ms"], 1_200_000);
    assert_eq!(wait["payload"]["max_total_timeout_ms"], 3_600_000);
    assert!(
        report["validation"]["warnings"][0]
            .as_str()
            .is_some_and(|warning| warning.contains("overrode 1 job_wait"))
    );
    assert_eq!(
        fs::read(&path).expect("source workflow after run"),
        original
    );
    let _ = fs::remove_file(path);
}

#[test]
fn thermal_frame_constraint_conflict_fails_before_service_connection() {
    let path = init_template("direct_thermal_frame_3d");
    let mut workflow: Value = serde_json::from_slice(&fs::read(&path).expect("template workflow"))
        .expect("template workflow json");
    workflow["workflow"]["steps"][0]["payload"]["model"]["nodes"][1]["fix_y"] = json!(true);
    fs::write(
        &path,
        serde_json::to_vec_pretty(&workflow).expect("conflicted workflow json"),
    )
    .expect("write conflicted workflow");

    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        "http://127.0.0.1:19999",
    ]);

    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("validation report");
    assert_eq!(report["status"], "invalid");
    assert_eq!(report["executed_step_count"], 0);
    assert!(
        report["validation"]["issues"]
            .as_array()
            .is_some_and(|issues| {
                issues.iter().any(|issue| {
                    issue
                        .as_str()
                        .is_some_and(|text| text.contains("non-zero load_y to fixed y degree"))
                })
            })
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("failed to connect"));
    let _ = fs::remove_file(path);
}

#[test]
fn malformed_batch_writes_a_standard_document_validation_report() {
    let path = unique_path("missing-action-batch");
    let report_path = unique_path("missing-action-report");
    let batch = json!({
        "schema_version": "kyuubiki.headless-execution-batch/v1",
        "exported_at": "2026-08-10T00:00:00Z",
        "language": "en",
        "workflow_id": "workflow.missing-action",
        "steps": [{"index": 1, "risk": "normal", "payload": {}}]
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&batch).expect("batch json"),
    )
    .expect("write malformed batch");
    let output = run(&[
        "run",
        path.to_str().expect("path"),
        "--json",
        "--report-out",
        report_path.to_str().expect("report path"),
        "--execute",
        "--executor",
        "mock",
    ]);

    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("preflight run report");
    assert_eq!(report["workflow_id"], "workflow.missing-action");
    assert_eq!(report["status"], "invalid");
    assert_eq!(
        report["execution_summary"]["failure"]["error_code"],
        "kyuubiki.headless.document_validation"
    );
    assert!(
        report["validation"]["issues"][0]
            .as_str()
            .is_some_and(|issue| issue.contains("missing field `action`"))
    );
    let persisted: Value =
        serde_json::from_slice(&fs::read(&report_path).expect("persisted preflight report"))
            .expect("persisted report json");
    assert_eq!(persisted, report);
    let error: Value = serde_json::from_slice(&output.stderr).expect("structured CLI error");
    assert_eq!(error["error"]["code"], "document_validation");
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(report_path);
}
