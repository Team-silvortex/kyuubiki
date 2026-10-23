use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};

fn input(kind: &str) -> Value {
    let translation = matches!(kind, "translation" | "near-axis");
    let rotation = kind == "rotation";
    let epsilon = if kind == "near-axis" {
        2.0_f64.powi(-60)
    } else {
        1.0
    };
    let direction = vec![json!({"id":"guide", "node":1, "direction":[1.0, epsilon, 0.0]})];
    json!({
        "nodes":(0..2).map(|index| {
            let fixed = index == 0;
            json!({"id":format!("n{index}"), "x":index as f64 * 1.7, "y":0.0, "z":0.0,
                "fix_x":fixed || rotation, "fix_y":fixed || rotation, "fix_z":fixed || rotation || translation,
                "fix_rx":fixed || translation, "fix_ry":fixed || translation, "fix_rz":fixed || translation || rotation,
                "load_x":if fixed || rotation { 0.0 } else { 360000.0 },
                "load_y":if fixed || rotation { 0.0 } else { -1400.0 }, "load_z":0.0,
                "moment_x":if fixed || translation { 0.0 } else { 68000.0 },
                "moment_y":if fixed || translation { 0.0 } else { -24000.0 }, "moment_z":0.0,
                "temperature_delta":if fixed { 13.0 } else { 47.0 }})
        }).collect::<Vec<_>>(),
        "elements":[{"id":"member", "node_i":0, "node_j":1, "local_y_axis":[0.0, 1.0, 0.0],
            "area":0.014, "youngs_modulus":205e9, "shear_modulus":79e9, "torsion_constant":4.9e-6,
            "moment_of_inertia_y":7.1e-6, "moment_of_inertia_z":6.2e-6,
            "section_modulus_y":1.4e-4, "section_modulus_z":1.1e-4, "thermal_expansion":11.4e-6,
            "section_depth_y":0.22, "section_depth_z":0.18,
            "temperature_gradient_y":19.0, "temperature_gradient_z":-11.0}],
        "directional_springs":[{"id":"spring", "node":1, "direction":[1.0, -1.0, 0.0], "stiffness":300.0}],
        "directional_rotational_springs":[{"id":"rotational-spring", "node":1,
            "direction":[1.0, -1.0, 0.0], "stiffness":350.0}],
        "directional_constraints":if translation { direction.clone() } else { vec![] },
        "directional_rotational_constraints":if rotation { direction } else { vec![] }
    })
}

fn check(kind: &str) {
    let input = input(kind);
    let decoded = serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
    let baseline = run_solve_operator("solve.thermal_frame_3d", decoded).unwrap();
    assert!(baseline["total_strain_energy"].as_f64().unwrap() > 0.0);
    if kind == "near-axis" {
        let ux = baseline["nodes"][1]["ux"].as_f64().unwrap();
        let uy = baseline["nodes"][1]["uy"].as_f64().unwrap();
        assert_ne!(ux, 0.0);
        assert!((ux / (-2.0_f64.powi(-60) * uy) - 1.0).abs() < 1.0e-12);
    }
    for (field, value) in [
        ("directional_constraints", "displacement"),
        ("directional_rotational_constraints", "rotation"),
    ] {
        for result in baseline[field].as_array().unwrap() {
            assert!(result[value].as_f64().unwrap().abs() < 1.0e-12);
        }
    }
    for (stage, completed) in [
        (SolverStage::ConstraintValidate, 0),
        (SolverStage::ConstraintValidateRow, 0),
        (SolverStage::ConstraintValidate, 36),
    ] {
        for workflow in [false, true] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let result = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == stage && point.completed_steps == completed {
                        cancel.request_cancel();
                    }
                },
                || {
                    if workflow {
                        run_workflow_graph(graph(input.clone())).map(|_| ())
                    } else {
                        run_solve_operator("solve.thermal_frame_3d", input.clone()).map(|_| ())
                    }
                },
            );
            assert!(result.unwrap_err().contains("constraint_validate"));
            assert!(control.was_interrupted());
        }
    }
    assert_eq!(
        run_solve_operator("solve.thermal_frame_3d", input.clone()).unwrap(),
        baseline
    );
    let workflow = run_workflow_graph(graph(input.clone())).unwrap();
    assert!(workflow.failed_nodes.is_empty());
    assert_eq!(workflow.artifacts["output.result"], baseline);

    let mut cold = input;
    for node in cold["nodes"].as_array_mut().unwrap() {
        node["temperature_delta"] = json!(0.0);
    }
    cold["elements"][0]["temperature_gradient_y"] = json!(0.0);
    cold["elements"][0]["temperature_gradient_z"] = json!(0.0);
    let cold = run_solve_operator("solve.thermal_frame_3d", cold).unwrap();
    assert_ne!(cold["elements"], baseline["elements"]);
}

#[test]
fn thermal_frame_original_axis_equations_gate_dispatch_and_workflow() {
    check("axis");
}

#[test]
fn thermal_frame_original_directional_translation_gates_dispatch_and_workflow() {
    check("translation");
}

#[test]
fn thermal_frame_original_directional_rotation_gates_dispatch_and_workflow() {
    check("rotation");
}

#[test]
fn thermal_frame_original_near_axis_support_keeps_small_components_in_workflows() {
    check("near-axis");
}

#[test]
fn thermal_frame_original_all_fixed_thermal_reactions_remain_admissible() {
    let mut input = input("axis");
    for node in input["nodes"].as_array_mut().unwrap() {
        for (key, value) in node.as_object_mut().unwrap() {
            if key.starts_with("fix_") {
                *value = json!(true);
            }
            if key.starts_with("load_") || key.starts_with("moment_") {
                *value = json!(0.0);
            }
        }
    }
    let direct = run_solve_operator("solve.thermal_frame_3d", input.clone()).unwrap();
    for node in direct["nodes"].as_array().unwrap() {
        for field in ["ux", "uy", "uz", "rx", "ry", "rz"] {
            assert_eq!(node[field], 0.0);
        }
    }
    assert!(direct["max_stress"].as_f64().unwrap() > 0.0);
    let run = run_workflow_graph(graph(input)).unwrap();
    assert!(run.failed_nodes.is_empty());
    assert_eq!(run.artifacts["output.result"], direct);
}

fn reaction_input(rotation: bool, sign: f64) -> Value {
    let mut model = input("axis");
    for node in model["nodes"].as_array_mut().unwrap() {
        node["temperature_delta"] = json!(0.0);
        for field in [
            "load_x", "load_y", "load_z", "moment_x", "moment_y", "moment_z",
        ] {
            node[field] = json!(0.0);
        }
    }
    for field in ["fix_x", "fix_y", "fix_z"] {
        model["nodes"][1][field] = json!(rotation);
    }
    for field in ["fix_rx", "fix_ry", "fix_rz"] {
        model["nodes"][1][field] = json!(!rotation);
    }
    for field in ["temperature_gradient_y", "temperature_gradient_z"] {
        model["elements"][0][field] = json!(0.0);
    }
    let fields = if rotation {
        ["moment_x", "moment_y", "moment_z"]
    } else {
        ["load_x", "load_y", "load_z"]
    };
    for (field, value) in fields.into_iter().zip([
        1.2e308,
        2.0_f64.sqrt().recip() * 1.0e308,
        2.0_f64.sqrt().recip() * 1.0e308,
    ]) {
        model["nodes"][1][field] = json!(sign * value);
    }
    let constraints = if rotation {
        "directional_rotational_constraints"
    } else {
        "directional_constraints"
    };
    model[constraints] = json!([[1.0, 0.0, 0.0], [-1.0, 1.0, 0.0], [1.0, 0.0, 1.0]]
        .into_iter().enumerate().map(|(index, direction)|
            json!({"id":format!("support-{index}"), "node":1, "direction":direction})).collect::<Vec<_>>());
    model
}

fn check_reactions(rotation: bool) {
    for sign in [-1.0, 1.0] {
        let input = reaction_input(rotation, sign);
        let decoded = serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
        let baseline = run_solve_operator("solve.thermal_frame_3d", decoded).unwrap();
        let field = if rotation {
            "directional_rotational_constraints"
        } else {
            "directional_constraints"
        };
        let reaction = if rotation {
            "reaction_moment"
        } else {
            "reaction_force"
        };
        let constraints = baseline[field].as_array().unwrap();
        for (constraint, expected) in constraints.iter().zip([1.2e308, 1.0e308, 1.0e308]) {
            assert!(
                (constraint[reaction].as_f64().unwrap() / (-sign * expected) - 1.0).abs() < 2.0e-14
            );
        }
        for (axis, expected) in [1.2, 2.0_f64.sqrt().recip(), 2.0_f64.sqrt().recip()]
            .into_iter()
            .enumerate()
        {
            let total = constraints
                .iter()
                .map(|constraint| {
                    (constraint[reaction].as_f64().unwrap() / 1.0e308)
                        * constraint["direction"][axis].as_f64().unwrap()
                })
                .sum::<f64>();
            assert!((total + sign * expected).abs() < 2.0e-14);
        }
        for (stage, completed) in [
            (SolverStage::SparseResidual, 0),
            (SolverStage::ResidualValidate, 12),
            (SolverStage::DenseSubstitution, 0),
            (SolverStage::DenseSubstitution, 6),
        ] {
            for workflow in [false, true] {
                let control = SolverControl::default();
                let cancel = control.clone();
                let result = with_solver_observer(
                    &control,
                    move |point| {
                        if point.stage == stage && point.completed_steps == completed {
                            cancel.request_cancel();
                        }
                    },
                    || {
                        if workflow {
                            run_workflow_graph(graph(input.clone())).map(|_| ())
                        } else {
                            run_solve_operator("solve.thermal_frame_3d", input.clone()).map(|_| ())
                        }
                    },
                );
                assert!(result.unwrap_err().contains("cancelled"));
                assert!(control.was_interrupted());
            }
        }
        assert_eq!(
            run_solve_operator("solve.thermal_frame_3d", input.clone()).unwrap(),
            baseline
        );
        let run = run_workflow_graph(graph(input)).unwrap();
        assert!(run.failed_nodes.is_empty());
        assert_eq!(run.artifacts["output.result"], baseline);
    }
}

#[test]
fn thermal_frame_reactions_large_translation_dispatch_and_workflow_balance() {
    check_reactions(false);
}

#[test]
fn thermal_frame_reactions_large_rotation_dispatch_and_workflow_balance() {
    check_reactions(true);
}

#[test]
fn thermal_frame_reactions_unrepresentable_coefficients_fail_workflows_then_replay() {
    for rotation in [false, true] {
        let field = if rotation {
            "directional_rotational_constraints"
        } else {
            "directional_constraints"
        };
        let load = if rotation { "moment_y" } else { "load_y" };
        let mut input = reaction_input(rotation, 1.0);
        input[field][1]["direction"] = json!([1.0, 1.0e-9, 0.0]);
        input["nodes"][1][load] = json!(1.0e300);
        for workflow in [false, true] {
            let error = if workflow {
                run_workflow_graph(graph(input.clone())).map(|_| ())
            } else {
                run_solve_operator("solve.thermal_frame_3d", input.clone()).map(|_| ())
            }
            .unwrap_err();
            assert!(
                error.contains("constraint reaction") && error.contains("representable"),
                "{error}"
            );
        }
        input["nodes"][1][load] = json!(1.0);
        let direct = run_solve_operator("solve.thermal_frame_3d", input.clone()).unwrap();
        let run = run_workflow_graph(graph(input)).unwrap();
        assert!(run.failed_nodes.is_empty());
        assert_eq!(run.artifacts["output.result"], direct);
    }
}

fn graph(input: Value) -> WorkflowGraphRunRequest {
    serde_json::from_value(json!({
        "graph": {
            "schema_version":"kyuubiki.workflow-graph/v1", "id":"projected-thermal-frame",
            "name":"Original projected equation admission", "version":"1.0.0",
            "entry_nodes":["input"], "output_nodes":["output"],
            "nodes":[
                {"id":"input", "kind":"input", "inputs":[], "outputs":[{"id":"model", "artifact_type":"study_model/thermal_frame_3d"}]},
                {"id":"solve", "kind":"solve", "operator_id":"solve.thermal_frame_3d",
                 "inputs":[{"id":"model", "artifact_type":"study_model/thermal_frame_3d"}], "outputs":[{"id":"result", "artifact_type":"result/thermal_frame_3d"}]},
                {"id":"output", "kind":"output", "outputs":[], "inputs":[{"id":"result", "artifact_type":"result/thermal_frame_3d"}]}
            ],
            "edges":[
                {"id":"in", "from":{"node":"input", "port":"model"}, "to":{"node":"solve", "port":"model"}, "artifact_type":"study_model/thermal_frame_3d"},
                {"id":"out", "from":{"node":"solve", "port":"result"}, "to":{"node":"output", "port":"result"}, "artifact_type":"result/thermal_frame_3d"}
            ]
        }, "input_artifacts":{"input":input}
    })).unwrap()
}
