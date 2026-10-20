use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use serde_json::{Value, json};

fn model(contact: bool, iterations: usize) -> Value {
    let mut value = json!({
        "nodes":[{"id":"base", "x":0.0, "fix_x":true, "load_x":0.0},
                 {"id":"tip", "x":1.0, "fix_x":false, "load_x":if contact { 2.0 } else { 3.0 }}],
        "elements":[{"id":"spring", "node_i":0, "node_j":1, "stiffness":1.0,
                     "cubic_stiffness":if contact { 0.0 } else { 1.0 }}],
        "load_steps":if contact { 4 } else { 1 }, "max_iterations":iterations, "tolerance":1e-12
    });
    if contact {
        value["contacts"] = json!([{"id":"stop", "node":1, "gap":1.0, "normal_stiffness":10.0}]);
    }
    value
}

fn run(contact: bool, input: Value) -> Value {
    let family = if contact {
        "contact_gap_1d"
    } else {
        "nonlinear_spring_1d"
    };
    let input_type = format!("study_model/{family}");
    let output_type = format!("result/{family}");
    let request: WorkflowGraphRunRequest = serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1", "id":"spring-recovery",
            "name":"Spring recovery", "version":"1.0.0", "entry_nodes":["input"], "output_nodes":["output"],
            "nodes":[
                {"id":"input", "kind":"input", "inputs":[], "outputs":[{"id":"model", "artifact_type":input_type}]},
                {"id":"solve", "kind":"solve", "operator_id":format!("solve.{family}"),
                 "inputs":[{"id":"model", "artifact_type":input_type}], "outputs":[{"id":"result", "artifact_type":output_type}]},
                {"id":"output", "kind":"output", "inputs":[{"id":"result", "artifact_type":output_type}], "outputs":[]}
            ],
            "edges":[
                {"id":"in", "from":{"node":"input", "port":"model"}, "to":{"node":"solve", "port":"model"}, "artifact_type":input_type},
                {"id":"out", "from":{"node":"solve", "port":"result"}, "to":{"node":"output", "port":"result"}, "artifact_type":output_type}
            ]
        },
        "input_artifacts":{"input":input}
    })).unwrap();
    run_workflow_graph(request)
        .unwrap()
        .artifacts
        .remove("output.result")
        .unwrap()
}

#[test]
fn workflow_preserves_partial_load_and_failed_trial_diagnostics() {
    for contact in [false, true] {
        let result = run(contact, model(contact, 1));
        assert_eq!(result["converged"], false);
        assert_eq!(
            result["achieved_load_factor"],
            if contact { 0.5 } else { 0.0 }
        );
        assert_eq!(result["nodes"][1]["ux"], if contact { 1.0 } else { 0.0 });
        assert_eq!(result["residual_norm"], 0.0);
        assert_eq!(
            result["steps"].as_array().unwrap().last().unwrap()["residual_norm"],
            if contact { 0.1875 } else { 1.875 }
        );
        if contact {
            assert_eq!(result["contacts"][0]["active"], false);
        }
    }
}

#[test]
fn workflow_replay_reaches_full_load_and_matches_direct_engine_execution() {
    for contact in [false, true] {
        let _ = run(contact, model(contact, 1));
        let input = model(contact, 32);
        let result = run(contact, input.clone());
        assert_eq!(result["converged"], true);
        assert_eq!(result["achieved_load_factor"], 1.0);
        let operator = if contact {
            "solve.contact_gap_1d"
        } else {
            "solve.nonlinear_spring_1d"
        };
        assert_eq!(result, run_solve_operator(operator, input).unwrap());
    }
}

#[test]
fn workflow_strong_hardening_uses_the_solver_globalization_without_engine_special_cases() {
    for contact in [false, true] {
        let mut input = model(contact, 12);
        input["elements"][0]["cubic_stiffness"] = json!(1e18);
        input["nodes"][1]["load_x"] = json!(if contact { 1.500001 } else { 1.000001 });
        input["load_steps"] = json!(1);
        if contact {
            input["contacts"][0]["gap"] = json!(0.5e-6);
            input["contacts"][0]["normal_stiffness"] = json!(1e6);
        }
        let result = run(contact, input.clone());
        let operator = if contact {
            "solve.contact_gap_1d"
        } else {
            "solve.nonlinear_spring_1d"
        };
        assert_eq!(result, run_solve_operator(operator, input).unwrap());
        assert_eq!(result["converged"], true);
        assert_eq!(result["achieved_load_factor"], 1.0);
        assert!((result["nodes"][1]["ux"].as_f64().unwrap() / 1e-6 - 1.0).abs() < 1e-10);
        assert!(result["residual_norm"].as_f64().unwrap() <= 1e-12);
    }
}

#[test]
fn workflow_small_scale_tolerance_preserves_failure_then_successful_replay() {
    let unit = 2.0_f64.powi(-50);
    let tolerance = unit * 1e-12;
    for contact in [false, true] {
        let operator = if contact {
            "solve.contact_gap_1d"
        } else {
            "solve.nonlinear_spring_1d"
        };
        let mut input = model(contact, 1);
        input["nodes"][1]["load_x"] = json!(if contact { 2.0 } else { 3.0 } * unit);
        input["tolerance"] = json!(tolerance);
        if contact {
            input["contacts"][0]["gap"] = json!(unit);
        } else {
            input["elements"][0]["cubic_stiffness"] = json!(1.0 / (unit * unit));
        }
        let result = run(contact, input.clone());
        assert_eq!(result, run_solve_operator(operator, input.clone()).unwrap());
        assert_eq!(result["converged"], false);
        assert_eq!(
            result["achieved_load_factor"],
            if contact { 0.5 } else { 0.0 }
        );
        assert_eq!(result["nodes"][1]["ux"], if contact { unit } else { 0.0 });
        assert_eq!(result["residual_norm"], 0.0);
        let failed = result["steps"].as_array().unwrap().last().unwrap();
        assert!(failed["residual_norm"].as_f64().unwrap() > tolerance);

        input["max_iterations"] = json!(32);
        let replay = run(contact, input.clone());
        assert_eq!(replay, run_solve_operator(operator, input).unwrap());
        assert_eq!(replay["converged"], true);
        assert_eq!(replay["achieved_load_factor"], 1.0);
        assert_eq!(replay["input"]["tolerance"], tolerance);
        assert!(replay["residual_norm"].as_f64().unwrap() <= tolerance);
        assert!(replay["nodes"][1]["ux"].as_f64().unwrap() > 0.0);
    }
}

#[test]
fn workflow_equilibrium_does_not_depend_on_force_or_length_units() {
    for contact in [false, true] {
        let operator = if contact {
            "solve.contact_gap_1d"
        } else {
            "solve.nonlinear_spring_1d"
        };
        for force in [1e-80, 1.0, 1e80] {
            for length in [1e-30, 1.0, 1e30] {
                let mut input = model(contact, 32);
                input["nodes"][1]["x"] = json!(length);
                input["nodes"][1]["load_x"] = json!(if contact { 1.25 } else { 1.125 } * force);
                input["elements"][0]["stiffness"] = json!(4.0 * force / length);
                input["elements"][0]["cubic_stiffness"] =
                    json!(8.0 * (force / length) / (length * length));
                input["tolerance"] = json!(1e-12 * force);
                if contact {
                    input["contacts"][0]["gap"] = json!(0.125 * length);
                    input["contacts"][0]["normal_stiffness"] = json!(force / length);
                }
                let result = run(contact, input.clone());
                assert_eq!(result, run_solve_operator(operator, input).unwrap());
                assert_eq!(result["converged"], true);
                assert_eq!(result["achieved_load_factor"], 1.0);
                assert!((result["nodes"][1]["ux"].as_f64().unwrap() / length - 0.25).abs() < 1e-10);
                assert!(result["residual_norm"].as_f64().unwrap() <= 1e-12 * force);
            }
        }
    }
}

#[test]
fn workflow_softening_contact_keeps_finite_cancelling_constitutive_terms() {
    for (k, c, load, gap, penalty, expected) in [
        (3.0, -1.5, 2.5, 0.5, 2.0, 1.0),
        (2.5, -0.25, 3.5, 1.5, 1.0, 2.0),
    ] {
        for unit in [1.0, 2.0_f64.powi(1022)] {
            let mut input = model(true, 32);
            input["elements"][0]["stiffness"] = json!(k * unit);
            input["elements"][0]["cubic_stiffness"] = json!(c * unit);
            input["nodes"][1]["load_x"] = json!(load * unit);
            input["contacts"][0]["gap"] = json!(gap);
            input["contacts"][0]["normal_stiffness"] = json!(penalty * unit);
            input["load_steps"] = json!(1);
            input["tolerance"] = json!(2.0_f64.powi(-40) * unit);
            let result = run(true, input.clone());
            assert_eq!(
                result,
                run_solve_operator("solve.contact_gap_1d", input).unwrap()
            );
            assert_eq!(result["converged"], true);
            assert_eq!(result["achieved_load_factor"], 1.0);
            assert!((result["nodes"][1]["ux"].as_f64().unwrap() - expected).abs() < 1e-10);
            let tangent = result["elements"][0]["tangent_stiffness"].as_f64().unwrap() / unit;
            assert!(tangent < 0.0 && tangent + penalty > 0.0);
            assert!(result["residual_norm"].as_f64().unwrap() <= 2.0_f64.powi(-40) * unit);
        }
    }
}
