use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};
use std::{cell::Cell, f64::consts::PI, rc::Rc};

fn model(dense: bool) -> Value {
    let mut model = json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "load_x":17.0, "mass":1.0},
            {"id":"one", "x":1.0, "fix_x":false, "load_x":1.0, "mass":1.0},
            {"id":"two", "x":2.0, "fix_x":false, "load_x":2.0, "mass":1.0},
            {"id":"three", "x":3.0, "fix_x":false, "load_x":3.0, "mass":1.0}
        ],
        "elements":[
            {"id":"s0", "node_i":0, "node_j":1, "stiffness":2.0, "damping":0.5},
            {"id":"s1", "node_i":1, "node_j":2, "stiffness":2.0, "damping":0.5},
            {"id":"s2", "node_i":2, "node_j":3, "stiffness":2.0, "damping":0.5}
        ], "frequencies_hz":[0.0, 1.0/(2.0*PI), 0.0]
    });
    if dense {
        model["elements"].as_array_mut().unwrap().push(json!({
            "id":"cycle", "node_i":1, "node_j":3, "stiffness":1.0, "damping":0.25
        }));
    }
    model
}

fn graph(model: Value) -> WorkflowGraphRunRequest {
    serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1", "id":"harmonic-original-equations",
            "name":"Harmonic original equations", "version":"1.0.0",
            "entry_nodes":["input"], "output_nodes":["output"],
            "nodes":[
                {"id":"input", "kind":"input", "inputs":[], "outputs":[{"id":"model", "artifact_type":"study_model/harmonic_spring_1d"}]},
                {"id":"solve", "kind":"solve", "operator_id":"solve.harmonic_spring_1d",
                 "inputs":[{"id":"model", "artifact_type":"study_model/harmonic_spring_1d"}], "outputs":[{"id":"result", "artifact_type":"result/harmonic_spring_1d"}]},
                {"id":"output", "kind":"output", "outputs":[], "inputs":[{"id":"result", "artifact_type":"result/harmonic_spring_1d"}]}
            ],
            "edges":[
                {"id":"in", "from":{"node":"input", "port":"model"}, "to":{"node":"solve", "port":"model"}, "artifact_type":"study_model/harmonic_spring_1d"},
                {"id":"out", "from":{"node":"solve", "port":"result"}, "to":{"node":"output", "port":"result"}, "artifact_type":"result/harmonic_spring_1d"}
            ]
        }, "input_artifacts":{"input":model}
    })).unwrap()
}

fn execute(workflow: bool, model: Value) -> Result<Value, String> {
    if workflow {
        let run = run_workflow_graph(graph(model))?;
        assert!(run.failed_nodes.is_empty());
        Ok(run.artifacts["output.result"].clone())
    } else {
        run_solve_operator("solve.harmonic_spring_1d", model)
    }
}

#[test]
fn harmonic_original_equations_gate_every_frequency_in_path_and_dense_workflows() {
    for dense in [false, true] {
        let input = model(dense);
        let baseline = execute(false, input.clone()).unwrap();
        for workflow in [false, true] {
            let counts = Rc::new(Cell::new((0, 0)));
            let observed = Rc::clone(&counts);
            let serialized = serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
            let result = with_solver_observer(
                &SolverControl::default(),
                move |point| {
                    if point.stage == SolverStage::ConstraintValidate {
                        let (entered, completed) = observed.get();
                        observed.set((
                            entered + usize::from(point.completed_steps == 0),
                            completed + usize::from(point.completed_steps == 8),
                        ));
                    }
                },
                || execute(workflow, serialized),
            )
            .unwrap();
            assert_eq!(counts.get(), (3, 3));
            assert_eq!(result, baseline);
            for frequency in result["frequencies"].as_array().unwrap() {
                assert_eq!(frequency["nodes"][0]["displacement_amplitude"], 0.0);
                assert_eq!(frequency["nodes"][0]["velocity_amplitude"], 0.0);
            }
            assert_eq!(result["frequencies"][0], result["frequencies"][2]);
        }
    }
}

#[test]
fn harmonic_original_equations_reject_erased_force_through_dispatch_and_workflow() {
    let mut input = model(false);
    input["nodes"].as_array_mut().unwrap().truncate(3);
    input["elements"].as_array_mut().unwrap().truncate(2);
    input["nodes"][1]["load_x"] = json!(0.0);
    input["nodes"][2]["load_x"] = json!(1.0);
    input["elements"][0]["stiffness"] = json!(1e300);
    input["elements"][1]["stiffness"] = json!(1e-300);
    input["frequencies_hz"] = json!([0.0]);
    for workflow in [false, true] {
        let error = execute(workflow, input.clone()).unwrap_err();
        assert!(
            error.contains("frequency 0")
                && error.contains("free row 0")
                && error.contains("backward-error"),
            "{error}"
        );
        assert!(execute(workflow, model(false)).is_ok());
    }
}

#[test]
fn harmonic_original_equations_cancel_later_frequency_state_and_residual_then_replay() {
    for dense in [false, true] {
        let input = model(dense);
        let baseline = execute(false, input.clone()).unwrap();
        for workflow in [false, true] {
            for (stage, completed) in [
                (SolverStage::ConstraintValidate, 0),
                (SolverStage::ConstraintValidate, 4),
                (SolverStage::ConstraintValidate, 8),
                (SolverStage::HarmonicResidual, 0),
                (SolverStage::HarmonicResidual, 3),
            ] {
                let control = SolverControl::default();
                let cancel = control.clone();
                let frequency = Rc::new(Cell::new(0));
                let observed = Rc::clone(&frequency);
                let error = with_solver_observer(
                    &control,
                    move |point| {
                        if point.stage == SolverStage::HarmonicSweep {
                            observed.set(point.completed_steps);
                        }
                        if observed.get() == 1
                            && point.stage == stage
                            && point.completed_steps == completed
                        {
                            cancel.request_cancel();
                        }
                    },
                    || execute(workflow, input.clone()),
                )
                .unwrap_err();
                assert_eq!(frequency.get(), 1);
                assert!(error.contains(stage.as_str()), "{error}");
                assert!(control.was_interrupted());
                assert_eq!(execute(workflow, input.clone()).unwrap(), baseline);
            }
        }
    }
}

#[test]
fn harmonic_original_equations_preserve_analytic_complex_response_under_signed_scaling() {
    for power in [-600, 0, 600] {
        for sign in [-1.0, 1.0] {
            let factor = 2.0_f64.powi(power);
            let mut input = model(false);
            input["nodes"].as_array_mut().unwrap().truncate(2);
            input["elements"].as_array_mut().unwrap().truncate(1);
            input["nodes"][1]["mass"] = json!(factor);
            input["nodes"][1]["load_x"] = json!(sign * factor);
            input["elements"][0]["stiffness"] = json!(2.0 * factor);
            input["elements"][0]["damping"] = json!(factor);
            input["frequencies_hz"] = json!([1.0 / (2.0 * PI)]);
            let direct = execute(false, input.clone()).unwrap();
            let tip = &direct["frequencies"][0]["nodes"][1];
            assert!(
                (tip["displacement_amplitude"].as_f64().unwrap() - 0.5_f64.sqrt()).abs() < 1e-12
            );
            let phase = if sign > 0.0 { -45.0 } else { 135.0 };
            assert!((tip["displacement_phase_deg"].as_f64().unwrap() - phase).abs() < 1e-10);
            assert_eq!(execute(true, input).unwrap(), direct);
        }
    }
}

fn frequency_model(power: i32, dense: bool) -> Value {
    let mut input = model(dense);
    let inertia = 2.0_f64.powi(power / 2);
    let omega = 2.0_f64.powi(power);
    for node in input["nodes"].as_array_mut().unwrap().iter_mut().skip(1) {
        node["mass"] = json!(2.0_f64.powi(-3 * power / 2));
        node["load_x"] = json!(2.0_f64.powi(-power / 2));
    }
    let elements = input["elements"].as_array_mut().unwrap();
    for element in elements.iter_mut() {
        element["stiffness"] = json!(inertia);
        element["damping"] = json!(0.0);
    }
    elements[0]["stiffness"] = json!(2.0 * inertia);
    elements[0]["damping"] = json!(inertia / omega);
    for node in [2, 3] {
        elements.push(
            json!({"id":format!("ground{node}"), "node_i":0, "node_j":node,
            "stiffness":2.0*inertia, "damping":inertia/omega}),
        );
    }
    input["frequencies_hz"] = json!([omega / (2.0 * PI)]);
    input
}

#[test]
fn harmonic_frequency_range_recovered_coefficients_reach_dispatch_and_workflows() {
    for power in [-600, 600] {
        for dense in [false, true] {
            let input = frequency_model(power, dense);
            let direct = execute(false, input.clone()).unwrap();
            for node in direct["frequencies"][0]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .skip(1)
            {
                let amplitude = node["displacement_amplitude"].as_f64().unwrap();
                let acceleration = node["acceleration_amplitude"].as_f64().unwrap();
                assert!((amplitude / (0.5_f64.sqrt() * 2.0_f64.powi(-power)) - 1.0).abs() < 1e-12);
                assert!(
                    (acceleration / (0.5_f64.sqrt() * 2.0_f64.powi(power)) - 1.0).abs() < 1e-12
                );
                assert!((node["displacement_phase_deg"].as_f64().unwrap() + 45.0).abs() < 1e-10);
            }
            assert_eq!(execute(true, input).unwrap(), direct);
        }
    }
}

#[test]
fn harmonic_frequency_range_coefficient_failure_keeps_frequency_context_and_replays() {
    for dense in [false, true] {
        let mut input = model(dense);
        for element in input["elements"].as_array_mut().unwrap() {
            element["damping"] = json!(0.0);
        }
        input["frequencies_hz"] = json!([0.0, 2.0_f64.powi(-600) / (2.0 * PI)]);
        for workflow in [false, true] {
            let error = execute(workflow, input.clone()).unwrap_err();
            assert!(
                error.contains("frequency 1")
                    && error.contains("node 1 inertia coefficient")
                    && error.contains("lose numerical range"),
                "{error}"
            );
            assert!(execute(workflow, frequency_model(-600, dense)).is_ok());
        }
    }
}

#[test]
fn harmonic_frequency_range_recovered_sweeps_cancel_before_publication_then_replay() {
    for power in [-600, 600] {
        for dense in [false, true] {
            let mut input = frequency_model(power, dense);
            input["frequencies_hz"] = json!([0.0, 2.0_f64.powi(power) / (2.0 * PI)]);
            let baseline = execute(false, input.clone()).unwrap();
            for workflow in [false, true] {
                let control = SolverControl::default();
                let cancel = control.clone();
                let frequency = Rc::new(Cell::new(0));
                let observed = Rc::clone(&frequency);
                let error = with_solver_observer(
                    &control,
                    move |point| {
                        if point.stage == SolverStage::HarmonicSweep {
                            observed.set(point.completed_steps);
                        }
                        if observed.get() == 1
                            && point.stage == SolverStage::ConstraintValidate
                            && point.completed_steps == 8
                        {
                            cancel.request_cancel();
                        }
                    },
                    || execute(workflow, input.clone()),
                )
                .unwrap_err();
                assert_eq!(frequency.get(), 1);
                assert!(
                    control.was_interrupted() && error.contains("constraint_validate"),
                    "{error}"
                );
                assert_eq!(execute(workflow, input.clone()).unwrap(), baseline);
            }
        }
    }
}

fn quantum_force_input(dense: bool) -> Value {
    let mut input = model(dense);
    for node in input["nodes"].as_array_mut().unwrap().iter_mut().skip(1) {
        node["mass"] = json!(2.0_f64.powi(-700));
        node["load_x"] = json!(f64::from_bits(1));
    }
    let elements = input["elements"].as_array_mut().unwrap();
    for element in elements.iter_mut() {
        element["stiffness"] = json!(2.0_f64.powi(-700));
        element["damping"] = json!(0.0);
    }
    elements[0]["stiffness"] = json!(2.0_f64.powi(-600));
    elements[0]["damping"] = json!(2.0_f64.powi(-600));
    for node in [2, 3] {
        elements.push(
            json!({"id":format!("ground{node}"), "node_i":0, "node_j":node,
            "stiffness":2.0_f64.powi(-600), "damping":2.0_f64.powi(-600)}),
        );
    }
    input["frequencies_hz"] = json!([1.0 / (2.0 * PI)]);
    input
}

#[test]
fn harmonic_complex_force_range_quantum_survives_serialized_dispatch_and_workflow() {
    for dense in [false, true] {
        let input: Value =
            serde_json::from_slice(&serde_json::to_vec(&quantum_force_input(dense)).unwrap())
                .unwrap();
        let direct = execute(false, input.clone()).unwrap();
        assert_eq!(direct["max_force"].as_f64().unwrap(), f64::from_bits(1));
        for element in direct["frequencies"][0]["elements"].as_array().unwrap() {
            if element["node_i"] == 0 {
                assert_eq!(
                    element["force_amplitude"].as_f64().unwrap(),
                    f64::from_bits(1)
                );
            }
        }
        assert_eq!(execute(true, input).unwrap(), direct);
    }
}

#[test]
fn harmonic_complex_force_range_failure_prevents_workflow_publication_and_replays() {
    let mut input = quantum_force_input(false);
    input["nodes"].as_array_mut().unwrap().truncate(2);
    let element = input["elements"][0].clone();
    input["elements"] = json!(
        (0..4)
            .map(|index| {
                let mut element = element.clone();
                element["id"] = json!(format!("parallel{index}"));
                element
            })
            .collect::<Vec<_>>()
    );
    for workflow in [false, true] {
        let error = execute(workflow, input.clone()).unwrap_err();
        assert!(
            error.contains("frequency 0")
                && error.contains("element 0 force")
                && error.contains("numerical range"),
            "{error}"
        );
        assert!(execute(workflow, quantum_force_input(false)).is_ok());
    }
}

#[test]
fn harmonic_complex_force_range_cancels_inside_force_recovery_then_replays() {
    for dense in [false, true] {
        let input = quantum_force_input(dense);
        let baseline = execute(false, input.clone()).unwrap();
        for workflow in [false, true] {
            for completed in [0, 3, 6] {
                let control = SolverControl::default();
                let cancel = control.clone();
                let entered = Rc::new(Cell::new(0));
                let observed = Rc::clone(&entered);
                let error = with_solver_observer(
                    &control,
                    move |point| {
                        if point.stage == SolverStage::ResultElements {
                            if point.completed_steps == 0 {
                                observed.set(observed.get() + 1);
                            }
                            if point.completed_steps == completed && observed.get() >= 2 {
                                cancel.request_cancel();
                            }
                        }
                    },
                    || execute(workflow, input.clone()),
                )
                .unwrap_err();
                assert!(entered.get() >= 2);
                assert!(
                    control.was_interrupted() && error.contains("result_elements"),
                    "{error}"
                );
                assert_eq!(execute(workflow, input.clone()).unwrap(), baseline);
            }
        }
    }
}
