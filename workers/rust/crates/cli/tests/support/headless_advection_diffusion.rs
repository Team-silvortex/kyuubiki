use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::model_artifact::{physical_result, solve_batch};
use super::orchestra_completion::http_json;
use super::write_ack_loss::{count, document};
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessModelSourceKind,
    HeadlessResearchRoundSpec, HeadlessRunReport, ServiceHeadlessExecutor,
    build_headless_research_round_evidence, execute_batch_with_executor, find_action_contract,
    headless_saved_model_source,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const ACTION: &str = "solve_advection_diffusion_bar_1d";
const KIND: &str = "advection_diffusion_bar_1d";

fn input(velocity: f64, source: f64, right: f64) -> Value {
    json!({
        "nodes":[
            {"x":0.0,"fix_concentration":true,"concentration":2.0},
            {"x":0.5,"fix_concentration":false,"source":source},
            {"x":1.0,"fix_concentration":true,"concentration":right}
        ],
        "elements":[
            {"node_i":0,"node_j":1,"area":1.0,"diffusivity":1.0,"velocity":velocity},
            {"node_i":1,"node_j":2,"area":1.0,"diffusivity":1.0,"velocity":velocity}
        ]
    })
}

fn step(action: &str, index: usize, payload: Value) -> HeadlessExecutionBatchStep {
    HeadlessExecutionBatchStep {
        action: action.into(),
        index,
        payload,
        risk: find_action_contract(action).unwrap().risk,
    }
}

fn batch(input: Value) -> HeadlessExecutionBatch {
    let mut batch = solve_batch(input);
    batch.steps[0].action = ACTION.into();
    batch.steps[0].risk = find_action_contract(ACTION).unwrap().risk;
    batch
}

fn close(value: &Value, pointer: &str, expected: f64) {
    let actual = value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("missing {pointer}: {value}"));
    assert!(
        (actual - expected).abs() <= expected.abs().max(1.0) * 1e-10,
        "{pointer}: {actual} != {expected}"
    );
}

fn assert_physics(result: &Value, velocity: f64, source: f64, right: f64) {
    assert!(result.get("result_artifact_ref").is_none());
    // Independent two-element Galerkin equation: 4*C_mid =
    // (2+v/2)*C_left + (2-v/2)*C_right + source.
    let mid = ((2.0 + velocity / 2.0) * 2.0 + (2.0 - velocity / 2.0) * right + source) / 4.0;
    close(result, "/nodes/0/concentration", 2.0);
    close(result, "/nodes/1/concentration", mid);
    close(result, "/nodes/2/concentration", right);
    close(result, "/nodes/1/source", source);
    let mut fluxes = Vec::new();
    for (index, (left, right)) in [(2.0, mid), (mid, right)].into_iter().enumerate() {
        let gradient = (right - left) / 0.5;
        let diffusive = -gradient;
        let advective = velocity * (left + right) / 2.0;
        let total = diffusive + advective;
        fluxes.push(total);
        for (field, expected) in [
            ("concentration_gradient", gradient),
            ("diffusive_flux", diffusive),
            ("advective_flux", advective),
            ("total_flux", total),
            ("peclet_number", velocity.abs() * 0.25),
        ] {
            close(result, &format!("/elements/{index}/{field}"), expected);
        }
    }
    assert!((fluxes[1] - fluxes[0] - source).abs() < 1e-10);
    close(
        result,
        "/max_total_flux",
        fluxes[0].abs().max(fluxes[1].abs()),
    );
    close(result, "/max_peclet_number", velocity.abs() * 0.25);
}

fn cli(batch: &HeadlessExecutionBatch, url: &str, success: bool) -> Result<HeadlessRunReport> {
    let source = write_temp_json("advection-diffusion-chain", &serde_json::to_value(batch)?);
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

pub(super) fn graph() -> Value {
    json!({
        "schema_version":"kyuubiki.workflow-graph/v1",
        "id":"workflow.live.advection-diffusion", "name":"Transport chain", "version":"1.0.0",
        "entry_nodes":["input"], "output_nodes":["output"],
        "nodes":[
            {"id":"input","kind":"input","outputs":[{"id":"model","artifact_type":"model/advection_diffusion_bar_1d"}]},
            {"id":"solve","kind":"solve","operator_id":"solve.advection_diffusion_bar_1d",
             "inputs":[{"id":"model","artifact_type":"model/advection_diffusion_bar_1d"}],
             "outputs":[{"id":"result","artifact_type":"result/advection_diffusion_bar_1d"}]},
            {"id":"output","kind":"output","inputs":[{"id":"result","artifact_type":"result/advection_diffusion_bar_1d"}],"outputs":[]}
        ],
        "edges":[
            {"id":"in-solve","from":{"node":"input","port":"model"},"to":{"node":"solve","port":"model"},"artifact_type":"model/advection_diffusion_bar_1d"},
            {"id":"solve-out","from":{"node":"solve","port":"result"},"to":{"node":"output","port":"result"},"artifact_type":"result/advection_diffusion_bar_1d"}
        ]
    })
}

#[test]
fn real_transport_service_chain_preserves_physics_sources_and_failure_recovery() -> Result<()> {
    let mut owner = LiveAgent::start()?;
    let mut peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    owner.restart_with_orchestrator(&url)?;
    peer.restart_with_orchestrator(&url)?;
    let mut executor = ServiceHeadlessExecutor::new(&url);
    let operators = http_json(server.port, "/api/v1/operators", None)?.1;
    assert!(
        operators["operators"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["id"] == "solve.advection_diffusion_bar_1d")
    );

    for (velocity, source, right) in [(0.0, 0.0, 4.0), (-0.5, 0.0, 2.0), (0.5, 4.0, 2.0)] {
        let report = execute_batch_with_executor(
            &batch(input(velocity, source, right)),
            &mut executor,
            false,
            false,
        );
        assert_eq!(report.status, "ok", "{report:?}\n{}", server.logs());
        assert_physics(&physical_result(&report), velocity, source, right);
    }
    let mut large = input(0.5, 4.0, 2.0);
    // Force file transport without retaining a large numerical fixture.
    large["transport_padding"] = json!("x".repeat(8_000_000));
    let report = cli(&batch(large), &url, true)?;
    assert_eq!(
        report.steps[0].result_preview["model_artifact_upload"]["immutable"],
        true
    );
    assert_physics(&physical_result(&report), 0.5, 4.0, 2.0);

    let project = executor
        .execute_step("project_create", 1, &json!({"name":"transport-research"}))
        .map_err(|e| e.message)?
        .result;
    let created = executor
        .execute_step(
            "model_create",
            1,
            &json!({
                "project_id":project["project_id"], "kind":KIND,"name":"transport-source",
                "payload":input(0.5,4.0,2.0)
            }),
        )
        .map_err(|e| e.message)?
        .result;
    let version_id = created["latest_version_id"].as_str().unwrap();
    let version = http_json(
        server.port,
        &format!("/api/v1/model-versions/{version_id}"),
        None,
    )?
    .1;
    let source =
        headless_saved_model_source(&version["version"], HeadlessModelSourceKind::ModelVersion)?;
    let mut saved = document(
        "solve_and_wait_from_model_version",
        json!({
            "model_version_id":version_id, "expected_model_source":source,
            "timeout_ms":5000,"interval_ms":10
        }),
    );
    saved.steps.truncate(1);
    saved.steps[0].risk = find_action_contract(&saved.steps[0].action).unwrap().risk;
    let saved_report = cli(&saved, &url, true)?;
    assert_eq!(
        saved_report.steps[0].result_preview["model_source"],
        serde_json::to_value(&source)?
    );
    assert_physics(
        &saved_report.steps[0].result_preview["result"]["result"],
        0.5,
        4.0,
        2.0,
    );
    let spec: HeadlessResearchRoundSpec = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-research-round-spec/v1",
        "round_id":"transport-source-bound", "workflow_id":saved.workflow_id,"iteration":1,
        "primary_metric_ids":["mid_concentration"],
        "metrics":[{"metric_id":"mid_concentration","pointer":"/steps/0/result_preview/result/result/nodes/1/concentration",
                    "unit":"1","objective":"observe"}]
    }))?;
    let expected =
        build_headless_research_round_evidence(&saved, &saved_report, &spec, None, None)?;
    let retained = data.0.join("retained.json");
    fs::write(
        &retained,
        serde_json::to_vec(&json!({"batch":saved,"report":saved_report}))?,
    )?;
    let retained: Value = serde_json::from_slice(&fs::read(retained)?)?;
    let retained_batch = serde_json::from_value(retained["batch"].clone())?;
    let retained_report = serde_json::from_value(retained["report"].clone())?;
    assert_eq!(
        build_headless_research_round_evidence(
            &retained_batch,
            &retained_report,
            &spec,
            None,
            None
        )?,
        expected
    );

    let mut workflow = batch(Value::Null);
    workflow.steps.truncate(3);
    workflow.steps[0] = step(
        "workflow_submit_graph",
        1,
        json!({"graph":graph(),"input_artifacts":{"input":input(-0.5,0.0,2.0)}}),
    );
    let workflow_report = execute_batch_with_executor(&workflow, &mut executor, true, false);
    assert_eq!(
        workflow_report.status,
        "ok",
        "{workflow_report:?}\n{}",
        server.logs()
    );
    let result = physical_result(&workflow_report);
    assert_eq!(result["failed_nodes"], json!([]));
    assert_eq!(result["completed_nodes"].as_array().unwrap().len(), 3);
    assert_physics(&result["artifacts"]["solve.result"], -0.5, 0.0, 2.0);

    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    let mut bad = input(0.5, 4.0, 2.0);
    bad["elements"][0]["diffusivity"] = json!(0.0);
    let mut guarded = batch(bad);
    guarded.steps.truncate(2);
    guarded.steps.push(step(
        "project_create",
        3,
        json!({"name":"must-not-follow-invalid-diffusivity"}),
    ));
    for report in [
        execute_batch_with_executor(&guarded, &mut executor, false, false),
        cli(&guarded, &url, false)?,
    ] {
        assert_eq!(report.status, "failed");
        assert_eq!(report.executed_step_count, 1);
        assert_eq!(report.steps.len(), 2);
        let failure = report.execution_summary.failure.as_ref().unwrap();
        assert!(!failure.retryable);
        assert_eq!(failure.retry_strategy, "none");
        let id = report.steps[0].result_preview["job_id"].as_str().unwrap();
        let job = http_json(server.port, &format!("/api/v1/jobs/{id}"), None)?.1;
        assert_eq!(job["job"]["status"], "failed");
        assert_eq!(job["job"]["has_result"], false);
    }
    let healthy = cli(&batch(input(0.0, 0.0, 4.0)), &url, true)?;
    assert_physics(&physical_result(&healthy), 0.0, 0.0, 4.0);
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
