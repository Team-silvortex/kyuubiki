use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};

#[test]
fn magnetic_original_constraints_gate_cancels_dispatch_and_workflow_then_replays() {
    for arms in [1, 4] {
        let input = json!({
            "nodes": (0..=arms).map(|index| json!({
                "id":format!("n{index}"), "x":index,
                "fix_magnetic_potential":index != 0,
                "magnetic_potential":2.0, "magnetomotive_source":if index == 0 { 1.0 } else { 0.0 }
            })).collect::<Vec<_>>(),
            "elements": (1..=arms).map(|index| json!({
                "id":format!("e{index}"), "node_i":0, "node_j":index,
                "area":1.0, "permeability":index as f64
            })).collect::<Vec<_>>()
        });
        for use_workflow in [false, true] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let result = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == SolverStage::ConstraintValidate && point.completed_steps == 0
                    {
                        cancel.request_cancel();
                    }
                },
                || {
                    if use_workflow {
                        run_workflow_graph(workflow(input.clone())).map(|_| ())
                    } else {
                        run_solve_operator("solve.magnetostatic_bar_1d", input.clone()).map(|_| ())
                    }
                },
            );
            assert!(result.unwrap_err().contains("constraint_validate"));
            assert!(control.was_interrupted());
            let direct = run_solve_operator("solve.magnetostatic_bar_1d", input.clone()).unwrap();
            let run = run_workflow_graph(workflow(input.clone())).unwrap();
            assert!(run.failed_nodes.is_empty());
            assert_eq!(run.artifacts["output.result"], direct);
        }
    }
}

#[test]
fn magnetic_prescribed_cancellation_survives_serialized_dispatch_and_workflow() {
    let delta = 2.0_f64.powi(-27);
    for arms in [1, 4] {
        for sign in [-1.0, 1.0] {
            let input = json!({
                "nodes": (0..=arms).map(|index| json!({
                    "id":format!("n{index}"), "x":index,
                    "fix_magnetic_potential":index != 0,
                    "magnetic_potential":if index == 1 { -sign * (1.0 - delta) } else { 0.0 },
                    "magnetomotive_source":if index == 0 { sign } else { 0.0 }
                })).collect::<Vec<_>>(),
                "elements": (1..=arms).map(|index| json!({
                    "id":format!("e{index}"), "node_i":0, "node_j":index, "area":1.0,
                    "permeability":if index == 1 { 1.0 + delta } else { index as f64 }
                })).collect::<Vec<_>>()
            });
            let decoded = serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
            let direct = run_solve_operator("solve.magnetostatic_bar_1d", decoded).unwrap();
            let run = run_workflow_graph(workflow(input)).unwrap();
            assert!(run.failed_nodes.is_empty());
            assert_eq!(run.artifacts["output.result"], direct);
            let result: Value =
                serde_json::from_slice(&serde_json::to_vec(&direct).unwrap()).unwrap();
            let expected = sign * delta * delta / (arms as f64 + delta);
            let actual = result["nodes"][0]["magnetic_potential"].as_f64().unwrap();
            assert!(
                (actual / expected - 1.0).abs() < 1.0e-12,
                "{actual:e} != {expected:e}"
            );
        }
    }
}

#[test]
fn magnetic_sparse_boundary_recovers_junction_load_through_json_and_workflow() {
    for value in [f64::from_bits(1), -f64::from_bits(3)] {
        let input = json!({
            "nodes": (0..=4).map(|index| json!({
                "id": format!("n{index}"), "x": index,
                "fix_magnetic_potential": index != 0,
                "magnetic_potential": if index == 0 { 0.0 } else { value },
                "magnetomotive_source": 0.0
            })).collect::<Vec<_>>(),
            "elements": (1..=4).map(|index| json!({
                "id": format!("e{index}"), "node_i":0, "node_j":index,
                "area":1.0, "permeability":0.25 * index as f64
            })).collect::<Vec<_>>()
        });
        let decoded = serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
        let direct = run_solve_operator("solve.magnetostatic_bar_1d", decoded).unwrap();
        let run = run_workflow_graph(workflow(input)).unwrap();
        assert!(run.failed_nodes.is_empty());
        assert_eq!(run.artifacts["output.result"], direct);
        let result: Value = serde_json::from_slice(&serde_json::to_vec(&direct).unwrap()).unwrap();
        assert_eq!(
            result["nodes"][0]["magnetic_potential"].as_f64().unwrap(),
            value
        );
        assert_eq!(result["max_flux_density"].as_f64().unwrap(), 0.0);
        assert_eq!(result["total_stored_energy"].as_f64().unwrap(), 0.0);
    }
}

#[test]
fn magnetic_prescribed_range_survives_workflow_dispatch_and_json_roundtrip() {
    for (value, permeability, count) in [
        (f64::from_bits(1), 0.5, 3),
        (-f64::from_bits(3), 0.5, 3),
        (2.0_f64.powi(1022), 8.0, 2),
        (-2.0_f64.powi(1022), 8.0, 2),
    ] {
        let input = json!({
            "nodes": (0..count).map(|index| json!({
                "id": format!("n{index}"), "x": index,
                "fix_magnetic_potential": index == 0 || (count == 3 && index == 2),
                "magnetic_potential": if index == 0 || index == 2 { value } else { 0.0 },
                "magnetomotive_source": 0.0
            })).collect::<Vec<_>>(),
            "elements": (1..count).map(|index| json!({
                "id": format!("e{index}"), "node_i":index - 1, "node_j":index,
                "area": 1.0, "permeability": permeability
            })).collect::<Vec<_>>()
        });
        let decoded = serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
        let direct = run_solve_operator("solve.magnetostatic_bar_1d", decoded).unwrap();
        let run = run_workflow_graph(workflow(input)).unwrap();
        assert!(run.failed_nodes.is_empty());
        assert_eq!(run.artifacts["output.result"], direct);
        let result: Value = serde_json::from_slice(&serde_json::to_vec(&direct).unwrap()).unwrap();
        for node in result["nodes"].as_array().unwrap() {
            assert_eq!(node["magnetic_potential"].as_f64().unwrap(), value);
        }
        assert_eq!(result["max_flux_density"].as_f64().unwrap(), 0.0);
        assert_eq!(result["total_stored_energy"].as_f64().unwrap(), 0.0);
    }
}

fn workflow(input: Value) -> WorkflowGraphRunRequest {
    serde_json::from_value(json!({
        "graph": {
            "schema_version": "kyuubiki.workflow-graph/v1",
            "id": "prescribed-magnetic-range", "name": "Prescribed magnetic range", "version": "1.0.0",
            "entry_nodes": ["input"], "output_nodes": ["output"],
            "nodes": [
                {"id":"input", "kind":"input", "inputs":[],
                 "outputs":[{"id":"model", "artifact_type":"study_model/magnetostatic_bar_1d"}]},
                {"id":"solve", "kind":"solve", "operator_id":"solve.magnetostatic_bar_1d",
                 "inputs":[{"id":"model", "artifact_type":"study_model/magnetostatic_bar_1d"}],
                 "outputs":[{"id":"result", "artifact_type":"result/magnetostatic_bar_1d"}]},
                {"id":"output", "kind":"output", "outputs":[],
                 "inputs":[{"id":"result", "artifact_type":"result/magnetostatic_bar_1d"}]}
            ],
            "edges": [
                {"id":"in", "from":{"node":"input", "port":"model"},
                 "to":{"node":"solve", "port":"model"}, "artifact_type":"study_model/magnetostatic_bar_1d"},
                {"id":"out", "from":{"node":"solve", "port":"result"},
                 "to":{"node":"output", "port":"result"}, "artifact_type":"result/magnetostatic_bar_1d"}
            ]
        },
        "input_artifacts": {"input": input}
    })).unwrap()
}
