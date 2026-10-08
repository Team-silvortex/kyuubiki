#[path = "support/diagnostic_chain.rs"]
mod diagnostic_chain;

use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::WorkflowGraphRunRequest;
use serde_json::{Value, json};

fn request(input: Value, recover: bool) -> WorkflowGraphRunRequest {
    diagnostic_chain::request(
        "transport",
        "transform.score_transport_quality",
        json!({"enabled_terms":["transport_artificial_diffusivity_peak"],
               "targets":{"transport_artificial_diffusivity_peak":1.0},
               "weights":{"transport_artificial_diffusivity_peak":1.0},"max_ready_score":2.0}),
        input,
        recover,
    )
}

fn model(n: usize, velocity: f64, reversed: bool) -> Value {
    json!({"scheme":"upwind",
        "nodes":(0..=n).map(|i| json!({"x":i as f64/n as f64,"concentration":if i==n {1.0} else {0.0},"fix_concentration":i==0 || i==n})).collect::<Vec<_>>(),
        "elements":(0..n).map(|i| json!({"node_i":if reversed {i+1} else {i},"node_j":if reversed {i} else {i+1},
            "area":1.0,"diffusivity":1.0,"velocity":velocity})).collect::<Vec<_>>()})
}

fn close(actual: &Value, expected: f64) {
    let actual = actual.as_f64().unwrap();
    assert!(
        (actual - expected).abs() < 1e-11 * expected.abs().max(1.0),
        "{actual} != {expected}"
    );
}

#[test]
fn real_upwind_solve_diagnose_score_decision_chain_tracks_mesh_diffusion_not_physical_flux() {
    for velocity in [20.0, -20.0] {
        for n in [2, 8] {
            for reversed in [false, true] {
                let run = run_workflow_graph(diagnostic_chain::with_solver(
                    request(model(n, velocity, reversed), false),
                    "solve.advection_diffusion_bar_1d",
                ))
                .unwrap();
                assert!(run.failed_nodes.is_empty());
                assert!(run.skipped_nodes.is_empty());
                let raw = &run.artifacts["raw.payload"];
                let diagnostic = &run.artifacts["diagnose.summary"];
                assert_eq!(raw["input"]["scheme"], "upwind");
                assert_eq!(diagnostic["diagnostic_scheme"], "upwind");
                let artificial = velocity.abs() / (2.0 * n as f64);
                close(
                    &diagnostic["transport_artificial_diffusivity_peak"],
                    artificial,
                );
                // Independent constant conservative flux from the discrete recurrence.
                let positive_flux =
                    -velocity.abs() / ((1.0 + velocity.abs() / n as f64).powi(n as i32) - 1.0);
                let flux = if velocity > 0.0 {
                    positive_flux
                } else {
                    -velocity.abs() + positive_flux
                };
                close(&diagnostic["transport_numerical_flux_peak"], flux);
                close(
                    &run.artifacts["quality.summary"]["transport_quality_score"],
                    artificial,
                );
                assert_eq!(
                    run.artifacts["decision.summary"]["composite_quality_ready"],
                    n == 8
                );
                assert_eq!(
                    run.artifacts["decision.summary"]["composite_quality_missing_metric_count"],
                    0
                );
                let physical_peak = raw["elements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| e["total_flux"].as_f64().unwrap().abs())
                    .fold(0.0_f64, f64::max);
                close(
                    &diagnostic["transport_total_flux_peak_magnitude"],
                    physical_peak,
                );
                assert_eq!(
                    run.artifacts["independent_output.value"],
                    json!({"value":7})
                );
            }
        }
    }
}

#[test]
fn corrupt_upwind_diagnostics_block_dependent_decisions_and_healthy_evidence_recovers() {
    let healthy = run_workflow_graph(diagnostic_chain::with_solver(
        request(model(8, -20.0, true), false),
        "solve.advection_diffusion_bar_1d",
    ))
    .unwrap()
    .artifacts["raw.payload"]
        .clone();
    for field in [
        "artificial_diffusivity",
        "stabilization_flux",
        "numerical_flux",
    ] {
        let mut corrupt = healthy.clone();
        corrupt["elements"][7]["stabilization"][field] = Value::Null;
        let path = format!("payload.elements[7].stabilization.{field}");
        let error = run_workflow_graph(request(corrupt.clone(), false)).unwrap_err();
        assert!(
            error.contains("workflow node diagnose failed") && error.contains(&path),
            "{error}"
        );
        let run = run_workflow_graph(request(corrupt.clone(), true)).unwrap();
        assert_eq!(run.failed_nodes, ["diagnose"]);
        assert_eq!(run.skipped_nodes.len(), 3);
        for node in ["diagnose", "quality", "objective", "decision"] {
            assert!(!run.artifacts.contains_key(&format!("{node}.summary")));
        }
        assert_eq!(run.artifacts["raw.payload"], corrupt);
        assert_eq!(
            run.artifacts["independent_output.value"],
            json!({"value":7})
        );
        let recovered = run_workflow_graph(request(healthy.clone(), false)).unwrap();
        assert!(recovered.failed_nodes.is_empty());
        assert_eq!(
            recovered.artifacts["decision.summary"]["composite_quality_ready"],
            true
        );
    }
}
