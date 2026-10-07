use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static SCRATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self::with_stamp(unique)
    }

    fn with_stamp(unique: u128) -> Self {
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-headless-publication-{}-{unique}-{}",
            std::process::id(),
            SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap_or_else(|error| {
            panic!("create independent scratch {}: {error}", path.display())
        });
        Self(path)
    }
}

#[test]
fn repeated_timestamp_does_not_share_scratch_or_cleanup_between_tests() {
    let first = Scratch::with_stamp(1);
    let second = Scratch::with_stamp(1);
    assert_ne!(first.0, second.0);
    fs::write(first.0.join("sentinel.json"), b"first").unwrap();
    fs::write(second.0.join("sentinel.json"), b"second").unwrap();
    let first_path = first.0.clone();
    drop(first);
    assert!(!first_path.exists());
    assert_eq!(fs::read(second.0.join("sentinel.json")).unwrap(), b"second");
    assert_eq!(fs::read_dir(&second.0).unwrap().count(), 1);
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

fn workflow(root: &Path) {
    let value = json!({
        "schema_version":"kyuubiki.headless-workflow/v1",
        "language":"en", "exported_at":"2026-10-07T00:00:00Z",
        "workflow":{"id":"output-publication", "steps":[{
            "action":"service_health", "payload": {
                "notes": (0..64).map(|_| "\u{7814}\u{7a76}\n\"\\".repeat(128)).collect::<Vec<_>>()
            }
        }]}
    });
    fs::write(
        root.join("workflow.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
}

fn assert_output_failure(output: &Output) -> Value {
    assert!(!output.status.success());
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        diagnostic["schema_version"],
        "kyuubiki.headless-cli-error/v1"
    );
    assert_eq!(diagnostic["error"]["code"], "report_output_failure");
    assert_eq!(diagnostic["error"]["stage"], "artifact_output");
    assert_eq!(diagnostic["error"]["retryable"], false);
    assert!(
        diagnostic["error"]["recommended_action"]
            .as_str()
            .unwrap()
            .contains("do not replay")
    );
    diagnostic
}

#[test]
fn relative_unicode_paths_publish_identical_file_and_stdout_json_without_sidecars() {
    let scratch = Scratch::new();
    workflow(&scratch.0);
    let out = "nested/\u{7814}\u{7a76}.json";
    let output = run(
        &scratch.0,
        &["run", "workflow.json", "--json", "--report-out", out],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "ok");
    let bytes = fs::read(scratch.0.join(out)).unwrap();
    assert!(bytes.len() > 64 * 1024);
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), report);
    let mut expected = bytes;
    expected.push(b'\n');
    assert!(output.stdout == expected, "stdout and file bytes differ");
    assert_eq!(fs::read_dir(scratch.0.join("nested")).unwrap().count(), 1);
}

#[test]
fn report_file_failure_retains_the_successful_dry_run_receipt_on_stdout() {
    let scratch = Scratch::new();
    workflow(&scratch.0);
    fs::create_dir(scratch.0.join("report.json")).unwrap();
    let output = run(
        &scratch.0,
        &[
            "run",
            "workflow.json",
            "--json",
            "--report-out",
            "report.json",
        ],
    );
    assert_output_failure(&output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "ok");
    assert_eq!(report["mode"], "dry_run");
    assert!(scratch.0.join("report.json").is_dir());
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 2);
}

#[test]
fn preflight_report_output_failure_keeps_the_original_validation_receipt() {
    let scratch = Scratch::new();
    fs::write(scratch.0.join("invalid.json"), b"{not json").unwrap();
    fs::create_dir(scratch.0.join("report.json")).unwrap();
    let output = run(
        &scratch.0,
        &[
            "run",
            "invalid.json",
            "--json",
            "--report-out",
            "report.json",
        ],
    );
    assert_output_failure(&output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "invalid");
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(
        report["execution_summary"]["failure"]["error_code"],
        "kyuubiki.headless.document_validation"
    );
    assert!(
        report["execution_summary"]["failure"]["message"]
            .as_str()
            .unwrap()
            .contains("failed to parse")
    );
}

#[test]
fn material_output_failure_keeps_the_completed_run_and_its_already_published_report() {
    let scratch = Scratch::new();
    let initialized = run(
        &scratch.0,
        &[
            "init",
            "--template",
            "material_dielectric_screening",
            "--out",
            "workflow.json",
        ],
    );
    assert!(
        initialized.status.success(),
        "status={} stdout={} stderr={}",
        initialized.status,
        String::from_utf8_lossy(&initialized.stdout),
        String::from_utf8_lossy(&initialized.stderr)
    );
    fs::create_dir(scratch.0.join("material.json")).unwrap();
    let output = run(
        &scratch.0,
        &[
            "run",
            "workflow.json",
            "--execute",
            "--executor",
            "mock",
            "--json",
            "--report-out",
            "report.json",
            "--material-report",
            "dielectric-screening",
            "--material-report-out",
            "material.json",
        ],
    );
    assert_output_failure(&output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "ok");
    assert!(report["executed_step_count"].as_u64().unwrap() > 0);
    assert_eq!(
        report,
        serde_json::from_slice::<Value>(&fs::read(scratch.0.join("report.json")).unwrap()).unwrap()
    );
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 3);
}

#[test]
fn closed_json_stdout_returns_a_structured_output_error_instead_of_panicking() {
    let scratch = Scratch::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_kyuubiki-headless"))
        .current_dir(&scratch.0)
        .args(["templates", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    assert_output_failure(&output);
}

#[cfg(unix)]
#[test]
fn parameter_receipt_symlink_failure_stops_before_any_service_request() {
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new();
    workflow(&scratch.0);
    let patch = json!({
        "schema_version":"kyuubiki.headless-parameter-patch/v1",
        "patch_id":"output-path-patch", "workflow_id":"output-publication",
        "changes":[{"path":"/steps/0/payload/notes/0", "expected":"\u{7814}\u{7a76}\n\"\\".repeat(128), "value":"changed"}]
    });
    fs::write(
        scratch.0.join("patch.json"),
        serde_json::to_vec(&patch).unwrap(),
    )
    .unwrap();
    fs::write(scratch.0.join("original.json"), b"{\"original\":true}").unwrap();
    symlink("original.json", scratch.0.join("receipt.json")).unwrap();
    let output = run(
        &scratch.0,
        &[
            "run",
            "workflow.json",
            "--execute",
            "--executor",
            "service",
            "--json",
            "--api-base-url",
            "http://127.0.0.1:19999",
            "--parameter-patch",
            "patch.json",
            "--parameter-patch-receipt-out",
            "receipt.json",
            "--report-out",
            "report.json",
        ],
    );
    let diagnostic = assert_output_failure(&output);
    assert!(
        !diagnostic["error"]["message"]
            .as_str()
            .unwrap()
            .contains("failed to connect")
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "invalid");
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(
        fs::read(scratch.0.join("original.json")).unwrap(),
        b"{\"original\":true}"
    );
    let failure = &report["execution_summary"]["failure"];
    assert_eq!(
        failure["error_code"],
        "kyuubiki.headless.report_output_failure"
    );
    assert_eq!(failure["stage"], "artifact_output");
    assert!(
        failure["recommended_action"]
            .as_str()
            .unwrap()
            .contains("do not replay")
    );
    assert!(
        fs::symlink_metadata(scratch.0.join("receipt.json"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 5);
}
