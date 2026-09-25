#[path = "support/diagnostic_chain.rs"]
mod diagnostic_chain;

use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{WorkflowGraphRunRequest, WorkflowGraphRunResult, WorkflowNodeRunStatus};
use serde_json::{Value, json};

fn payload(domain: &str) -> Value {
    if domain == "thermo" {
        json!({"nodes":[{"temperature_delta":10.0,"ux":3.0,"uy":4.0},
            {"temperature_delta":20.0,"ux":5.0,"uy":12.0}],
            "elements":[{"von_mises":30.0,"mechanical_strain_x":-0.1},
                {"von_mises":50.0,"mechanical_strain_x":-0.2}]})
    } else {
        json!({"nodes":[{"concentration":0.0,"source":0.0},{"concentration":1.0,"source":0.0}],
            "elements":[{"total_flux":-1.0,"peclet_number":1.0},{"total_flux":1.5,"peclet_number":2.0}]})
    }
}

fn request(domain: &str, input: Value, recover: bool) -> WorkflowGraphRunRequest {
    let (quality, config) = if domain == "thermo" {
        (
            "thermal",
            json!({"enabled_terms":["thermo_temperature_delta_max","thermo_stress_peak"]}),
        )
    } else {
        ("transport", json!({}))
    };
    diagnostic_chain::request(
        domain,
        &format!("transform.score_{quality}_quality"),
        config,
        input,
        recover,
    )
}

fn invalid_cases() -> Vec<(&'static str, Value, &'static str)> {
    let mut cases = Vec::new();
    for domain in ["thermo", "transport"] {
        let mut input = payload(domain);
        input["converged"] = json!(false);
        cases.push((domain, input, "payload.converged"));
        let mut input = payload(domain);
        input["elements"][1] = json!({});
        cases.push((domain, input, "payload.elements[1]"));
        let mut input = payload(domain);
        input["nodes"][1] = Value::Null;
        cases.push((domain, input, "payload.nodes[1]"));
        cases.push((
            domain,
            json!({"nodes":[],"elements":[]}),
            "did not find any diagnostic fields",
        ));
    }
    let mut input = payload("thermo");
    input["elements"][1]["mechanical_strain_x"] = Value::Null;
    cases.push(("thermo", input, "payload.elements[1].mechanical_strain_x"));
    let mut input = payload("thermo");
    input["elements"][1]["von_mises_stress"] = json!("50");
    input["max_stress"] = json!(1.0);
    cases.push(("thermo", input, "payload.elements[1].von_mises_stress"));
    let mut input = payload("transport");
    input["nodes"][1].as_object_mut().unwrap().remove("source");
    cases.push(("transport", input, "payload.nodes[1].source"));
    let mut input = payload("transport");
    input["elements"][1]["total_flux"] = Value::Null;
    input["elements"][1]["flux"] = json!(1.0);
    cases.push(("transport", input, "payload.elements[1].total_flux"));
    for (domain, field, path) in [
        (
            "thermo",
            "temperature_delta",
            "thermo_temperature_delta_span",
        ),
        ("transport", "concentration", "transport_concentration_span"),
    ] {
        let mut input = payload(domain);
        input["nodes"][0][field] = json!(-1e308);
        input["nodes"][1][field] = json!(1e308);
        cases.push((domain, input, path));
    }
    cases
}

fn assert_isolated(run: &WorkflowGraphRunResult, raw: &Value, path: &str) {
    assert_eq!(run.failed_nodes, vec!["diagnose"]);
    assert_eq!(run.skipped_nodes.len(), 3);
    for node in ["quality", "objective", "decision"] {
        assert!(run.skipped_nodes.iter().any(|id| id == node));
    }
    for node in ["diagnose", "quality", "objective", "decision"] {
        assert!(!run.artifacts.contains_key(&format!("{node}.summary")));
    }
    assert_eq!(&run.artifacts["raw.payload"], raw);
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
    assert!(trace.error_message.as_ref().unwrap().contains(path));
}

fn assert_ready(run: &WorkflowGraphRunResult) {
    assert!(run.failed_nodes.is_empty());
    assert!(run.skipped_nodes.is_empty());
    assert_eq!(
        run.artifacts["decision.summary"]["composite_quality_missing_metric_count"],
        0
    );
    assert_eq!(
        run.artifacts["decision.summary"]["composite_quality_ready"],
        true
    );
}

#[test]
fn incomplete_thermo_and_transport_diagnostics_cannot_publish_quality() {
    for (domain, input, path) in invalid_cases() {
        let error = run_workflow_graph(request(domain, input, false)).unwrap_err();
        assert!(error.contains("workflow node diagnose failed"), "{error}");
        assert!(error.contains(path), "{error}");
    }
}

#[test]
fn thermo_and_transport_recovery_retains_raw_evidence_and_independent_work() {
    for (domain, input, path) in invalid_cases() {
        let run = run_workflow_graph(request(domain, input.clone(), true)).unwrap();
        assert_isolated(&run, &input, path);
    }
}

#[test]
fn corrected_thermo_and_transport_samples_restore_the_complete_decision_chain() {
    for (domain, input, _) in invalid_cases() {
        assert_eq!(
            run_workflow_graph(request(domain, input, true))
                .unwrap()
                .failed_nodes
                .len(),
            1
        );
        assert_ready(&run_workflow_graph(request(domain, payload(domain), false)).unwrap());
    }
}

#[test]
fn missing_transport_source_keeps_the_composite_objective_blocked() {
    let mut input = payload("transport");
    for node in input["nodes"].as_array_mut().unwrap() {
        node.as_object_mut().unwrap().remove("source");
    }
    let run = run_workflow_graph(request("transport", input, false)).unwrap();
    assert!(run.failed_nodes.is_empty());
    assert!(
        run.artifacts["diagnose.summary"]
            .get("transport_source_sum")
            .is_none()
    );
    assert_eq!(
        run.artifacts["decision.summary"]["composite_quality_missing_metric_count"],
        1
    );
    assert_eq!(
        run.artifacts["decision.summary"]["composite_quality_ready"],
        false
    );
}

#[test]
fn missing_thermo_stress_keeps_the_composite_objective_blocked() {
    let mut input = payload("thermo");
    for element in input["elements"].as_array_mut().unwrap() {
        element.as_object_mut().unwrap().remove("von_mises");
    }
    let run = run_workflow_graph(request("thermo", input, false)).unwrap();
    assert!(run.failed_nodes.is_empty());
    assert!(
        run.artifacts["diagnose.summary"]
            .get("thermo_stress_peak")
            .is_none()
    );
    assert_eq!(
        run.artifacts["decision.summary"]["composite_quality_missing_metric_count"],
        1
    );
    assert_eq!(
        run.artifacts["decision.summary"]["composite_quality_ready"],
        false
    );
}

fn thermal_model(quad: bool) -> Value {
    let coordinates = if quad {
        vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
    } else {
        vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]
    };
    let nodes = coordinates
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            json!({
                "id":format!("n{i}"),"x":x,"y":y,"fix_x":true,"fix_y":true,
                "load_x":0.0,"load_y":0.0,"temperature_delta":40.0
            })
        })
        .collect::<Vec<_>>();
    let mut element = json!({"id":"e0","node_i":0,"node_j":1,"node_k":2,
        "thickness":0.02,"youngs_modulus":1000.0,"poisson_ratio":0.25,"thermal_expansion":0.00001});
    if quad {
        element["node_l"] = json!(3);
    }
    json!({"nodes":nodes,"elements":[element]})
}

fn transport_model() -> Value {
    json!({"nodes":[
        {"id":"c0","x":0.0,"fix_concentration":true,"concentration":1.0,"source":0.0},
        {"id":"c1","x":0.5,"fix_concentration":false,"concentration":0.0,"source":0.0},
        {"id":"c2","x":1.0,"fix_concentration":true,"concentration":0.2,"source":0.0}],
        "elements":[
        {"id":"e0","node_i":0,"node_j":1,"area":0.02,"diffusivity":1.0,"velocity":0.2},
        {"id":"e1","node_i":1,"node_j":2,"area":0.02,"diffusivity":1.0,"velocity":0.2}]})
}

#[test]
fn real_thermo_and_transport_solvers_feed_checked_quality_and_recover_after_corruption() {
    for (domain, solver, model) in [
        (
            "thermo",
            "solve.thermal_plane_triangle_2d",
            thermal_model(false),
        ),
        ("thermo", "solve.thermal_plane_quad_2d", thermal_model(true)),
        (
            "transport",
            "solve.advection_diffusion_bar_1d",
            transport_model(),
        ),
    ] {
        let req = diagnostic_chain::with_solver(request(domain, model, false), solver);
        let run = run_workflow_graph(req).unwrap();
        assert_ready(&run);
        let raw = &run.artifacts["raw.payload"];
        let diagnostic = &run.artifacts["diagnose.summary"];
        assert_eq!(
            diagnostic["diagnostic_node_count"],
            raw["nodes"].as_array().unwrap().len()
        );
        assert_eq!(
            diagnostic["diagnostic_element_count"],
            raw["elements"].as_array().unwrap().len()
        );
        let bad_path = if domain == "thermo" {
            assert_eq!(
                diagnostic["thermo_stress_peak"],
                raw["elements"][0]["von_mises"]
            );
            assert_eq!(
                diagnostic["thermo_thermal_strain_peak"],
                raw["elements"][0]["thermal_strain"]
            );
            assert_eq!(
                diagnostic["thermo_mechanical_strain_peak"],
                raw["elements"][0]["mechanical_strain_y"]
            );
            assert_eq!(diagnostic["thermo_temperature_delta_max"], 40.0);
            let expected_stress = 1000.0 * 0.00001 * 40.0 / (1.0 - 0.25);
            assert!(
                (diagnostic["thermo_stress_peak"].as_f64().unwrap() / expected_stress - 1.0).abs()
                    < 1e-12
            );
            "payload.elements[0].mechanical_strain_x"
        } else {
            assert_eq!(diagnostic["transport_source_sum"], 0.0);
            let flux = raw["elements"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["total_flux"].as_f64().unwrap())
                .max_by(|a, b| a.abs().total_cmp(&b.abs()))
                .unwrap();
            assert_eq!(diagnostic["transport_total_flux_peak"], flux);
            "payload.nodes[0].source"
        };
        let mut corrupt = raw.clone();
        if domain == "thermo" {
            corrupt["elements"][0]["mechanical_strain_x"] = Value::Null;
        } else {
            corrupt["nodes"][0]["source"] = Value::Null;
        }
        let failed = run_workflow_graph(request(domain, corrupt.clone(), true)).unwrap();
        assert_isolated(&failed, &corrupt, bad_path);
        assert_ready(&run_workflow_graph(request(domain, raw.clone(), false)).unwrap());
    }
}
