use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static SCRATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-output-paths-{}-{stamp}-{}",
            std::process::id(),
            SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let workflow = json!({
            "schema_version":"kyuubiki.headless-workflow/v1", "language":"en",
            "exported_at":"2026-10-07T00:00:00Z",
            "workflow":{"id":"output-paths","steps":[{"action":"service_health","payload":{"input":1.0}}]}
        });
        fs::write(
            path.join("workflow.json"),
            serde_json::to_vec(&workflow).unwrap(),
        )
        .unwrap();
        Self(path)
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

fn assert_path_conflict(output: &Output) {
    assert!(!output.status.success(), "colliding paths were accepted");
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostic["error"]["code"], "output_path_conflict");
    assert_eq!(diagnostic["error"]["stage"], "command_validation");
    assert_eq!(diagnostic["error"]["retryable"], false);
    assert!(
        diagnostic["error"]["recommended_action"]
            .as_str()
            .unwrap()
            .contains("do not replay")
    );
}

const RUN_OUTPUTS: [&str; 4] = [
    "--report-out",
    "--material-report-out",
    "--research-round-out",
    "--parameter-patch-receipt-out",
];

fn write_patch(root: &Path) {
    fs::write(
        root.join("patch.json"),
        serde_json::to_vec(&json!({
            "schema_version":"kyuubiki.headless-parameter-patch/v1",
            "patch_id":"output-paths-patch", "workflow_id":"output-paths",
            "changes":[{"path":"/steps/0/payload/input", "expected":1.0,"value":2.0}]
        }))
        .unwrap(),
    )
    .unwrap();
}

fn assert_zero_step_report(output: &Output) {
    assert_path_conflict(output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "invalid");
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(report["steps"], json!([]));
    assert_eq!(
        report["execution_summary"]["failure"]["stage"],
        "command_validation"
    );
}

#[test]
fn run_report_cannot_replace_its_source_workflow_even_with_a_different_spelling() {
    let scratch = Scratch::new();
    let original = fs::read(scratch.0.join("workflow.json")).unwrap();
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
            "./workflow.json",
        ],
    );
    assert_path_conflict(&output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "invalid");
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(report["steps"], json!([]));
    assert_eq!(fs::read(scratch.0.join("workflow.json")).unwrap(), original);
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
}

#[test]
fn duplicate_run_and_material_output_paths_are_rejected_before_creating_files() {
    let scratch = Scratch::new();
    let output = run(
        &scratch.0,
        &[
            "run",
            "workflow.json",
            "--json",
            "--report-out",
            "new/reports.json",
            "--material-report-out",
            "new/./reports.json",
        ],
    );
    assert_path_conflict(&output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["executed_step_count"], 0);
    assert!(!scratch.0.join("new").exists());
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
}

#[test]
fn output_files_cannot_also_be_another_outputs_parent_directory() {
    for (first, second) in [
        ("new/report.json", "new/report.json/material.json"),
        ("new/report.json/material.json", "new/report.json"),
    ] {
        let scratch = Scratch::new();
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
                first,
                "--material-report-out",
                second,
            ],
        );
        assert_zero_step_report(&output);
        assert!(!scratch.0.join("new").exists());
        assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
    }
}

#[test]
fn every_run_output_pair_is_reserved_before_parameter_receipt_publication() {
    for (index, left) in RUN_OUTPUTS.iter().enumerate() {
        for right in &RUN_OUTPUTS[index + 1..] {
            let scratch = Scratch::new();
            write_patch(&scratch.0);
            let output = run(
                &scratch.0,
                &[
                    "run",
                    "workflow.json",
                    "--json",
                    "--parameter-patch",
                    "patch.json",
                    left,
                    "new/shared.json",
                    right,
                    "new/./shared.json",
                ],
            );
            assert_zero_step_report(&output);
            assert!(!scratch.0.join("new").exists(), "{left} / {right}");
            assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 2);
        }
    }
}

#[test]
fn every_run_output_protects_all_declared_inputs_and_preserves_existing_reports() {
    for input in ["workflow.json", "patch.json", "spec.json", "previous.json"] {
        for output_flag in RUN_OUTPUTS {
            let scratch = Scratch::new();
            write_patch(&scratch.0);
            // These two fixtures need not decode: the path guard precedes research validation.
            fs::write(scratch.0.join("spec.json"), b"round spec sentinel").unwrap();
            fs::write(
                scratch.0.join("previous.json"),
                b"previous evidence sentinel",
            )
            .unwrap();
            fs::write(
                scratch.0.join("safe-report.json"),
                b"previous run report sentinel",
            )
            .unwrap();
            let protected: Vec<_> = [
                "workflow.json",
                "patch.json",
                "spec.json",
                "previous.json",
                "safe-report.json",
            ]
            .map(|name| (name, fs::read(scratch.0.join(name)).unwrap()))
            .into();
            let mut args = vec![
                "run",
                "workflow.json",
                "--execute",
                "--executor",
                "service",
                "--json",
                "--parameter-patch",
                "patch.json",
                "--research-round-spec",
                "spec.json",
                "--previous-round-evidence",
                "previous.json",
            ];
            if output_flag != "--report-out" {
                args.extend(["--report-out", "safe-report.json"]);
            }
            args.extend([output_flag, input]);
            let output = run(&scratch.0, &args);
            assert_zero_step_report(&output);
            for (name, original) in protected {
                assert_eq!(
                    fs::read(scratch.0.join(name)).unwrap(),
                    original,
                    "{output_flag} -> {input}: {name}"
                );
            }
            assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 5);
        }
    }
}

#[test]
fn malformed_workflow_is_not_replaced_by_its_own_preflight_failure_report() {
    let scratch = Scratch::new();
    let original = b"{ malformed workflow";
    fs::write(scratch.0.join("workflow.json"), original).unwrap();
    let output = run(
        &scratch.0,
        &[
            "run",
            "workflow.json",
            "--json",
            "--report-out",
            "workflow.json",
        ],
    );
    assert_zero_step_report(&output);
    assert_eq!(fs::read(scratch.0.join("workflow.json")).unwrap(), original);
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
}

#[test]
fn render_and_plan_cannot_replace_the_workflow_patch_or_their_own_patch_receipt() {
    for command in ["render", "plan"] {
        for target in ["./workflow.json", "patch.json", "new/receipt.json"] {
            let scratch = Scratch::new();
            write_patch(&scratch.0);
            let original = fs::read(scratch.0.join("workflow.json")).unwrap();
            let patch = fs::read(scratch.0.join("patch.json")).unwrap();
            let output = run(
                &scratch.0,
                &[
                    command,
                    "workflow.json",
                    "--json",
                    "--parameter-patch",
                    "patch.json",
                    "--parameter-patch-receipt-out",
                    "new/receipt.json",
                    "--out",
                    target,
                ],
            );
            assert_path_conflict(&output);
            assert!(output.stdout.is_empty());
            assert_eq!(fs::read(scratch.0.join("workflow.json")).unwrap(), original);
            assert_eq!(fs::read(scratch.0.join("patch.json")).unwrap(), patch);
            assert!(!scratch.0.join("new").exists());
        }
    }
}

#[test]
fn parameter_receipt_cannot_replace_either_input_in_any_consuming_command() {
    for command in ["inspect", "validate", "render", "plan", "run"] {
        for target in ["workflow.json", "patch.json"] {
            let scratch = Scratch::new();
            write_patch(&scratch.0);
            let original = fs::read(scratch.0.join("workflow.json")).unwrap();
            let patch = fs::read(scratch.0.join("patch.json")).unwrap();
            let output = run(
                &scratch.0,
                &[
                    command,
                    "workflow.json",
                    "--json",
                    "--parameter-patch",
                    "patch.json",
                    "--parameter-patch-receipt-out",
                    target,
                ],
            );
            assert_path_conflict(&output);
            if command == "run" {
                assert_zero_step_report(&output);
            }
            assert_eq!(fs::read(scratch.0.join("workflow.json")).unwrap(), original);
            assert_eq!(fs::read(scratch.0.join("patch.json")).unwrap(), patch);
            assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 2);
        }
    }
}

#[test]
fn distinct_transform_outputs_and_receipts_remain_supported() {
    for command in ["render", "plan"] {
        let scratch = Scratch::new();
        write_patch(&scratch.0);
        let original = fs::read(scratch.0.join("workflow.json")).unwrap();
        fs::write(scratch.0.join("output.json"), b"old output").unwrap();
        let output = run(
            &scratch.0,
            &[
                command,
                "workflow.json",
                "--json",
                "--parameter-patch",
                "patch.json",
                "--parameter-patch-receipt-out",
                "receipt.json",
                "--out",
                "output.json",
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            value,
            serde_json::from_slice::<Value>(&fs::read(scratch.0.join("output.json")).unwrap())
                .unwrap()
        );
        let receipt: Value =
            serde_json::from_slice(&fs::read(scratch.0.join("receipt.json")).unwrap()).unwrap();
        assert_eq!(receipt["patch_id"], "output-paths-patch");
        assert_eq!(fs::read(scratch.0.join("workflow.json")).unwrap(), original);
        assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 4);
    }
}

#[cfg(unix)]
#[test]
fn hard_link_aliases_are_rejected_for_both_inputs_and_output_pairs() {
    for target in ["workflow.json", "existing-report.json"] {
        let scratch = Scratch::new();
        fs::write(scratch.0.join("existing-report.json"), b"retained report").unwrap();
        fs::hard_link(scratch.0.join(target), scratch.0.join("alias.json")).unwrap();
        let original = fs::read(scratch.0.join(target)).unwrap();
        let mut args = vec![
            "run",
            "workflow.json",
            "--json",
            "--report-out",
            "alias.json",
        ];
        if target == "existing-report.json" {
            args.extend(["--material-report-out", target]);
        }
        let output = run(&scratch.0, &args);
        assert_zero_step_report(&output);
        assert_eq!(fs::read(scratch.0.join(target)).unwrap(), original);
        assert_eq!(fs::read(scratch.0.join("alias.json")).unwrap(), original);
    }
}

#[cfg(unix)]
#[test]
fn symlink_parent_is_resolved_before_dotdot_when_protecting_sources() {
    let scratch = Scratch::new();
    fs::create_dir(scratch.0.join("nested")).unwrap();
    std::os::unix::fs::symlink(scratch.0.join("nested"), scratch.0.join("link")).unwrap();
    let original = fs::read(scratch.0.join("workflow.json")).unwrap();
    let output = run(
        &scratch.0,
        &[
            "run",
            "workflow.json",
            "--json",
            "--report-out",
            "link/../workflow.json",
        ],
    );
    assert_zero_step_report(&output);
    assert_eq!(fs::read(scratch.0.join("workflow.json")).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn alias_created_during_service_execution_preserves_completed_stdout_and_source() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};
    let scratch = Scratch::new();
    let source = scratch.0.join("workflow.json");
    let destination = scratch.0.join("late-report.json");
    let original = fs::read(&source).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (finished, completion) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "service request did not arrive");
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("accept service request: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = [0; 4096];
        let length = stream.read(&mut request).unwrap();
        assert!(request[..length].starts_with(b"GET /api/health "));
        fs::hard_link(source, destination).unwrap();
        let body = r#"{"service":"late-alias-fixture","status":"ok"}"#;
        write!(stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()
        ).unwrap();
        drop(stream);
        let mut request_count = 1;
        loop {
            match listener.accept() {
                Ok((mut extra, _)) => {
                    request_count += 1;
                    extra
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    extra
                        .set_write_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let length = extra.read(&mut request).unwrap();
                    assert!(request[..length].starts_with(b"GET /api/health "));
                    write!(
                        extra,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .unwrap();
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if completion.try_recv().is_ok() {
                        break;
                    }
                    assert!(Instant::now() < deadline, "CLI completion did not arrive");
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("accept additional request: {error}"),
            }
        }
        request_count
    });
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
            &url,
            "--report-out",
            "late-report.json",
        ],
    );
    finished.send(()).unwrap();
    assert_eq!(
        server.join().unwrap(),
        1,
        "output conflict replayed the service request"
    );
    assert_path_conflict(&output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "ok");
    assert_eq!(report["executed_step_count"], 1);
    assert_eq!(report["steps"][0]["status"], "executed");
    assert!(report["execution_summary"]["failure"].is_null());
    assert_eq!(fs::read(scratch.0.join("workflow.json")).unwrap(), original);
    assert_eq!(
        fs::read(scratch.0.join("late-report.json")).unwrap(),
        original
    );
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 2);
}
