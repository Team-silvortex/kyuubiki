use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};

fn model(dense: bool, reversed: bool, late_failure: bool) -> Value {
    let mut input = json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"tip","x":1.0,"fix_x":false,"mass":2.0_f64.powi(55),"load_x":1.0}
        ],"elements":[
            {"id":"soft","node_i":0,"node_j":1,"stiffness":1.0,"damping":2.0},
            {"id":"hard","node_i":0,"node_j":1,"stiffness":2.0_f64.powi(55)}
        ],"frequencies_hz":if late_failure {vec![1.0 / std::f64::consts::TAU, 1e200]}
            else {vec![1.0 / std::f64::consts::TAU]}
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
        let elements = input["elements"].as_array_mut().unwrap();
        elements.reverse();
        for element in elements {
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
            "schema_version":"kyuubiki.workflow-graph/v1","id":"harmonic-assembly",
            "name":"Harmonic coefficient assembly","version":"1.0.0",
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

fn assert_response(result: Value) {
    let decoded: Value = serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    for value in [result, decoded] {
        let response = &value["frequencies"][0]["nodes"][1];
        let expected = 0.2_f64.hypot(0.4);
        let actual = response["displacement_amplitude"].as_f64().unwrap();
        assert!((actual / expected - 1.0).abs() <= 4.0 * f64::EPSILON);
        let phase = response["displacement_phase_deg"].as_f64().unwrap();
        assert!((phase - (-0.4_f64).atan2(0.2).to_degrees()).abs() <= 1e-12);
        assert!(value["max_force"].as_f64().unwrap().is_finite());
    }
}

#[test]
fn harmonic_assembly_serialized_dispatch_and_workflow_match_the_independent_oscillator() {
    for workflow in [false, true] {
        for dense in [false, true] {
            for reversed in [false, true] {
                assert_response(execute(workflow, model(dense, reversed, false)).unwrap());
            }
        }
    }
}

#[test]
fn harmonic_assembly_late_frequency_failure_cannot_publish_the_corrected_prefix() {
    for workflow in [false, true] {
        for dense in [false, true] {
            let bad = model(dense, false, true);
            let error = execute(workflow, bad.clone()).unwrap_err();
            assert!(
                error.contains("frequency 1") && error.contains("range"),
                "{error}"
            );
            assert_eq!(execute(workflow, bad).unwrap_err(), error);
            assert_response(execute(workflow, model(dense, false, false)).unwrap());
        }
    }
}

#[test]
fn harmonic_assembly_cancelled_grouping_and_final_rounding_prevent_workflow_publication() {
    for workflow in [false, true] {
        for dense in [false, true] {
            for offset in [1, 2, 3] {
                let input = model(dense, false, false);
                let boundary = input["elements"].as_array().unwrap().len() as u64 + offset;
                let control = SolverControl::default();
                let cancel = control.clone();
                let error = with_solver_observer(
                    &control,
                    move |p| {
                        if p.stage == SolverStage::ElementAssembly && p.completed_steps == boundary
                        {
                            cancel.request_cancel();
                        }
                    },
                    || execute(workflow, input),
                )
                .unwrap_err();
                assert!(
                    control.was_interrupted() && error.contains("cancelled"),
                    "{error}"
                );
                assert_response(execute(workflow, model(dense, false, false)).unwrap());
            }
        }
    }
}
