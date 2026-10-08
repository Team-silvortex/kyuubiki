use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::model_artifact::{physical_result, solve_batch};
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

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.graph-entity-input-normalization.json"
    ))
    .unwrap()
}

fn batch(action: &str, input: Value) -> HeadlessExecutionBatch {
    let mut batch = solve_batch(input);
    batch.steps[0].action = action.into();
    batch.steps[0].risk = find_action_contract(action).unwrap().risk;
    batch
}

fn padded(mut input: Value) -> Value {
    // Exercise actual file transport, not a stored large-mesh benchmark fixture.
    input["transport_padding"] = json!("x".repeat(8_000_000));
    input
}

fn guarded(input: Value) -> HeadlessExecutionBatch {
    let mut batch = batch("solve_heat_bar_1d", input);
    batch.steps.truncate(2);
    batch.steps.push(HeadlessExecutionBatchStep {
        action: "project_create".into(),
        index: 3,
        risk: find_action_contract("project_create").unwrap().risk,
        payload: json!({"name":"forbidden-after-invalid-graph"}),
    });
    batch
}

fn run_cli(batch: &HeadlessExecutionBatch, url: &str, success: bool) -> Result<HeadlessRunReport> {
    let source = write_temp_json("graph-entity-normalization", &serde_json::to_value(batch)?);
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

fn close(result: &Value, pointer: &str, expected: f64) {
    let actual = result
        .pointer(pointer)
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("missing {pointer}: {result}"));
    assert!(
        (actual - expected).abs() <= expected.abs().max(1e-10) * 1e-10,
        "{pointer}: {actual} != {expected}"
    );
}

fn assert_physics(action: &str, result: &Value) {
    match action {
        "solve_thermal_bar_1d" => {
            close(result, "/max_displacement", 0.00010005);
            close(result, "/max_stress", 10000.0);
            close(result, "/max_axial_force", 100.0);
            close(result, "/max_temperature_delta", 10.0);
        }
        "solve_heat_bar_1d" | "solve_transient_heat_bar_1d" => {
            close(result, "/nodes/1/temperature", 30.0);
            close(result, "/max_temperature", 40.0);
            close(result, "/max_heat_flux", 80.0);
            for index in 0..2 {
                close(
                    result,
                    &format!("/elements/{index}/temperature_gradient"),
                    20.0,
                );
                close(result, &format!("/elements/{index}/heat_flux"), -80.0);
            }
            if action == "solve_transient_heat_bar_1d" {
                close(result, "/final_time", 0.3);
                close(result, "/total_thermal_energy", 360.0);
                assert_eq!(result["history"].as_array().unwrap().len(), 4);
                for step in result["history"].as_array().unwrap() {
                    close(step, "/nodal_temperatures/1", 30.0);
                    close(step, "/total_thermal_energy", 360.0);
                }
            }
        }
        "solve_advection_diffusion_bar_1d" => {
            close(result, "/nodes/1/concentration", 2.0);
            close(result, "/max_total_flux", 1.0);
            close(result, "/max_peclet_number", 0.125);
            for index in 0..2 {
                close(result, &format!("/elements/{index}/total_flux"), 1.0);
                close(result, &format!("/elements/{index}/diffusive_flux"), 0.0);
            }
        }
        "solve_magnetostatic_bar_1d" => {
            close(result, "/nodes/1/magnetic_potential", 1.0);
            close(result, "/max_magnetic_field_strength", 2.0);
            close(result, "/max_flux_density", 8.0);
            close(result, "/total_stored_energy", 8.0);
            close(result, "/elements/0/magnetic_field_strength", -2.0);
            close(result, "/elements/0/magnetic_flux_density", -8.0);
        }
        "solve_electrostatic_bar_1d"
        | "solve_electrostatic_plane_triangle_2d"
        | "solve_electrostatic_plane_quad_2d" => {
            close(result, "/max_potential", 1.0);
            close(result, "/max_electric_field", 1.0);
            close(result, "/max_flux_density", 2.0);
            let energy = match action {
                "solve_electrostatic_bar_1d" => {
                    close(result, "/nodes/1/potential", 0.5);
                    close(result, "/elements/0/electric_field", -1.0);
                    close(result, "/elements/0/electric_flux_density", -2.0);
                    1.0
                }
                "solve_electrostatic_plane_triangle_2d" => 0.25,
                _ => 0.5,
            };
            close(result, "/total_stored_energy", energy);
        }
        "solve_heat_plane_triangle_2d" | "solve_heat_plane_quad_2d" => {
            close(result, "/max_temperature", 40.0);
            close(result, "/max_heat_flux", 80.0);
            close(result, "/elements/0/heat_flux_x", -80.0);
            close(result, "/elements/0/heat_flux_y", 0.0);
        }
        _ => panic!("uncovered physical model {action}"),
    }
    for collection in ["nodes", "elements"] {
        for (index, entity) in result[collection].as_array().unwrap().iter().enumerate() {
            assert_eq!(result["input"][collection][index]["id"], entity["id"]);
        }
    }
}

fn assert_failed(report: &HeadlessRunReport, artifact: bool) {
    assert_eq!(report.status, "failed", "{report:?}");
    assert_eq!(report.executed_step_count, usize::from(artifact));
    assert_eq!(report.steps.len(), if artifact { 2 } else { 1 });
    let failure = report.execution_summary.failure.as_ref().unwrap();
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
    if artifact {
        assert_eq!(
            failure.error_code,
            "kyuubiki.headless.invalid_solver_params"
        );
        assert_eq!(failure.stage, "agent_decode");
    }
}

#[test]
fn real_graph_entity_inline_and_artifact_results_agree_and_stop_ambiguous_id_chains() -> Result<()>
{
    let mut owner = LiveAgent::start()?;
    let mut peer = LiveAgent::start()?;
    let state = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&state.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    owner.restart_with_orchestrator(&url)?;
    peer.restart_with_orchestrator(&url)?;
    let mut executor = ServiceHeadlessExecutor::new(&url);
    let cases = fixture();
    let mut expected_admissions = 0;
    let mut covered = 0;
    for (action, input) in cases["models"].as_object().unwrap() {
        let inline =
            execute_batch_with_executor(&batch(action, input.clone()), &mut executor, false, false);
        assert_eq!(
            inline.status,
            "ok",
            "{action}: {inline:?}\n{}",
            server.logs()
        );
        assert!(inline.steps[0].result_preview["model_artifact_upload"].is_null());
        let large = padded(input.clone());
        let source_bytes = serde_json::to_vec(&large)?;
        let artifact =
            execute_batch_with_executor(&batch(action, large), &mut executor, false, false);
        assert_eq!(
            artifact.status,
            "ok",
            "{action}: {artifact:?}\n{}",
            server.logs()
        );
        let upload = &artifact.steps[0].result_preview["model_artifact_upload"];
        assert_eq!(
            upload["sha256"],
            format!("{:x}", Sha256::digest(&source_bytes))
        );
        assert_eq!(upload["size_bytes"], source_bytes.len());
        assert_eq!(upload["immutable"], true);
        let physical = physical_result(&artifact);
        assert_eq!(physical_result(&inline), physical, "{action}");
        assert_physics(action, &physical);
        for (collection, prefix) in [("nodes", "n"), ("elements", "e")] {
            for (index, entity) in physical[collection].as_array().unwrap().iter().enumerate() {
                assert_eq!(entity["id"], format!("{prefix}{index}"));
            }
        }
        let restored: HeadlessRunReport = serde_json::from_slice(&serde_json::to_vec(&artifact)?)?;
        assert_eq!(physical_result(&restored), physical);
        covered += 1;
        expected_admissions += 2;
    }
    assert_eq!(covered, 10);
    let mut literal_ids = cases["models"]["solve_heat_bar_1d"].clone();
    literal_ids["nodes"][0]["id"] = json!(" ");
    literal_ids["nodes"][1]["id"] = json!("");
    literal_ids["elements"][0]["id"] = json!("\t");
    let inline = execute_batch_with_executor(
        &batch("solve_heat_bar_1d", literal_ids.clone()),
        &mut executor,
        false,
        false,
    );
    let artifact = execute_batch_with_executor(
        &batch("solve_heat_bar_1d", padded(literal_ids)),
        &mut executor,
        false,
        false,
    );
    assert_eq!(inline.status, "ok", "{inline:?}");
    assert_eq!(artifact.status, "ok", "{artifact:?}");
    let physical = physical_result(&artifact);
    assert_eq!(physical_result(&inline), physical);
    assert_physics("solve_heat_bar_1d", &physical);
    assert_eq!(physical["nodes"][0]["id"], " ");
    assert_eq!(physical["nodes"][1]["id"], "n1");
    assert_eq!(physical["elements"][0]["id"], "\t");
    expected_admissions += 2;
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    for (collection, index, value, replacement) in [
        ("nodes", 1, json!("n0"), false),
        ("elements", 0, json!(7), false),
        ("nodes", 0, json!(["tuple", 0.0, true, 0.0, 0.0]), true),
        ("elements", 0, json!(["tuple", 0, 1, 2.0, 4.0]), true),
    ] {
        let mut input = cases["models"]["solve_heat_bar_1d"].clone();
        if replacement {
            input[collection][index] = value;
        } else {
            input[collection][index]["id"] = value;
        }
        let small = guarded(input.clone());
        assert_failed(
            &execute_batch_with_executor(&small, &mut executor, false, false),
            false,
        );
        assert_failed(&run_cli(&small, &url, false)?, false);
        assert_eq!(count(&owner)? + count(&peer)?, expected_admissions as u64);
        let large = guarded(padded(input));
        for report in [
            execute_batch_with_executor(&large, &mut executor, false, false),
            run_cli(&large, &url, false)?,
        ] {
            assert_failed(&report, true);
            let id = report.steps[0].result_preview["job_id"].as_str().unwrap();
            let job = http_json(server.port, &format!("/api/v1/jobs/{id}"), None)?.1;
            assert_eq!(job["job"]["status"], "failed");
            assert_eq!(job["job"]["has_result"], false);
        }
        expected_admissions += 2;
        assert_eq!(
            http_json(server.port, "/api/v1/projects", None)?.1,
            projects
        );
    }
    let healthy = run_cli(
        &batch(
            "solve_heat_bar_1d",
            padded(cases["models"]["solve_heat_bar_1d"].clone()),
        ),
        &url,
        true,
    )?;
    assert_physics("solve_heat_bar_1d", &physical_result(&healthy));
    expected_admissions += 1;
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(expected_admissions, 31);
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
