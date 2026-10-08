use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::model_artifact::{physical_result, solve_batch};
use super::model_reference_gates::model;
use super::orchestra_completion::http_json;
use super::write_ack_loss::count;
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessRunReport, ServiceHeadlessExecutor,
    execute_batch_with_executor, find_action_contract,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn padded(mut input: Value) -> Value {
    // Cross the real upload threshold without persisting a large physics fixture.
    input["transport_padding"] = json!("x".repeat(8_000_000));
    input
}

fn guarded(input: Value) -> HeadlessExecutionBatch {
    let mut batch = solve_batch(input);
    batch.steps.truncate(2);
    batch.steps.push(HeadlessExecutionBatchStep {
        action: "project_create".into(),
        index: 3,
        risk: find_action_contract("project_create").unwrap().risk,
        payload: json!({"name":"forbidden-after-invalid-bar"}),
    });
    batch
}

fn run_cli(batch: &HeadlessExecutionBatch, url: &str, success: bool) -> Result<HeadlessRunReport> {
    let source = write_temp_json("axial-bar-normalization", &serde_json::to_value(batch)?);
    let path = source.parent().unwrap().join("report.json");
    let output = run_headless_command(&[
        "run",
        source.to_str().unwrap(),
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        url,
        "--json",
        "--report-out",
        path.to_str().unwrap(),
    ]);
    assert_eq!(output.status.success(), success, "{output:?}");
    let report = parse_json_output(&output);
    assert_eq!(report, serde_json::from_slice::<Value>(&fs::read(path)?)?);
    if !success {
        let failure: Value = serde_json::from_slice(&output.stderr)?;
        assert_eq!(failure["error"]["code"], "headless_execution_failed");
        assert_eq!(failure["error"]["retryable"], false);
    }
    Ok(serde_json::from_value(report)?)
}

fn assert_physics(result: &Value, force: f64) {
    let displacement = force / (210e9 * 0.01);
    for (field, expected) in [
        ("tip_displacement", displacement),
        ("max_stress", force.abs() / 0.01),
        ("reaction_force", -force),
        ("total_strain_energy", 0.5 * force * displacement),
    ] {
        let actual = result[field].as_f64().unwrap();
        assert!(
            (actual / expected - 1.0).abs() < 1e-12,
            "{field}: {actual} != {expected}"
        );
    }
    assert_eq!(result["input"]["youngs_modulus"], 210e9);
    assert!(result["input"].get("youngs_modulus_gpa").is_none());
    assert_eq!(result["input"]["elements"], 4);
    assert_eq!(result["nodes"].as_array().unwrap().len(), 5);
    for element in result["elements"].as_array().unwrap() {
        assert!((element["axial_force"].as_f64().unwrap() / force - 1.0).abs() < 1e-12);
    }
}

fn assert_failed(report: &HeadlessRunReport, artifact: bool) {
    assert_eq!(report.status, "failed", "{report:?}");
    assert_eq!(report.executed_step_count, usize::from(artifact));
    assert_eq!(report.steps.len(), if artifact { 2 } else { 1 });
    let failure = report.execution_summary.failure.as_ref().unwrap();
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
    assert_eq!(failure.step_index, if artifact { 2 } else { 1 });
    if artifact {
        assert_eq!(
            failure.error_code, "kyuubiki.headless.invalid_solver_params",
            "{report:?}"
        );
        assert_eq!(failure.stage, "agent_decode");
    }
}

#[test]
fn real_axial_bar_inline_and_artifact_normalization_agree_and_reject_without_downstream_writes()
-> Result<()> {
    let mut owner = LiveAgent::start()?;
    let mut peer = LiveAgent::start()?;
    let state = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&state.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    owner.restart_with_orchestrator(&url)?;
    peer.restart_with_orchestrator(&url)?;
    let mut executor = ServiceHeadlessExecutor::new(&url);
    let mut expected_admissions = 0;
    for (name, patch, force) in [
        ("gpa-only", json!({}), 1000.0),
        ("pa-only", json!({"youngs_modulus":"2.1e11"}), 1000.0),
        (
            "dual-roundoff",
            json!({"youngs_modulus":210000000000.00003}),
            1000.0,
        ),
        (
            "numeric-strings",
            json!({"length":"1e0","area":"0.01","elements":"4.0", "tip_force":"-1e3","youngs_modulus_gpa":"2.1e2"}),
            -1000.0,
        ),
    ] {
        let mut input = model(force);
        if name == "pa-only" {
            input.as_object_mut().unwrap().remove("youngs_modulus_gpa");
        }
        input
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let inline =
            execute_batch_with_executor(&solve_batch(input.clone()), &mut executor, false, false);
        assert_eq!(inline.status, "ok", "{name}: {inline:?}\n{}", server.logs());
        assert!(inline.steps[0].result_preview["model_artifact_upload"].is_null());
        let large = padded(input);
        let source_bytes = serde_json::to_vec(&large)?;
        let batch = solve_batch(large);
        let artifact = execute_batch_with_executor(&batch, &mut executor, false, false);
        assert_eq!(
            artifact.status,
            "ok",
            "{name}: {artifact:?}\n{}",
            server.logs()
        );
        let upload = &artifact.steps[0].result_preview["model_artifact_upload"];
        assert_eq!(
            upload["sha256"],
            format!("{:x}", Sha256::digest(&source_bytes))
        );
        assert_eq!(upload["size_bytes"], source_bytes.len());
        assert_eq!(upload["immutable"], true);
        let inline_result = physical_result(&inline);
        let artifact_result = physical_result(&artifact);
        assert_eq!(inline_result, artifact_result, "{name}");
        assert_physics(&artifact_result, force);
        let retained = serde_json::to_vec(&json!({"batch":batch,"report":artifact}))?;
        let restored: Value = serde_json::from_slice(&retained)?;
        assert_eq!(restored["batch"], serde_json::to_value(&batch)?);
        assert_eq!(restored["report"], serde_json::to_value(&artifact)?);
        let restored_report: HeadlessRunReport =
            serde_json::from_value(restored["report"].clone())?;
        assert_eq!(physical_result(&restored_report), artifact_result);
        expected_admissions += 2;
    }
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    for patch in [json!({"youngs_modulus":200e9}), json!({"elements":4.5})] {
        let mut input = model(1000.0);
        input
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let small = guarded(input.clone());
        let rejected = execute_batch_with_executor(&small, &mut executor, false, false);
        assert_failed(&rejected, false);
        assert_failed(&run_cli(&small, &url, false)?, false);
        assert_eq!(
            http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
                .as_array()
                .unwrap()
                .len(),
            expected_admissions
        );
        assert_eq!(count(&owner)? + count(&peer)?, expected_admissions as u64);
        let large = guarded(padded(input));
        let rejected = execute_batch_with_executor(&large, &mut executor, false, false);
        assert_failed(&rejected, true);
        let cli = run_cli(&large, &url, false)?;
        assert_failed(&cli, true);
        for report in [&rejected, &cli] {
            let id = report.steps[0].result_preview["job_id"].as_str().unwrap();
            let job = http_json(server.port, &format!("/api/v1/jobs/{id}"), None)?.1;
            assert_eq!(job["job"]["status"], "failed", "{job}");
            assert_eq!(job["job"]["has_result"], false);
            assert!(
                job["job"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("invalid_params")
            );
        }
        expected_admissions += 2;
        assert_eq!(
            http_json(server.port, "/api/v1/projects", None)?.1,
            projects
        );
    }
    let healthy = run_cli(&solve_batch(padded(model(1000.0))), &url, true)?;
    assert_physics(&physical_result(&healthy), 1000.0);
    expected_admissions += 1;
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, expected_admissions as u64);
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        expected_admissions
    );
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1,
        projects
    );
    Ok(())
}
