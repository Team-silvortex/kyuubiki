use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::model_artifact::{physical_result, solve_batch};
use super::orchestra_completion::http_json;
use super::write_ack_loss::count;
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessRunReport, ServiceHeadlessExecutor,
    execute_batch_with_executor, find_action_contract,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const ACTION: &str = "solve_advection_diffusion_bar_1d";

pub(super) fn batch(input: Value) -> HeadlessExecutionBatch {
    let mut batch = solve_batch(input);
    batch.steps[0].action = ACTION.into();
    batch.steps[0].risk = find_action_contract(ACTION).unwrap().risk;
    batch
}

fn single(left: f64, right: f64, diffusivity: f64, velocity: f64) -> Value {
    json!({
        "nodes":[{"x":0.0,"concentration":left,"fix_concentration":true},
                 {"x":1.0,"concentration":right,"fix_concentration":true}],
        "elements":[{"node_i":0,"node_j":1,"area":1.0,"diffusivity":diffusivity,"velocity":velocity}]
    })
}

fn chain(reversed: bool) -> Value {
    let mut input = json!({
        "nodes":[{"x":0.0,"concentration":2.0,"fix_concentration":true},
                 {"x":0.5,"source":4.0,"fix_concentration":false},
                 {"x":1.0,"concentration":4.0,"fix_concentration":true}],
        "elements":[{"node_i":0,"node_j":1,"area":1.0,"diffusivity":1.0,"velocity":0.5},
                    {"node_i":1,"node_j":2,"area":1.0,"diffusivity":1.0,"velocity":0.5}]
    });
    if reversed {
        for e in input["elements"].as_array_mut().unwrap() {
            let left = e["node_i"].clone();
            e["node_i"] = e["node_j"].clone();
            e["node_j"] = left;
        }
    }
    input
}

fn close(result: &Value, pointer: &str, expected: f64) {
    let actual = result
        .pointer(pointer)
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("missing finite field {pointer}: {result}"));
    assert!(actual.is_finite());
    assert!(
        (actual / expected - 1.0).abs() < 1e-12,
        "{pointer}: {actual} != {expected}"
    );
}

fn assert_chain(result: &Value) {
    // Independent Galerkin balance: 4*C_mid = 2.25*2 + 1.75*4 + 4.
    close(result, "/nodes/1/concentration", 3.875);
    close(result, "/elements/0/concentration_gradient", 3.75);
    close(result, "/elements/1/concentration_gradient", 0.25);
    close(result, "/elements/0/total_flux", -2.28125);
    close(result, "/elements/1/total_flux", 1.71875);
    close(result, "/max_peclet_number", 0.125);
    let balance = result["elements"][1]["total_flux"].as_f64().unwrap()
        - result["elements"][0]["total_flux"].as_f64().unwrap();
    assert_eq!(balance, 4.0);
}

pub(super) fn cli(
    batch: &HeadlessExecutionBatch,
    url: &str,
    success: bool,
) -> Result<HeadlessRunReport> {
    let source = write_temp_json("transport-finite-output", &serde_json::to_value(batch)?);
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

pub(super) fn guarded(input: Value) -> HeadlessExecutionBatch {
    let mut batch = batch(input);
    batch.steps.truncate(3);
    batch.steps[2].action = "project_create".into();
    batch.steps[2].risk = find_action_contract("project_create").unwrap().risk;
    batch.steps[2].payload = json!({"name":"must-not-follow-invalid-transport-output"});
    batch
}

fn assert_failed(report: &HeadlessRunReport, port: u16, field: &str) -> Result<()> {
    assert_eq!(report.status, "failed", "{report:?}");
    assert_eq!(report.executed_step_count, 1);
    assert_eq!(report.steps.len(), 2);
    let failure = report.execution_summary.failure.as_ref().unwrap();
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
    let id = report.steps[0].result_preview["job_id"].as_str().unwrap();
    let job = http_json(port, &format!("/api/v1/jobs/{id}"), None)?.1;
    assert_eq!(job["job"]["status"], "failed");
    assert_eq!(job["job"]["has_result"], false);
    let text = job.to_string();
    assert!(
        text.contains(field) && text.contains("representable"),
        "{job}"
    );
    Ok(())
}

#[test]
fn real_transport_output_failures_publish_no_result_and_same_agents_recover() -> Result<()> {
    let mut owner = LiveAgent::start()?;
    let mut peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    owner.restart_with_orchestrator(&url)?;
    peer.restart_with_orchestrator(&url)?;
    let mut executor = ServiceHeadlessExecutor::new(&url);
    for reversed in [false, true] {
        let report =
            execute_batch_with_executor(&batch(chain(reversed)), &mut executor, false, false);
        assert_eq!(report.status, "ok", "{report:?}\n{}", server.logs());
        assert_chain(&physical_result(&report));
    }
    let large = execute_batch_with_executor(
        &batch(single(1e308, 1e308, 1.0, 1e-308)),
        &mut executor,
        false,
        false,
    );
    assert_eq!(large.status, "ok", "{large:?}\n{}", server.logs());
    close(
        &physical_result(&large),
        "/elements/0/average_concentration",
        1e308,
    );
    close(&physical_result(&large), "/elements/0/total_flux", 1.0);

    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    let mut dense = single(0.0, 0.0, 1e308, 0.0);
    let duplicate = dense["elements"][0].clone();
    dense["elements"].as_array_mut().unwrap().push(duplicate);
    for (input, field) in [
        (single(1e308, 1e308, 1.0, 2.0), "advective flux"),
        (single(1.5e308, 0.5e308, 1.0, 1.0), "total flux"),
        (single(1.0, -1.0, 1e-308, 10.0), "Peclet number"),
        (dense, "assembled matrix"),
    ] {
        let report = execute_batch_with_executor(&guarded(input), &mut executor, false, false);
        assert_failed(&report, server.port, field)?;
    }
    let mut file = single(1e308, 1e308, 1.0, 2.0);
    file["transport_padding"] = json!("x".repeat(8_000_000));
    let report = cli(&guarded(file), &url, false)?;
    assert_eq!(
        report.steps[0].result_preview["model_artifact_upload"]["immutable"],
        true
    );
    assert_failed(&report, server.port, "advective flux")?;
    let recovered = cli(&batch(chain(true)), &url, true)?;
    assert_chain(&physical_result(&recovered));
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 9);
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        9
    );
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1,
        projects
    );
    Ok(())
}
