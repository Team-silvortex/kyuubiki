#[path = "support/diagnostic_chain.rs"]
mod diagnostic_chain;

use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{WorkflowGraphRunRequest, WorkflowNodeRunStatus};
use serde_json::{Value, json};

#[derive(Clone, Copy)]
struct Domain {
    name: &'static str,
    scalar: &'static str,
    x: &'static str,
    y: &'static str,
    term: &'static str,
}

const DOMAINS: [Domain; 3] = [
    Domain {
        name: "thermal",
        scalar: "temperature",
        x: "heat_flux_x",
        y: "heat_flux_y",
        term: "thermal_temperature_max",
    },
    Domain {
        name: "electrostatic",
        scalar: "potential",
        x: "electric_field_x",
        y: "electric_field_y",
        term: "electrostatic_field_peak_magnitude",
    },
    Domain {
        name: "magnetostatic",
        scalar: "vector_potential",
        x: "magnetic_field_strength_x",
        y: "magnetic_field_strength_y",
        term: "magnetostatic_field_peak_magnitude",
    },
];

fn payload(domain: Domain) -> Value {
    json!({"nodes":[{domain.scalar:1.0},{domain.scalar:3.0}],
        "elements":[{domain.x:3.0,domain.y:4.0},{domain.x:4.0,domain.y:3.0}]})
}

fn request(domain: Domain, input: Value, recover: bool) -> WorkflowGraphRunRequest {
    diagnostic_chain::request(
        domain.name,
        &format!("transform.score_{}_quality", domain.name),
        json!({"enabled_terms":[domain.term]}),
        input,
        recover,
    )
}

fn invalid_cases() -> Vec<(Domain, Value, String)> {
    let mut cases = Vec::new();
    for domain in DOMAINS {
        let mut bad = payload(domain);
        bad["nodes"][1][domain.scalar] = Value::Null;
        cases.push((domain, bad, format!("payload.nodes[1].{}", domain.scalar)));
        let mut bad = payload(domain);
        bad["elements"][1] = json!({});
        cases.push((domain, bad, "payload.elements[1]".into()));
        let mut bad = payload(domain);
        bad["converged"] = json!(false);
        cases.push((domain, bad, "payload.converged".into()));
        let mut bad = payload(domain);
        bad["nodes"][0][domain.scalar] = json!(-1e308);
        bad["nodes"][1][domain.scalar] = json!(1e308);
        cases.push((
            domain,
            bad,
            format!("{}_{}_span", domain.name, domain.scalar),
        ));
    }
    cases
}

#[test]
fn incomplete_domain_diagnostics_fail_before_quality_or_objective_publication() {
    for (domain, bad, path) in invalid_cases() {
        let error = run_workflow_graph(request(domain, bad, false)).unwrap_err();
        assert!(error.contains("workflow node diagnose failed"), "{error}");
        assert!(error.contains(&path), "{error}");
        assert!(!error.contains("panicked"), "{error}");
    }
}

#[test]
fn diagnostic_recovery_preserves_raw_evidence_and_independent_branches() {
    for (domain, bad, path) in invalid_cases() {
        let run = run_workflow_graph(request(domain, bad.clone(), true)).unwrap();
        assert_eq!(run.failed_nodes, vec!["diagnose"]);
        assert_eq!(run.skipped_nodes.len(), 3);
        for node in ["quality", "objective", "decision"] {
            assert!(run.skipped_nodes.iter().any(|id| id == node));
        }
        for node in ["diagnose", "quality", "objective", "decision"] {
            assert!(!run.artifacts.contains_key(&format!("{node}.summary")));
        }
        assert_eq!(run.artifacts["raw.payload"], bad);
        assert_eq!(
            run.artifacts["independent_output.value"],
            json!({"value":7})
        );
        let trace = run
            .node_runs
            .iter()
            .find(|trace| trace.node_id == "diagnose")
            .unwrap();
        assert_eq!(trace.status, WorkflowNodeRunStatus::Failed);
        assert!(trace.error_message.as_ref().unwrap().contains(&path));
    }
}

#[test]
fn corrected_domain_samples_replay_through_the_complete_quality_chain() {
    for (domain, bad, _) in invalid_cases() {
        assert_eq!(
            run_workflow_graph(request(domain, bad, true))
                .unwrap()
                .failed_nodes
                .len(),
            1
        );
        let run = run_workflow_graph(request(domain, payload(domain), false)).unwrap();
        assert!(run.failed_nodes.is_empty());
        assert!(run.skipped_nodes.is_empty());
        assert_eq!(
            run.artifacts["decision.summary"]["composite_quality_ready"],
            true
        );
        assert_eq!(
            run.artifacts["decision.summary"]["composite_quality_missing_metric_count"],
            0
        );
    }
}

#[test]
fn wholly_absent_optional_groups_leave_quality_visibly_blocked() {
    for domain in DOMAINS {
        let mut input = payload(domain);
        input[if domain.name == "thermal" {
            "nodes"
        } else {
            "elements"
        }] = json!([]);
        let run = run_workflow_graph(request(domain, input, false)).unwrap();
        assert!(run.failed_nodes.is_empty());
        assert_eq!(
            run.artifacts["decision.summary"]["composite_quality_ready"],
            false
        );
        assert!(
            run.artifacts["decision.summary"]["composite_quality_missing_metric_count"]
                .as_u64()
                .unwrap()
                > 0
        );
    }
}

fn model(domain: Domain, quad: bool) -> Value {
    let (fix, source, material) = match domain.name {
        "thermal" => ("fix_temperature", "heat_load", "conductivity"),
        "electrostatic" => ("fix_potential", "charge_density", "permittivity"),
        _ => ("fix_vector_potential", "current_density", "permeability"),
    };
    let coordinates = if quad {
        vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
    } else {
        vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]
    };
    let nodes = coordinates.into_iter().enumerate().map(|(index,(x,y))| json!({
        "id":format!("n{index}"),"x":x,"y":y,fix:index < 2,domain.scalar:index as f64,source:0.25
    })).collect::<Vec<_>>();
    let mut element =
        json!({"id":"e0","node_i":0,"node_j":1,"node_k":2,"thickness":1.0,material:1.0});
    if quad {
        element["node_l"] = json!(3);
    }
    json!({"nodes":nodes,"elements":[element]})
}

#[test]
fn real_triangle_and_quad_solvers_feed_checked_diagnostics_and_quality() {
    for domain in DOMAINS {
        for quad in [false, true] {
            let solver_domain = if domain.name == "thermal" {
                "heat"
            } else {
                domain.name
            };
            let shape = if quad { "quad" } else { "triangle" };
            let req = diagnostic_chain::with_solver(
                request(domain, model(domain, quad), false),
                &format!("solve.{solver_domain}_plane_{shape}_2d"),
            );
            let run = run_workflow_graph(req).unwrap();
            assert!(run.failed_nodes.is_empty());
            assert!(run.skipped_nodes.is_empty());
            let raw = &run.artifacts["raw.payload"];
            let diagnostic = &run.artifacts["diagnose.summary"];
            assert_eq!(
                diagnostic["diagnostic_node_count"],
                if quad { 4 } else { 3 }
            );
            assert_eq!(diagnostic["diagnostic_element_count"], 1);
            let max = raw["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row[domain.scalar].as_f64().unwrap())
                .fold(f64::NEG_INFINITY, f64::max);
            assert_eq!(
                diagnostic[format!("{}_{}_max", domain.name, domain.scalar)],
                max
            );
            let vector = if domain.name == "thermal" {
                "flux"
            } else {
                "field"
            };
            let expected = raw["elements"][0][domain.x]
                .as_f64()
                .unwrap()
                .hypot(raw["elements"][0][domain.y].as_f64().unwrap());
            let actual = diagnostic[format!("{}_{vector}_peak_magnitude", domain.name)]
                .as_f64()
                .unwrap();
            assert!((actual - expected).abs() < 1e-12);
            let objective = &run.artifacts["decision.summary"];
            assert_eq!(objective["composite_quality_missing_metric_count"], 0);
            assert_eq!(objective["composite_quality_ready"], true);
        }
    }
}
