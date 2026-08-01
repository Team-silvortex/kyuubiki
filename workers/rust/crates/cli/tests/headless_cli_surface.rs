use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-headless-cli-surface-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("fixture root");
    root
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kyuubiki-headless"))
        .args(args)
        .output()
        .expect("run headless CLI")
}

fn successful(args: &[&str]) -> Output {
    let output = run(args);
    assert!(
        output.status.success(),
        "command failed: {:?}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn native_headless_cli_closes_template_to_dry_run_journey() {
    let root = fixture_root();
    let workflow = root.join("nested/workflow.json");
    let batch = root.join("nested/batch.json");
    let plan = root.join("nested/plan.json");

    let templates = successful(&["templates", "--runtime", "service_only", "--json"]);
    let templates: Value = serde_json::from_slice(&templates.stdout).expect("template JSON");
    assert!(templates["template_count"].as_u64().unwrap_or_default() > 0);

    successful(&[
        "init",
        "--template",
        "solve_wait_result",
        "--workflow-id",
        "workflow.native-surface",
        "--out",
        workflow.to_str().expect("workflow path"),
    ]);
    assert!(workflow.is_file());

    let rendered = successful(&[
        "render",
        workflow.to_str().expect("workflow path"),
        "--out",
        batch.to_str().expect("batch path"),
        "--json",
    ]);
    let rendered: Value = serde_json::from_slice(&rendered.stdout).expect("render JSON");
    assert_eq!(
        rendered["schema_version"],
        "kyuubiki.headless-execution-batch/v1"
    );
    assert_eq!(rendered["workflow_id"], "workflow.native-surface");
    assert!(batch.is_file());

    let validation = successful(&["validate", batch.to_str().expect("batch path"), "--json"]);
    let validation: Value = serde_json::from_slice(&validation.stdout).expect("validation JSON");
    assert_eq!(validation["ok"], true);

    let planned = successful(&[
        "plan",
        batch.to_str().expect("batch path"),
        "--out",
        plan.to_str().expect("plan path"),
        "--json",
    ]);
    let planned: Value = serde_json::from_slice(&planned.stdout).expect("plan JSON");
    assert_eq!(planned["ok"], true);
    assert!(plan.is_file());

    let executed = successful(&["run", batch.to_str().expect("batch path"), "--json"]);
    let executed: Value = serde_json::from_slice(&executed.stdout).expect("run JSON");
    assert_eq!(executed["mode"], "dry_run");
    assert_eq!(executed["validation"]["ok"], true);

    fs::remove_dir_all(root).expect("clean fixture");
}
