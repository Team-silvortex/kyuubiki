use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};
use std::cell::Cell;
use std::rc::Rc;

const OPERATOR: &str = "solve.transient_spring_1d";

fn input() -> Value {
    json!({
        "nodes": [
            {"id":"root", "x":0.0, "fix_x":true, "mass":2.0,
             "load_x":17.0, "initial_displacement":0.0, "initial_velocity":0.0},
            {"id":"a", "x":1.0, "fix_x":false, "mass":3.0,
             "load_x":0.4, "initial_displacement":0.1, "initial_velocity":0.05},
            {"id":"b", "x":2.0, "fix_x":false, "mass":5.0,
             "load_x":-0.1, "initial_displacement":0.04, "initial_velocity":-0.04},
            {"id":"c", "x":3.0, "fix_x":false, "mass":7.0,
             "load_x":-0.3, "initial_displacement":-0.02, "initial_velocity":0.1}
        ],
        "elements": [
            {"id":"ka", "node_i":0, "node_j":1, "stiffness":100.0, "damping":0.5},
            {"id":"kb", "node_i":1, "node_j":2, "stiffness":75.0, "damping":2.0},
            {"id":"kc", "node_i":2, "node_j":3, "stiffness":35.0, "damping":1.0}
        ],
        "time_step":0.02, "steps":7, "history_stride":4
    })
}

fn graph(input: Value) -> WorkflowGraphRunRequest {
    let model_type = "study_model/transient_spring_1d";
    let result_type = "result/transient_spring_1d";
    serde_json::from_value(json!({
        "graph": {
            "schema_version":"kyuubiki.workflow-graph/v1", "id":"newmark-original-equations",
            "name":"Newmark original equation admission", "version":"1.0.0",
            "entry_nodes":["input"], "output_nodes":["output"],
            "nodes":[
                {"id":"input", "kind":"input", "inputs":[], "outputs":[{"id":"model", "artifact_type":model_type}]},
                {"id":"solve", "kind":"solve", "operator_id":OPERATOR,
                 "inputs":[{"id":"model", "artifact_type":model_type}], "outputs":[{"id":"result", "artifact_type":result_type}]},
                {"id":"output", "kind":"output", "outputs":[], "inputs":[{"id":"result", "artifact_type":result_type}]}
            ],
            "edges":[
                {"id":"in", "from":{"node":"input", "port":"model"}, "to":{"node":"solve", "port":"model"}, "artifact_type":model_type},
                {"id":"out", "from":{"node":"solve", "port":"result"}, "to":{"node":"output", "port":"result"}, "artifact_type":result_type}
            ]
        }, "input_artifacts":{"input":input}
    })).unwrap()
}

fn execute(input: Value, workflow: bool) -> Result<Value, String> {
    if workflow {
        let run = run_workflow_graph(graph(input))?;
        assert!(run.failed_nodes.is_empty());
        Ok(run.artifacts["output.result"].clone())
    } else {
        run_solve_operator(OPERATOR, input)
    }
}

#[test]
fn newmark_original_equations_gate_every_step_in_serialized_dispatch_and_workflows() {
    let model: Value = serde_json::from_slice(&serde_json::to_vec(&input()).unwrap()).unwrap();
    let baseline = execute(model.clone(), false).unwrap();
    assert_eq!(baseline["history"].as_array().unwrap().len(), 3);
    assert_eq!(baseline["history"][1]["step"], 4);
    for workflow in [false, true] {
        let entered = Rc::new(Cell::new(0));
        let completed = Rc::new(Cell::new(0));
        let observer_entered = Rc::clone(&entered);
        let observer_completed = Rc::clone(&completed);
        let result = with_solver_observer(
            &SolverControl::default(),
            move |point| {
                if point.stage == SolverStage::ConstraintValidate {
                    if point.completed_steps == 0 {
                        observer_entered.set(observer_entered.get() + 1);
                    }
                    if point.completed_steps == 8 {
                        observer_completed.set(observer_completed.get() + 1);
                    }
                }
            },
            || execute(model.clone(), workflow),
        )
        .unwrap();
        assert_eq!(entered.get(), 7);
        assert_eq!(completed.get(), 7);
        assert_eq!(result, baseline);
    }
}

#[test]
fn newmark_original_equations_cancel_unsaved_steps_and_replay_workflows() {
    let model = input();
    let baseline = execute(model.clone(), false).unwrap();
    for (stage, completed) in [
        (SolverStage::ConstraintValidate, 0),
        (SolverStage::ConstraintValidate, 4),
        (SolverStage::ConstraintValidate, 8),
    ] {
        for workflow in [false, true] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let step = Rc::new(Cell::new(0));
            let observed_step = Rc::clone(&step);
            let result = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == SolverStage::TransientStep {
                        observed_step.set(point.completed_steps);
                    }
                    if observed_step.get() == 3
                        && point.stage == stage
                        && point.completed_steps == completed
                    {
                        cancel.request_cancel();
                    }
                },
                || execute(model.clone(), workflow),
            );
            let error = result.expect_err(&format!("{stage:?}/{completed}, workflow={workflow}"));
            assert!(error.contains("constraint_validate"), "{error}");
            assert_eq!(step.get(), 3);
            assert!(control.was_interrupted());
            assert_eq!(execute(model.clone(), workflow).unwrap(), baseline);
        }
    }
}

#[test]
fn newmark_original_equations_cancel_wide_original_rows_then_replay() {
    let mut model = input();
    model["nodes"] = json!(
        (0..1030)
            .map(|index| {
                json!({"id":format!("n{index}"), "x":index, "fix_x":index != 1,
               "mass":1.0, "load_x":if index == 1 { 1.0 } else { 0.0 },
               "initial_displacement":0.0, "initial_velocity":0.0})
            })
            .collect::<Vec<_>>()
    );
    model["elements"] = json!(
        (0..1030)
            .filter(|&index| index != 1)
            .map(|index| {
                json!({"id":format!("e{index}"), "node_i":1, "node_j":index,
               "stiffness":1.0, "damping":0.0})
            })
            .collect::<Vec<_>>()
    );
    model["time_step"] = json!(1.0);
    model["steps"] = json!(3);
    model["history_stride"] = json!(3);
    let baseline = execute(model.clone(), false).unwrap();
    for workflow in [false, true] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let step = Cell::new(0);
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::TransientStep {
                    step.set(point.completed_steps);
                }
                if step.get() == 2
                    && point.stage == SolverStage::ConstraintValidateRow
                    && point.completed_steps == 1024
                {
                    cancel.request_cancel();
                }
            },
            || execute(model.clone(), workflow),
        )
        .unwrap_err();
        assert!(error.contains("constraint_validate_row"), "{error}");
        assert!(control.was_interrupted());
        assert_eq!(execute(model.clone(), workflow).unwrap(), baseline);
    }
}

#[test]
fn newmark_original_equations_all_fixed_loaded_models_remain_admissible() {
    let mut model = input();
    for node in model["nodes"].as_array_mut().unwrap() {
        node["fix_x"] = json!(true);
    }
    let direct = execute(model.clone(), false).unwrap();
    for node in direct["nodes"].as_array().unwrap() {
        for field in ["ux", "vx", "ax"] {
            assert_eq!(node[field], 0.0);
        }
    }
    for frame in direct["history"].as_array().unwrap() {
        for field in ["displacements", "velocities"] {
            assert!(
                frame[field]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|value| value == 0.0)
            );
        }
    }
    assert_eq!(execute(model, true).unwrap(), direct);
}

#[test]
fn newmark_original_equations_sampling_retains_full_state_and_discrete_balance() {
    let mut model = input();
    model["history_stride"] = Value::Null;
    let full = execute(model.clone(), false).unwrap();
    let dt = model["time_step"].as_f64().unwrap();
    for pair in full["history"].as_array().unwrap().windows(2) {
        let old = &pair[0];
        let new = &pair[1];
        let midpoint = (0..4)
            .map(|index| {
                0.5 * (old["velocities"][index].as_f64().unwrap()
                    + new["velocities"][index].as_f64().unwrap())
            })
            .collect::<Vec<_>>();
        let mut work = 0.0;
        for (index, node) in model["nodes"].as_array().unwrap().iter().enumerate() {
            let increment = new["displacements"][index].as_f64().unwrap()
                - old["displacements"][index].as_f64().unwrap();
            assert!((increment - dt * midpoint[index]).abs() < 1.0e-15);
            work += node["load_x"].as_f64().unwrap() * increment;
        }
        let dissipation = model["elements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|element| {
                let i = element["node_i"].as_u64().unwrap() as usize;
                let j = element["node_j"].as_u64().unwrap() as usize;
                dt * element["damping"].as_f64().unwrap() * (midpoint[j] - midpoint[i]).powi(2)
            })
            .sum::<f64>();
        let energy_change = ["kinetic_energy", "strain_energy"]
            .into_iter()
            .map(|field| new[field].as_f64().unwrap() - old[field].as_f64().unwrap())
            .sum::<f64>();
        assert!((energy_change + dissipation - work).abs() < 1.0e-12);
    }
    let sampled = execute(input(), true).unwrap();
    for field in [
        "nodes",
        "elements",
        "max_displacement",
        "max_velocity",
        "max_force",
    ] {
        assert_eq!(sampled[field], full[field]);
    }
    for frame in sampled["history"].as_array().unwrap() {
        let step = frame["step"].as_u64().unwrap() as usize;
        assert_eq!(*frame, full["history"][step]);
    }
}
