#[path = "support/diagnostic_chain.rs"]
mod diagnostic_chain;

use kyuubiki_engine::{run_extract_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowNodeRunStatus;
use kyuubiki_solver::solver_control::{
    SolverControl, SolverStage, with_solver_control, with_solver_observer,
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn extract(domain: &str, input: Value) -> Result<Value, String> {
    run_extract_operator(
        &format!("extract.{domain}_result_diagnostics"),
        input,
        json!({}),
    )
}

fn cfd(nodes: usize, elements: usize) -> Value {
    json!({
        "nodes": (0..nodes).map(|i| json!({"id":format!("n{i}"),
            "velocity_magnitude":2.0,"pressure":-3.0})).collect::<Vec<_>>(),
        "elements": (0..elements).map(|i| json!({"id":format!("e{i}"),
            "divergence_error":0.01,"reynolds_number":0.5,
            "viscous_dissipation":0.25})).collect::<Vec<_>>()
    })
}

fn cancel_on_scan(
    control: &SolverControl,
    scan: usize,
    completed: u64,
) -> impl Fn(kyuubiki_solver::solver_control::SolverCheckpoint) + use<> {
    let cancel = control.clone();
    let current_scan = Cell::new(0);
    move |point| {
        if point.stage != SolverStage::ResultDiagnostics {
            return;
        }
        if point.completed_steps == 0 {
            current_scan.set(current_scan.get() + 1);
        }
        if current_scan.get() == scan && point.completed_steps == completed {
            cancel.request_cancel();
        }
    }
}

fn assert_cancelled(result: Result<Value, String>, control: &SolverControl, completed: u64) {
    let error = result.unwrap_err();
    assert!(
        error.contains(&format!(
            "cancelled at result_diagnostics after {completed} steps"
        )),
        "{error}"
    );
    assert!(control.was_interrupted());
    assert_eq!(
        control.last_checkpoint().unwrap().stage,
        SolverStage::ResultDiagnostics
    );
}

#[test]
fn cfd_node_scan_cancellation_precedes_late_corruption_and_fresh_call_recovers() {
    let healthy = cfd(129, 129);
    let mut bad = healthy.clone();
    bad["nodes"][100]["pressure"] = Value::Null;
    let control = SolverControl::default();
    let result = with_solver_observer(&control, cancel_on_scan(&control, 1, 64), || {
        extract("stokes_flow", bad)
    });
    assert_cancelled(result, &control, 64);
    let fresh = SolverControl::default();
    let summary = with_solver_control(&fresh, || extract("stokes_flow", healthy)).unwrap();
    assert!((summary["cfd_pressure_mean"].as_f64().unwrap() + 3.0).abs() < 1e-12);
    assert_eq!(summary["cfd_velocity_max"], 2.0);
    assert_eq!(summary["cfd_viscous_dissipation_total"], 32.25);
    assert!(!fresh.was_interrupted());
}

#[test]
fn cfd_element_scan_cancellation_discards_completed_node_statistics() {
    let mut bad = cfd(129, 129);
    bad["elements"][100]["viscous_dissipation"] = Value::Null;
    let control = SolverControl::default();
    let result = with_solver_observer(&control, cancel_on_scan(&control, 2, 64), || {
        extract("stokes_flow", bad)
    });
    assert_cancelled(result, &control, 64);
    let fresh = extract("stokes_flow", cfd(129, 129)).unwrap();
    assert_eq!(fresh["cfd_divergence_error_peak_element_id"], "e128");
    assert_eq!(fresh["diagnostic_node_count"], 129);
}

#[test]
fn cfd_safe_point_cadence_covers_both_scans_and_short_final_records() {
    for (nodes, elements) in [(1, 1), (63, 65), (64, 64), (65, 129), (129, 63)] {
        let control = SolverControl::default();
        let points = Rc::new(RefCell::new(Vec::new()));
        let seen = points.clone();
        with_solver_observer(
            &control,
            move |point| {
                assert_eq!(point.stage, SolverStage::ResultDiagnostics);
                seen.borrow_mut().push(point.completed_steps);
            },
            || extract("stokes_flow", cfd(nodes, elements)),
        )
        .unwrap();
        let expected = [nodes, elements]
            .into_iter()
            .flat_map(|len| (0..=len).filter(move |i| i % 64 == 0 || *i == len))
            .map(|i| i as u64)
            .collect::<Vec<_>>();
        assert_eq!(
            *points.borrow(),
            expected,
            "nodes={nodes}, elements={elements}"
        );
        let control = SolverControl::default();
        let result = with_solver_observer(
            &control,
            cancel_on_scan(&control, 2, elements as u64),
            || extract("stokes_flow", cfd(nodes, elements)),
        );
        assert_cancelled(result, &control, elements as u64);
    }
}

#[test]
fn cfd_pending_cancellation_is_observed_before_reading_first_record() {
    let mut bad = cfd(1, 1);
    bad["nodes"][0] = Value::Null;
    let control = SolverControl::default();
    control.request_cancel();
    let error = with_solver_control(&control, || extract("stokes_flow", bad.clone())).unwrap_err();
    assert!(
        error.contains("cancelled before numerical execution"),
        "{error}"
    );
    assert!(control.was_interrupted());
    assert!(control.last_checkpoint().is_none());
    let fresh = SolverControl::default();
    let error = with_solver_control(&fresh, || extract("stokes_flow", bad)).unwrap_err();
    assert!(
        error.contains("payload.nodes[0] must be an object"),
        "{error}"
    );
    assert!(!fresh.was_interrupted());
    assert!(extract("stokes_flow", cfd(1, 1)).is_ok());
}

#[derive(Clone, Copy)]
struct Domain {
    name: &'static str,
    scalar: &'static str,
    vector_x: &'static str,
    vector_y: &'static str,
    vector_source: &'static str,
    vector_scan: usize,
    peak: &'static str,
    term: &'static str,
}

const DOMAINS: [Domain; 4] = [
    Domain {
        name: "thermal",
        scalar: "temperature",
        vector_x: "heat_flux_x",
        vector_y: "heat_flux_y",
        vector_source: "elements",
        vector_scan: 4,
        peak: "thermal_flux_peak_magnitude",
        term: "thermal_temperature_max",
    },
    Domain {
        name: "electrostatic",
        scalar: "potential",
        vector_x: "electric_field_x",
        vector_y: "electric_field_y",
        vector_source: "elements",
        vector_scan: 4,
        peak: "electrostatic_field_peak_magnitude",
        term: "electrostatic_field_peak_magnitude",
    },
    Domain {
        name: "magnetostatic",
        scalar: "vector_potential",
        vector_x: "magnetic_field_strength_x",
        vector_y: "magnetic_field_strength_y",
        vector_source: "elements",
        vector_scan: 4,
        peak: "magnetostatic_field_peak_magnitude",
        term: "magnetostatic_field_peak_magnitude",
    },
    Domain {
        name: "thermo",
        scalar: "temperature_delta",
        vector_x: "ux",
        vector_y: "uy",
        vector_source: "nodes",
        vector_scan: 2,
        peak: "thermo_displacement_peak_magnitude",
        term: "thermo_temperature_delta_max",
    },
];

fn domain_payload(domain: Domain) -> Value {
    let node = json!({domain.scalar:3.0,"heat_load":0.0,"charge_density":0.0,
        "current_density":0.0,"ux":3.0,"uy":4.0});
    let element = json!({domain.vector_x:3.0,domain.vector_y:4.0,
        "temperature_gradient_x":0.0,"temperature_gradient_y":0.0,
        "energy_density":0.25,"von_mises_stress":0.25,
        "thermal_strain":0.0,"mechanical_strain":0.0,"total_strain":0.0});
    json!({"nodes":vec![node;129],"elements":vec![element;129]})
}

#[test]
fn shared_domain_vector_scans_cancel_after_scalar_groups_without_partial_results() {
    for domain in DOMAINS {
        let healthy = domain_payload(domain);
        let mut bad = healthy.clone();
        bad[domain.vector_source][100][domain.vector_y] = Value::Null;
        let control = SolverControl::default();
        let result = with_solver_observer(
            &control,
            cancel_on_scan(&control, domain.vector_scan, 64),
            || extract(domain.name, bad.clone()),
        );
        assert_cancelled(result, &control, 64);
        let fresh = extract(domain.name, healthy).unwrap();
        assert_eq!(fresh[domain.peak], 5.0, "{}", domain.name);
        assert_eq!(fresh["diagnostic_node_count"], 129);
        let fresh_error = extract(domain.name, bad).unwrap_err();
        assert!(fresh_error.contains(domain.vector_y), "{fresh_error}");
        assert!(!fresh_error.contains("cancelled"), "{fresh_error}");
    }
}

#[test]
fn thermo_component_fallback_cancels_without_publishing_earlier_stress_peaks() {
    let mut healthy = domain_payload(DOMAINS[3]);
    for element in healthy["elements"].as_array_mut().unwrap() {
        let row = element.as_object_mut().unwrap();
        row.remove("von_mises_stress");
        row.insert("stress_x".into(), json!(3.0));
        row.insert("stress_y".into(), json!(4.0));
        row.insert("stress_xy".into(), json!(-5.0));
    }
    let mut bad = healthy.clone();
    bad["elements"][100]["stress_xy"] = Value::Null;
    let control = SolverControl::default();
    // Temperature, displacement, absent scalar stress, then x/y/z/xy components.
    let result = with_solver_observer(&control, cancel_on_scan(&control, 7, 64), || {
        extract("thermo", bad.clone())
    });
    assert_cancelled(result, &control, 64);
    let fresh = extract("thermo", healthy).unwrap();
    assert_eq!(fresh["thermo_stress_peak"], -5.0);
    assert_eq!(fresh["thermo_displacement_peak_magnitude"], 5.0);
    let error = extract("thermo", bad).unwrap_err();
    assert!(error.contains("payload.elements[100].stress_xy"), "{error}");
    assert!(!error.contains("cancelled"), "{error}");
}

#[test]
fn diagnostic_graph_error_policy_cannot_publish_cancelled_work_and_fresh_run_recovers() {
    let cases = DOMAINS
        .into_iter()
        .map(|domain| {
            (
                domain.name,
                format!(
                    "transform.score_{}_quality",
                    if domain.name == "thermo" {
                        "thermal"
                    } else {
                        domain.name
                    }
                ),
                domain.term,
                domain_payload(domain),
            )
        })
        .chain(std::iter::once((
            "stokes_flow",
            "transform.score_cfd_quality".into(),
            "cfd_velocity_span",
            cfd(129, 129),
        )));
    for (domain, quality, term, input) in cases {
        let request = |recover| {
            diagnostic_chain::request(
                domain,
                &quality,
                json!({"enabled_terms":[term],"targets":{term:10.0}}),
                input.clone(),
                recover,
            )
        };
        let control = SolverControl::default();
        let outcome = with_solver_observer(&control, cancel_on_scan(&control, 1, 64), || {
            run_workflow_graph(request(false))
        });
        let error = match outcome {
            Err(error) => error,
            Ok(_) => panic!("cancelled graph must not publish a completed run"),
        };
        assert!(
            error.contains("cancelled at result_diagnostics after 64 steps"),
            "{error}"
        );

        let control = SolverControl::default();
        let mut internal_run = None;
        let error = with_solver_observer(&control, cancel_on_scan(&control, 1, 64), || {
            internal_run = Some(run_workflow_graph(request(true))?);
            Ok::<(), String>(())
        })
        .unwrap_err();
        assert!(
            error.contains("cancelled at result_diagnostics after 64 steps"),
            "{error}"
        );
        // Inspect the inner trace only; the cancelled execution scope rejects its publication.
        let interrupted = internal_run.unwrap();
        assert!(control.was_interrupted());
        assert_eq!(interrupted.failed_nodes, ["diagnose"]);
        assert_eq!(interrupted.skipped_nodes.len(), 3);
        for node in ["quality", "objective", "decision"] {
            assert!(interrupted.skipped_nodes.iter().any(|id| id == node));
        }
        for node in ["diagnose", "quality", "objective", "decision"] {
            assert!(
                !interrupted
                    .artifacts
                    .contains_key(&format!("{node}.summary"))
            );
        }
        assert_eq!(interrupted.artifacts["raw.payload"], input);
        assert_eq!(
            interrupted.artifacts["independent_output.value"],
            json!({"value":7})
        );
        assert_eq!(
            interrupted
                .node_runs
                .iter()
                .find(|node| node.node_id == "diagnose")
                .unwrap()
                .status,
            WorkflowNodeRunStatus::Failed
        );

        let fresh = SolverControl::default();
        let recovered = with_solver_control(&fresh, || run_workflow_graph(request(false))).unwrap();
        assert!(recovered.failed_nodes.is_empty());
        assert!(recovered.skipped_nodes.is_empty());
        assert_eq!(
            recovered.artifacts["decision.summary"]["composite_quality_ready"],
            true
        );
        assert!(!fresh.was_interrupted());
        assert_eq!(interrupted.failed_nodes, ["diagnose"]);
        assert!(!interrupted.artifacts.contains_key("decision.summary"));
    }
}

#[test]
fn real_stokes_result_final_diagnostic_cancellation_allows_explicit_reduction_rerun() {
    let nodes = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            json!({"id":format!("n{i}"),"x":x,"y":y,
            "fix_velocity_x":true,"fix_velocity_y":true,"velocity_x":y,
            "velocity_y":0.0,"fix_pressure":true,"pressure":0.0})
        })
        .collect::<Vec<_>>();
    let model = json!({"nodes":nodes,
        "elements":[{"id":"e0","node_i":0,"node_j":1,"node_k":2,"node_l":3,
            "thickness":1.0,"viscosity":1.0,"density":1.0}]});
    let initial = run_workflow_graph(diagnostic_chain::with_solver(
        diagnostic_chain::request(
            "stokes_flow",
            "transform.score_cfd_quality",
            json!({"enabled_terms":["cfd_velocity_span"]}),
            model,
            false,
        ),
        "solve.stokes_flow_quad_2d",
    ))
    .unwrap();
    assert_eq!(
        initial.artifacts["decision.summary"]["composite_quality_ready"],
        true
    );
    let raw = initial.artifacts["raw.payload"].clone();
    let control = SolverControl::default();
    let result = with_solver_observer(&control, cancel_on_scan(&control, 2, 1), || {
        extract("stokes_flow", raw.clone())
    });
    assert_cancelled(result, &control, 1);
    let fresh = extract("stokes_flow", raw.clone()).unwrap();
    assert_eq!(fresh["diagnostic_node_count"], 4);
    assert_eq!(fresh["diagnostic_element_count"], 1);
    assert_eq!(fresh["cfd_velocity_max"], 1.0);
    assert_eq!(
        fresh["cfd_divergence_error_peak"],
        raw["elements"][0]["divergence_error"]
    );
    assert_eq!(
        fresh["cfd_viscous_dissipation_total"],
        raw["elements"][0]["viscous_dissipation"]
    );
}
