use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};

fn family(contact: bool) -> &'static str {
    if contact {
        "contact_gap_1d"
    } else {
        "nonlinear_spring_1d"
    }
}

fn input(contact: bool) -> Value {
    let mut model = json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "load_x":17.0},
            {"id":"tip", "x":1.0, "fix_x":false, "load_x":2.0}
        ],
        "elements":[{"id":"spring", "node_i":0, "node_j":1, "stiffness":1.0,
                     "cubic_stiffness":if contact { 0.0 } else { 1.0 }}],
        "load_steps":4, "max_iterations":32, "tolerance":1e-12
    });
    if contact {
        model["contacts"] = json!([{"id":"stop", "node":1, "gap":1.0, "normal_stiffness":10.0}]);
    }
    model
}

fn graph(contact: bool, model: Value) -> WorkflowGraphRunRequest {
    let family = family(contact);
    let model_type = format!("study_model/{family}");
    let result_type = format!("result/{family}");
    serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1", "id":"newton-original-equations",
            "name":"Newton original equation admission", "version":"1.0.0",
            "entry_nodes":["input"], "output_nodes":["output"],
            "nodes":[
                {"id":"input", "kind":"input", "inputs":[], "outputs":[{"id":"model", "artifact_type":model_type}]},
                {"id":"solve", "kind":"solve", "operator_id":format!("solve.{family}"),
                 "inputs":[{"id":"model", "artifact_type":model_type}], "outputs":[{"id":"result", "artifact_type":result_type}]},
                {"id":"output", "kind":"output", "outputs":[], "inputs":[{"id":"result", "artifact_type":result_type}]}
            ],
            "edges":[
                {"id":"in", "from":{"node":"input", "port":"model"}, "to":{"node":"solve", "port":"model"}, "artifact_type":model_type},
                {"id":"out", "from":{"node":"solve", "port":"result"}, "to":{"node":"output", "port":"result"}, "artifact_type":result_type}
            ]
        }, "input_artifacts":{"input":model}
    })).unwrap()
}

fn execute(contact: bool, workflow: bool, model: Value) -> Result<Value, String> {
    if workflow {
        let run = run_workflow_graph(graph(contact, model))?;
        assert!(run.failed_nodes.is_empty());
        Ok(run.artifacts["output.result"].clone())
    } else {
        run_solve_operator(&format!("solve.{}", family(contact)), model)
    }
}

fn admitted(contact: bool, workflow: bool, model: Value) -> Value {
    let size = model["nodes"].as_array().unwrap().len() as u64;
    let counts = Rc::new(Cell::new((0, 0)));
    let observed = Rc::clone(&counts);
    let result = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::ConstraintValidate {
                let (entered, completed) = observed.get();
                observed.set((
                    entered + usize::from(point.completed_steps == 0),
                    completed + usize::from(point.completed_steps == 2 * size),
                ));
            }
        },
        || execute(contact, workflow, model),
    )
    .unwrap();
    let iterations = result["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| step["iterations"].as_u64().unwrap() as usize)
        .sum::<usize>();
    assert_eq!(counts.get(), (iterations, iterations));
    result
}

fn check(contact: bool) {
    let model = serde_json::from_slice(&serde_json::to_vec(&input(contact)).unwrap()).unwrap();
    let result = admitted(contact, false, model);
    assert_eq!(result["converged"], true);
    assert_eq!(result["achieved_load_factor"], 1.0);
    let u = result["nodes"][1]["ux"].as_f64().unwrap();
    assert!((u - if contact { 12.0 / 11.0 } else { 1.0 }).abs() < 1e-10);
    assert_eq!(admitted(contact, true, input(contact)), result);
}

#[test]
fn newton_original_equations_gate_spring_dispatch_and_workflow_corrections() {
    check(false);
}

#[test]
fn newton_original_equations_gate_contact_dispatch_and_workflow_corrections() {
    check(true);
}

#[test]
fn newton_original_equations_cancel_after_committed_steps_then_replay() {
    for contact in [false, true] {
        let model = input(contact);
        let baseline = execute(contact, false, model.clone()).unwrap();
        for workflow in [false, true] {
            for completed in [0, 2, 4] {
                let control = SolverControl::default();
                let cancel = control.clone();
                let step = Rc::new(Cell::new(0));
                let observed = Rc::clone(&step);
                let error = with_solver_observer(
                    &control,
                    move |point| {
                        if point.stage == SolverStage::StabilityStep {
                            observed.set(point.completed_steps);
                        }
                        if observed.get() == 3
                            && point.stage == SolverStage::ConstraintValidate
                            && point.completed_steps == completed
                        {
                            cancel.request_cancel();
                        }
                    },
                    || execute(contact, workflow, model.clone()),
                )
                .unwrap_err();
                assert!(error.contains("constraint_validate"), "{error}");
                assert_eq!(step.get(), 3);
                assert!(control.was_interrupted());
                assert_eq!(execute(contact, workflow, model.clone()).unwrap(), baseline);
            }
        }
    }
}

#[test]
fn newton_original_equations_zero_iteration_fixed_models_keep_state_checks() {
    for contact in [false, true] {
        let mut model = input(contact);
        model["nodes"][1]["fix_x"] = json!(true);
        let state_checks = Rc::new(Cell::new(0));
        let observed = Rc::clone(&state_checks);
        let direct = with_solver_observer(
            &SolverControl::default(),
            move |point| {
                if point.stage == SolverStage::StabilityRecovery && point.completed_steps == 0 {
                    observed.set(observed.get() + 1);
                }
            },
            || Ok::<_, String>(admitted(contact, false, model.clone())),
        )
        .unwrap();
        assert_eq!(state_checks.get(), 4);
        assert_eq!(direct["converged"], true);
        assert!(
            direct["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|node| node["ux"] == 0.0)
        );
        assert!(
            direct["steps"]
                .as_array()
                .unwrap()
                .iter()
                .all(|step| step["iterations"] == 0)
        );
        assert_eq!(admitted(contact, true, model), direct);
    }
}

#[test]
fn newton_original_equations_preserve_partial_load_rollback_and_clean_replay() {
    for contact in [false, true] {
        let mut model = input(contact);
        model["max_iterations"] = json!(1);
        let partial = admitted(contact, false, model.clone());
        assert_eq!(partial["converged"], false);
        assert_eq!(
            partial["achieved_load_factor"],
            if contact { 0.5 } else { 0.0 }
        );
        assert_eq!(partial["nodes"][1]["ux"], if contact { 1.0 } else { 0.0 });
        assert_eq!(admitted(contact, true, model), partial);
        let full = admitted(contact, true, input(contact));
        assert_eq!(full["converged"], true);
        assert_eq!(full, admitted(contact, false, input(contact)));
    }
}

#[test]
fn newton_original_equations_cover_branched_hardening_and_active_contacts() {
    let expected = [0.0, 0.25, -0.125, 0.5, -0.25];
    for contact in [false, true] {
        for exponent in [-200, 0, 200] {
            let scale = 2.0_f64.powi(exponent);
            let mut model = input(contact);
            model["nodes"] = json!(expected.iter().enumerate().map(|(index, _)| {
                json!({"id":format!("n{index}"), "x":index, "fix_x":index == 0, "load_x":0.0})
            }).collect::<Vec<_>>());
            let mut loads = [0.0; 5];
            model["elements"] = json!([(0, 1), (1, 2), (1, 3), (1, 4)].into_iter().enumerate().map(|(index, (i, j))| {
                let k = (index + 1) as f64 * scale;
                let extension: f64 = expected[j] - expected[i];
                let force = k * extension + k * extension.powi(3);
                loads[i] -= force;
                loads[j] += force;
                json!({"id":format!("e{index}"), "node_i":i, "node_j":j, "stiffness":k, "cubic_stiffness":k})
            }).collect::<Vec<_>>());
            if contact {
                model["contacts"] =
                    json!([{"id":"stop", "node":3, "gap":0.25, "normal_stiffness":2.0 * scale}]);
                loads[3] += 0.5 * scale;
            }
            for (node, load) in model["nodes"].as_array_mut().unwrap().iter_mut().zip(loads) {
                node["load_x"] = json!(load);
            }
            model["tolerance"] = json!(1e-12 * scale);
            let factorizations = Rc::new(Cell::new(0));
            let observed = Rc::clone(&factorizations);
            let result = with_solver_observer(
                &SolverControl::default(),
                move |point| {
                    if point.stage == SolverStage::DenseFactor && point.completed_steps == 0 {
                        observed.set(observed.get() + 1);
                    }
                },
                || execute(contact, false, model.clone()),
            )
            .unwrap();
            assert!(factorizations.get() > 0);
            assert_eq!(result["converged"], true);
            for (node, target) in result["nodes"].as_array().unwrap().iter().zip(expected) {
                assert!((node["ux"].as_f64().unwrap() - target).abs() < 1e-10);
            }
            assert_eq!(admitted(contact, true, model), result);
        }
    }
}

#[test]
fn newton_original_equations_cancel_wide_rows_before_line_search_then_replay() {
    let mut model = input(false);
    model["nodes"] = json!(
        (0..1030)
            .map(|index| {
                json!({"id":format!("n{index}"), "x":index, "fix_x":index != 1,
               "load_x":if index == 1 { 1029.0 } else { 0.0 }})
            })
            .collect::<Vec<_>>()
    );
    model["elements"] = json!((0..1030).filter(|&index| index != 1).map(|index| {
        json!({"id":format!("e{index}"), "node_i":index, "node_j":1, "stiffness":1.0, "cubic_stiffness":0.0})
    }).collect::<Vec<_>>());
    model["load_steps"] = json!(3);
    model["tolerance"] = json!(1e-9);
    let baseline = admitted(false, false, model.clone());
    assert_eq!(baseline["converged"], true);
    for workflow in [false, true] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let step = Cell::new(0);
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::StabilityStep {
                    step.set(point.completed_steps);
                }
                if step.get() == 2
                    && point.stage == SolverStage::ConstraintValidateRow
                    && point.completed_steps == 1024
                {
                    cancel.request_cancel();
                }
            },
            || execute(false, workflow, model.clone()),
        )
        .unwrap_err();
        assert!(error.contains("constraint_validate_row"), "{error}");
        assert!(control.was_interrupted());
        assert_eq!(admitted(false, workflow, model.clone()), baseline);
    }
}
