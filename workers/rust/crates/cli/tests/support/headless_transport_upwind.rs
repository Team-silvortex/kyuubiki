use super::advection_diffusion::graph;
use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::model_artifact::physical_result;
use super::orchestra_completion::http_json;
use super::transport_output::{batch, cli, guarded};
use super::write_ack_loss::count;
use super::{Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::{
    HeadlessResearchRoundSpec, ServiceHeadlessExecutor, build_headless_research_round_evidence,
    execute_batch_with_executor, find_action_contract,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn model(velocity: f64, source: f64, reversed: bool) -> Value {
    let mut input = json!({
        "scheme":"upwind",
        "nodes":[{"x":0.0,"concentration":0.0,"fix_concentration":true},
                 {"x":0.5,"source":source,"fix_concentration":false},
                 {"x":1.0,"concentration":1.0,"fix_concentration":true}],
        "elements":[{"node_i":0,"node_j":1,"area":1.0,"diffusivity":1.0,"velocity":velocity},
                    {"node_i":1,"node_j":2,"area":1.0,"diffusivity":1.0,"velocity":velocity}]
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
    let actual = result.pointer(pointer).and_then(Value::as_f64).unwrap();
    assert!(actual.is_finite());
    assert!(
        (actual - expected).abs() <= 1e-10 * expected.abs().max(1.0),
        "{pointer}: {actual} != {expected}"
    );
}

fn assert_physics(result: &Value, velocity: f64, source: f64) {
    // Independent conservative balance for h=1/2, D=A=1, boundary [0,1].
    let mid = (2.0 + (-velocity).max(0.0) + source) / (4.0 + velocity.abs());
    close(result, "/nodes/1/concentration", mid);
    assert_eq!(result["input"]["scheme"], "upwind");
    let mut fluxes = Vec::new();
    for (i, (left, right)) in [(0.0, mid), (mid, 1.0)].into_iter().enumerate() {
        let gradient = (right - left) * 2.0;
        let physical = -gradient + velocity * (left + right) * 0.5;
        let artificial = velocity.abs() * 0.25;
        let numerical = -gradient + velocity * if velocity >= 0.0 { left } else { right };
        close(result, &format!("/elements/{i}/total_flux"), physical);
        close(
            result,
            &format!("/elements/{i}/stabilization/artificial_diffusivity"),
            artificial,
        );
        close(
            result,
            &format!("/elements/{i}/stabilization/stabilization_flux"),
            -artificial * gradient,
        );
        close(
            result,
            &format!("/elements/{i}/stabilization/numerical_flux"),
            numerical,
        );
        fluxes.push(numerical);
    }
    assert!((fluxes[1] - fluxes[0] - source).abs() < 1e-10);
    close(
        result,
        "/max_numerical_flux",
        fluxes[0].abs().max(fluxes[1].abs()),
    );
}

#[test]
fn real_upwind_service_chain_preserves_scheme_flux_metrics_and_failure_recovery() -> Result<()> {
    let mut owner = LiveAgent::start()?;
    let mut peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    owner.restart_with_orchestrator(&url)?;
    peer.restart_with_orchestrator(&url)?;
    let mut executor = ServiceHeadlessExecutor::new(&url);
    for (velocity, source, reversed) in [(20.0, 0.0, false), (-20.0, 0.0, false), (20.0, 4.0, true)]
    {
        let report = execute_batch_with_executor(
            &batch(model(velocity, source, reversed)),
            &mut executor,
            false,
            false,
        );
        assert_eq!(report.status, "ok", "{report:?}\n{}", server.logs());
        assert_physics(&physical_result(&report), velocity, source);
    }

    let mut file = model(20.0, 4.0, true);
    file["transport_padding"] = json!("x".repeat(8_000_000));
    let retained_batch = batch(file);
    let file_report = cli(&retained_batch, &url, true)?;
    assert_eq!(
        file_report.steps[0].result_preview["model_artifact_upload"]["immutable"],
        true
    );
    assert_physics(&physical_result(&file_report), 20.0, 4.0);
    let spec: HeadlessResearchRoundSpec = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-research-round-spec/v1",
        "round_id":"transport-upwind-conservation", "workflow_id":retained_batch.workflow_id,"iteration":1,
        "primary_metric_ids":["numerical_flux"],
        "metrics":[{"metric_id":"numerical_flux","pointer":"/steps/2/result_preview/result/elements/0/stabilization/numerical_flux",
                    "unit":"1","objective":"observe"}]
    }))?;
    let expected =
        build_headless_research_round_evidence(&retained_batch, &file_report, &spec, None, None)?;
    let path = data.0.join("retained-upwind.json");
    fs::write(
        &path,
        serde_json::to_vec(&json!({"batch":retained_batch,"report":file_report}))?,
    )?;
    let retained: Value = serde_json::from_slice(&fs::read(path)?)?;
    assert_eq!(
        build_headless_research_round_evidence(
            &serde_json::from_value(retained["batch"].clone())?,
            &serde_json::from_value(retained["report"].clone())?,
            &spec,
            None,
            None
        )?,
        expected
    );

    let mut workflow = batch(Value::Null);
    workflow.steps.truncate(3);
    workflow.steps[0].action = "workflow_submit_graph".into();
    workflow.steps[0].risk = find_action_contract("workflow_submit_graph").unwrap().risk;
    workflow.steps[0].payload =
        json!({"graph":graph(),"input_artifacts":{"input":model(-20.0,0.0,true)}});
    let report = execute_batch_with_executor(&workflow, &mut executor, true, false);
    assert_eq!(report.status, "ok", "{report:?}\n{}", server.logs());
    let result = physical_result(&report);
    assert_eq!(result["failed_nodes"], json!([]));
    assert_eq!(result["completed_nodes"].as_array().unwrap().len(), 3);
    assert_physics(&result["artifacts"]["solve.result"], -20.0, 0.0);

    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    let mut invalid = model(20.0, 0.0, false);
    invalid["scheme"] = json!("auto");
    let rejected =
        execute_batch_with_executor(&guarded(invalid.clone()), &mut executor, false, false);
    assert_eq!(rejected.status, "failed");
    assert_eq!(rejected.executed_step_count, 0);
    assert_eq!(rejected.steps.len(), 1);
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    assert_eq!(count(&owner)? + count(&peer)?, 5);
    invalid["transport_padding"] = json!("x".repeat(8_000_000));
    let file_failure = cli(&guarded(invalid), &url, false)?;
    let mut overflow = model(2.0, 0.0, false);
    for node in overflow["nodes"].as_array_mut().unwrap() {
        node["concentration"] = json!(1e308);
        node["fix_concentration"] = json!(true);
    }
    let overflow_failure =
        execute_batch_with_executor(&guarded(overflow), &mut executor, false, false);
    for (report, reason) in [
        (file_failure, "unknown variant"),
        (overflow_failure, "representable"),
    ] {
        assert_eq!(report.status, "failed", "{report:?}");
        assert_eq!(report.executed_step_count, 1);
        assert_eq!(report.steps.len(), 2);
        assert!(!report.execution_summary.failure.as_ref().unwrap().retryable);
        let id = report.steps[0].result_preview["job_id"].as_str().unwrap();
        let job = http_json(server.port, &format!("/api/v1/jobs/{id}"), None)?.1;
        assert_eq!(job["job"]["status"], "failed");
        assert_eq!(job["job"]["has_result"], false);
        assert!(job.to_string().contains(reason), "{job}");
    }
    let recovered = cli(&batch(model(20.0, 4.0, false)), &url, true)?;
    assert_physics(&physical_result(&recovered), 20.0, 4.0);
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 8);
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1,
        projects
    );
    Ok(())
}
