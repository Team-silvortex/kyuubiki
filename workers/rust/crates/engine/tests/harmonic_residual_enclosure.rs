use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};

fn model(dense: bool, reversed: bool) -> Value {
    let mut input = json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"tip","x":1.0,"fix_x":false,"mass":(1.0+f64::EPSILON)*2.0_f64.powi(-1000),"load_x":0.5}
        ],"elements":[
            {"id":"spring","node_i":0,"node_j":1,"stiffness":1.0,"damping":0.5}
        ],"frequencies_hz":[1.0/std::f64::consts::TAU]
    });
    if dense {
        for node in 2..=4 {
            input["nodes"].as_array_mut().unwrap().push(json!({
                "id":format!("control-{node}"),"x":node as f64,"fix_x":false,"mass":1.0,"load_x":0.0
            }));
            input["elements"].as_array_mut().unwrap().push(json!({
                "id":format!("ground-{node}"),"node_i":0,"node_j":node,"stiffness":4.0
            }));
        }
        for (i, j) in [(2, 3), (3, 4), (4, 2)] {
            input["elements"].as_array_mut().unwrap().push(json!({
                "id":format!("cycle-{i}-{j}"),"node_i":i,"node_j":j,"stiffness":1.0
            }));
        }
    }
    if reversed {
        input["elements"].as_array_mut().unwrap().reverse();
        for element in input["elements"].as_array_mut().unwrap() {
            let first = element["node_i"].clone();
            element["node_i"] = element["node_j"].clone();
            element["node_j"] = first;
        }
    }
    serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap()
}

fn graph(input: Value) -> WorkflowGraphRunRequest {
    let model_type = "study_model/harmonic_spring_1d";
    let result_type = "result/harmonic_spring_1d";
    let port = |id, kind| json!({"id":id,"artifact_type":kind});
    serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1","id":"harmonic-residual-enclosure",
            "name":"Harmonic residual enclosure","version":"1.0.0",
            "entry_nodes":["input"],"output_nodes":["output"],
            "nodes":[
                {"id":"input","kind":"input","inputs":[],"outputs":[port("model",model_type)]},
                {"id":"solve","kind":"solve","operator_id":"solve.harmonic_spring_1d",
                 "inputs":[port("model",model_type)],"outputs":[port("result",result_type)]},
                {"id":"output","kind":"output","inputs":[port("result",result_type)],"outputs":[]}
            ],"edges":[
                {"id":"in","from":{"node":"input","port":"model"},"to":{"node":"solve","port":"model"},"artifact_type":model_type},
                {"id":"out","from":{"node":"solve","port":"result"},"to":{"node":"output","port":"result"},"artifact_type":result_type}
            ]
        },"input_artifacts":{"input":input}
    })).unwrap()
}

fn execute(workflow: bool, input: Value) -> Result<Value, String> {
    if workflow {
        let result = run_workflow_graph(graph(input))?;
        assert!(result.failed_nodes.is_empty());
        Ok(result.artifacts["output.result"].clone())
    } else {
        run_solve_operator("solve.harmonic_spring_1d", input)
    }
}

fn assert_response(value: Value) {
    let decoded: Value = serde_json::from_slice(&serde_json::to_vec(&value).unwrap()).unwrap();
    for value in [value, decoded] {
        let response = &value["frequencies"][0]["nodes"][1];
        let amplitude = response["displacement_amplitude"].as_f64().unwrap();
        assert!((amplitude / 0.4_f64.hypot(0.2) - 1.0).abs() <= 4.0 * f64::EPSILON);
        assert!(
            (response["displacement_phase_deg"].as_f64().unwrap()
                - (-0.2_f64).atan2(0.4).to_degrees())
            .abs()
                <= 1e-12
        );
        assert!((value["max_force"].as_f64().unwrap() - 0.5).abs() <= 4.0 * f64::EPSILON);
    }
}

#[test]
fn harmonic_residual_enclosure_serialized_dispatch_and_workflow_publish_the_checked_response() {
    for workflow in [false, true] {
        for dense in [false, true] {
            for reversed in [false, true] {
                assert_response(execute(workflow, model(dense, reversed)).unwrap());
            }
        }
    }
}

#[test]
fn harmonic_residual_enclosure_workflow_cancellation_during_replay_prevents_publication() {
    for workflow in [false, true] {
        for dense in [false, true] {
            for mode in 0..3 {
                let input = model(dense, false);
                let n = input["nodes"].as_array().unwrap().len() as u64;
                let starts = Rc::new(Cell::new(0));
                let events = starts.clone();
                let progress = Rc::new(Cell::new(0));
                let observed = progress.clone();
                let control = SolverControl::default();
                let cancel = control.clone();
                let error = with_solver_observer(
                    &control,
                    move |p| {
                        if p.stage == SolverStage::HarmonicResidual && p.completed_steps == 0 {
                            events.set(events.get() + 1);
                            if mode == 0 && events.get() == 2 {
                                cancel.request_cancel();
                            }
                        }
                        if mode == 1
                            && events.get() >= 2
                            && p.stage == SolverStage::HarmonicResidual
                            && p.completed_steps == n
                        {
                            observed.set(observed.get() + 1);
                            if observed.get() == 2 {
                                cancel.request_cancel();
                            }
                        }
                        if mode == 2
                            && p.stage == SolverStage::ConstraintValidate
                            && p.completed_steps == 2 * n
                        {
                            cancel.request_cancel();
                        }
                    },
                    || execute(workflow, input.clone()),
                )
                .unwrap_err();
                assert!(
                    starts.get() >= 2 && control.was_interrupted() && error.contains("cancelled"),
                    "{error}"
                );
                assert_response(execute(workflow, input).unwrap());
            }
        }
    }
}

#[test]
fn harmonic_residual_enclosure_late_failure_cannot_publish_the_checked_prefix() {
    for workflow in [false, true] {
        for dense in [false, true] {
            let mut input = model(dense, false);
            input["frequencies_hz"]
                .as_array_mut()
                .unwrap()
                .push(json!(1e308));
            let error = execute(workflow, input.clone()).unwrap_err();
            assert!(
                error.contains("frequency 1") && error.contains("range"),
                "{error}"
            );
            assert_eq!(execute(workflow, input).unwrap_err(), error);
            assert_response(execute(workflow, model(dense, false)).unwrap());
        }
    }
}

fn parallel_model(dense: bool, power: i32) -> Value {
    let mut input = model(dense, false);
    let mut spring = input["elements"].as_array_mut().unwrap().remove(0);
    spring["stiffness"] = json!(2.0_f64.powi(-power));
    spring["damping"] = json!(2.0_f64.powi(-power - 1));
    for index in 0..(1 << power) {
        spring["id"] = json!(format!("parallel-{index}"));
        input["elements"]
            .as_array_mut()
            .unwrap()
            .push(spring.clone());
    }
    input
}

fn assert_parallel_response(value: &Value, power: i32) {
    let response = &value["frequencies"][0]["nodes"][1];
    let amplitude = response["displacement_amplitude"].as_f64().unwrap();
    assert!((amplitude / 0.4_f64.hypot(0.2) - 1.0).abs() <= 4.0 * f64::EPSILON);
    assert!(
        (response["displacement_phase_deg"].as_f64().unwrap() - (-0.2_f64).atan2(0.4).to_degrees())
            .abs()
            <= 1e-12
    );
    assert!(
        (value["max_force"].as_f64().unwrap() / 2.0_f64.powi(-power - 1) - 1.0).abs()
            <= 4.0 * f64::EPSILON
    );
    let mut count = 0;
    for element in value["frequencies"][0]["elements"].as_array().unwrap() {
        if element["id"].as_str().unwrap().starts_with("parallel-") {
            count += 1;
            assert!(
                (element["force_amplitude"].as_f64().unwrap() / 2.0_f64.powi(-power - 1) - 1.0)
                    .abs()
                    <= 4.0 * f64::EPSILON
            );
        }
    }
    assert_eq!(count, 1 << power);
}

#[test]
fn harmonic_residual_many_parallel_serialized_dispatch_and_workflow_keep_the_same_response() {
    for dense in [false, true] {
        for workflow in [false, true] {
            let power = if workflow { 15 } else { 16 };
            let result = execute(workflow, parallel_model(dense, power)).unwrap();
            assert_parallel_response(&result, power);
        }
    }
}

#[test]
fn harmonic_residual_many_parallel_workflow_cancellation_prevents_partial_publication() {
    for dense in [false, true] {
        for workflow in [false, true] {
            let power = if workflow { 15 } else { 16 };
            let starts = Rc::new(Cell::new(0));
            let observed = starts.clone();
            let control = SolverControl::default();
            let cancel = control.clone();
            let input = parallel_model(dense, power);
            let error = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == SolverStage::HarmonicResidual {
                        if p.completed_steps == 0 {
                            observed.set(observed.get() + 1);
                        }
                        if observed.get() == 2 && p.completed_steps == 1024 {
                            cancel.request_cancel();
                        }
                    }
                },
                || execute(workflow, input.clone()),
            )
            .unwrap_err();
            assert!(
                starts.get() == 2 && control.was_interrupted() && error.contains("cancelled"),
                "{error}"
            );
            assert_parallel_response(&execute(workflow, input).unwrap(), power);
        }
    }
}

#[test]
fn harmonic_residual_many_parallel_workflow_output_budget_still_fails_closed() {
    for dense in [false, true] {
        let error = execute(true, parallel_model(dense, 16)).unwrap_err();
        assert!(
            error.contains("output exceeds JSON node security budget"),
            "{error}"
        );
        assert!(!error.contains("backward-error"), "{error}");
        assert_parallel_response(&execute(true, parallel_model(dense, 15)).unwrap(), 15);
    }
}
