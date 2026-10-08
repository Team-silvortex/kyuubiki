#[path = "../../../engine/tests/support/diagnostic_chain.rs"]
pub(super) mod diagnostic_chain;

use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::model_artifact::physical_result;
use super::orchestra_completion::http_json;
use super::transport_output::{batch, cli_with_policy};
use super::write_ack_loss::count;
use super::{Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessResearchRoundSpec, build_headless_research_round_evidence,
    find_action_contract,
};
use kyuubiki_protocol::{SolveAdvectionDiffusionBar1dRequest, WorkflowGraphRunRequest};
use serde_json::{Value, json};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn model(n: usize, velocity: f64) -> Value {
    json!({"scheme":"upwind",
        "nodes":(0..=n).map(|i| json!({"x":i as f64/n as f64,"concentration":if i==n {1.0} else {0.0},"fix_concentration":i==0 || i==n})).collect::<Vec<_>>(),
        "elements":(0..n).map(|i| json!({"node_i":i+1,"node_j":i,"area":1.0,"diffusivity":1.0,"velocity":velocity})).collect::<Vec<_>>()})
}

fn request(input: Value, solve: bool, recover: bool) -> WorkflowGraphRunRequest {
    let request = diagnostic_chain::request(
        "transport",
        "transform.score_transport_quality",
        json!({"enabled_terms":["transport_artificial_diffusivity_peak"],
            "targets":{"transport_artificial_diffusivity_peak":1.0},
            "weights":{"transport_artificial_diffusivity_peak":1.0},"max_ready_score":2.0}),
        input,
        recover,
    );
    if solve {
        diagnostic_chain::with_solver(request, "solve.advection_diffusion_bar_1d")
    } else {
        request
    }
}

pub(super) fn workflow(request: WorkflowGraphRunRequest) -> HeadlessExecutionBatch {
    let mut batch = batch(Value::Null);
    batch.steps.truncate(3);
    batch.steps[0].action = "workflow_submit_graph".into();
    batch.steps[0].risk = find_action_contract("workflow_submit_graph").unwrap().risk;
    batch.steps[0].payload =
        json!({"graph":request.graph,"input_artifacts":request.input_artifacts});
    batch
}

fn close(actual: &Value, expected: f64) {
    let actual = actual.as_f64().unwrap();
    assert!(
        (actual - expected).abs() <= 1e-10 * expected.abs().max(1.0),
        "{actual} != {expected}"
    );
}

pub(super) fn assert_retained(actual: &Value, expected: &Value, path: &str) {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => {
            assert_eq!(actual.len(), expected.len(), "{path}: field count changed");
            for (key, value) in expected {
                assert_retained(actual.get(key).unwrap(), value, &format!("{path}.{key}"));
            }
        }
        (Value::Array(actual), Value::Array(expected)) => {
            assert_eq!(actual.len(), expected.len(), "{path}: record count changed");
            for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                assert_retained(actual, expected, &format!("{path}[{index}]"));
            }
        }
        (Value::Number(actual), Value::Number(expected)) if actual != expected => {
            // Only JSON integer/float spelling may differ, never a floating-point bit.
            assert_ne!(actual.is_f64(), expected.is_f64(), "{path}: number changed");
            let integer = if actual.is_f64() { expected } else { actual };
            let magnitude = integer
                .as_i64()
                .map(i64::unsigned_abs)
                .or_else(|| integer.as_u64())
                .unwrap();
            assert!(
                magnitude <= 1_u64 << 53,
                "{path}: integer rewrite is not exact"
            );
            assert_eq!(
                actual.as_f64().unwrap().to_bits(),
                expected.as_f64().unwrap().to_bits(),
                "{path}: retained number changed"
            );
        }
        _ => assert_eq!(actual, expected, "{path}: retained value changed"),
    }
}

fn assert_decision(result: &Value, n: usize, velocity: f64) {
    assert_eq!(result["failed_nodes"], json!([]));
    assert_eq!(result["completed_nodes"].as_array().unwrap().len(), 9);
    let artifacts = &result["artifacts"];
    let artificial = velocity.abs() / (2.0 * n as f64);
    let positive_flux = -velocity.abs() / ((1.0 + velocity.abs() / n as f64).powi(n as i32) - 1.0);
    let flux = if velocity > 0.0 {
        positive_flux
    } else {
        -velocity.abs() + positive_flux
    };
    assert_eq!(artifacts["raw.payload"]["input"]["scheme"], "upwind");
    assert_eq!(artifacts["diagnose.summary"]["diagnostic_scheme"], "upwind");
    close(
        &artifacts["diagnose.summary"]["transport_artificial_diffusivity_peak"],
        artificial,
    );
    close(
        &artifacts["diagnose.summary"]["transport_numerical_flux_peak"],
        flux,
    );
    close(
        &artifacts["quality.summary"]["transport_quality_score"],
        artificial,
    );
    assert_eq!(
        artifacts["decision.summary"]["composite_quality_ready"],
        n == 8
    );
    assert_eq!(artifacts["independent_output.value"], json!({"value":7}));
}

#[test]
fn real_upwind_research_decision_chain_retains_metrics_blocks_corruption_and_recovers() -> Result<()>
{
    let mut owner = LiveAgent::start()?;
    let mut peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    owner.restart_with_orchestrator(&url)?;
    peer.restart_with_orchestrator(&url)?;
    let coarse = workflow(request(model(2, 20.0), true, false));
    let report = cli_with_policy(&coarse, &url, true, true)?;
    assert_decision(&physical_result(&report), 2, 20.0);
    let refined = workflow(request(model(8, -20.0), true, false));
    let report = cli_with_policy(&refined, &url, true, true)?;
    let result = physical_result(&report);
    assert_decision(&result, 8, -20.0);
    let spec: HeadlessResearchRoundSpec = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-research-round-spec/v1",
        "round_id":"transport-stabilization-objective", "workflow_id":refined.workflow_id,"iteration":1,
        "primary_metric_ids":["artificial_diffusivity"],
        "metrics":[{"metric_id":"artificial_diffusivity", "pointer":"/steps/2/result_preview/result/artifacts/diagnose.summary/transport_artificial_diffusivity_peak",
                    "unit":"1","objective":"minimize"}]
    }))?;
    let evidence = build_headless_research_round_evidence(&refined, &report, &spec, None, None)?;
    assert!(build_headless_research_round_evidence(&coarse, &report, &spec, None, None).is_err());
    let path = data.0.join("retained-upwind-research.json");
    fs::write(
        &path,
        serde_json::to_vec(&json!({"batch":refined,"report":report}))?,
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
        evidence
    );

    let mut corrupted = result["artifacts"]["raw.payload"].clone();
    corrupted["elements"][7]["stabilization"]["numerical_flux"] = Value::Null;
    let report = cli_with_policy(
        &workflow(request(corrupted.clone(), false, true)),
        &url,
        true,
        true,
    )?;
    let partial = physical_result(&report);
    assert_eq!(partial["failed_nodes"], json!(["diagnose"]));
    assert_eq!(partial["skipped_nodes"].as_array().unwrap().len(), 3);
    for node in ["diagnose", "quality", "objective", "decision"] {
        assert!(
            partial["artifacts"]
                .get(format!("{node}.summary"))
                .is_none()
        );
    }
    let mut preserved = partial["artifacts"]["raw.payload"].clone();
    assert_eq!(
        serde_json::from_value::<SolveAdvectionDiffusionBar1dRequest>(preserved["input"].clone())?,
        serde_json::from_value::<SolveAdvectionDiffusionBar1dRequest>(corrupted["input"].clone())?
    );
    // Batch normalization can spell integral model parameters as integers.
    // The typed input is identical; every raw output number must retain its bits.
    preserved["input"] = corrupted["input"].clone();
    assert_retained(&preserved, &corrupted, "raw.payload");
    assert_eq!(
        partial["artifacts"]["independent_output.value"],
        json!({"value":7})
    );
    let trace = partial["node_runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["node_id"] == "diagnose")
        .unwrap();
    assert!(
        trace["error_message"]
            .as_str()
            .unwrap()
            .contains("payload.elements[7].stabilization.numerical_flux")
    );
    let recovered = cli_with_policy(
        &workflow(request(model(8, 20.0), true, false)),
        &url,
        true,
        true,
    )?;
    assert_decision(&physical_result(&recovered), 8, 20.0);
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 10);
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    Ok(())
}
