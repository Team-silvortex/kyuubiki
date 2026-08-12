use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
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
        "kyuubiki-headless-parameter-patch-{}-{nanos}-{sequence}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("workspace");
    path
}

fn init_workflow(root: &Path) -> PathBuf {
    let workflow = root.join("workflow.json");
    let output = run(&[
        "init",
        "--template",
        "direct_thermal_frame_3d",
        "--out",
        workflow.to_str().expect("workflow path"),
    ]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    workflow
}

fn write_patch(root: &Path, expected: Value, value: Value) -> PathBuf {
    let path = root.join("round-2.patch.json");
    let patch = json!({
        "schema_version": "kyuubiki.headless-parameter-patch/v1",
        "patch_id": "thermal-load-round-2",
        "workflow_id": "template.direct_thermal_frame_3d",
        "template_id": "direct_thermal_frame_3d",
        "changes": [{
            "path": "/steps/0/payload/model/nodes/1/load_y",
            "expected": expected,
            "value": value
        }]
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&patch).expect("patch json"),
    )
    .expect("write patch");
    path
}

fn json_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout should be JSON: {error}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn guarded_patch_is_shared_by_inspect_validate_render_plan_and_run() {
    let root = workspace();
    let workflow = init_workflow(&root);
    let patch = write_patch(&root, json!(-1000.0), json!(-1250.0));
    let workflow = workflow.to_str().expect("workflow path");
    let patch = patch.to_str().expect("patch path");

    for command in ["inspect", "validate", "plan"] {
        let output = run(&[command, workflow, "--parameter-patch", patch, "--json"]);
        assert!(
            output.status.success(),
            "{command} stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let payload = json_stdout(&output);
        if command == "inspect" || command == "validate" {
            assert_eq!(payload["warning_count"], 1);
        }
    }

    let render = run(&["render", workflow, "--parameter-patch", patch, "--json"]);
    assert!(render.status.success());
    let rendered = json_stdout(&render);
    assert_eq!(
        rendered["steps"][0]["payload"]["model"]["nodes"][1]["load_y"],
        -1250.0
    );
    assert!(
        rendered["warnings"][0]
            .as_str()
            .is_some_and(|warning| warning.contains("before_sha256="))
    );

    let execution = run(&["run", workflow, "--parameter-patch", patch, "--json"]);
    assert!(execution.status.success());
    let report = json_stdout(&execution);
    assert_eq!(report["status"], "ok");
    assert_eq!(report["validation"]["warning_count"], 1);
    assert_eq!(
        report["steps"][0]["payload"]["model"]["nodes"][1]["load_y"],
        -1250.0
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stale_patch_fails_closed_with_a_machine_actionable_run_report() {
    let root = workspace();
    let workflow = init_workflow(&root);
    let patch = write_patch(&root, json!(-900.0), json!(-1250.0));
    let report_path = root.join("failed-run.json");
    let output = run(&[
        "run",
        workflow.to_str().expect("workflow path"),
        "--parameter-patch",
        patch.to_str().expect("patch path"),
        "--json",
        "--report-out",
        report_path.to_str().expect("report path"),
    ]);

    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&fs::read(&report_path).expect("failure report"))
        .expect("report json");
    assert_eq!(report["status"], "invalid");
    assert_eq!(
        report["execution_summary"]["failure"]["error_code"],
        "kyuubiki.headless.parameter_patch_validation"
    );
    assert_eq!(
        report["execution_summary"]["failure"]["stage"],
        "parameter_patch"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("baseline mismatch"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = fs::remove_dir_all(root);
}
