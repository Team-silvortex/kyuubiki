use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};

fn model(dense: bool, reversed: bool) -> Value {
    let d = 2.0_f64.powi(-27);
    let mut input = json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"tip","x":1.0,"fix_x":false,"mass":1.0+d,"load_x":5.0*2.0_f64.powi(-54)-2.0_f64.powi(-81)}
        ],"elements":[
            {"id":"spring","node_i":0,"node_j":1,"stiffness":1.0+3.0*d+2.0_f64.powi(-51)}
        ],"frequencies_hz":[(1.0+d)/std::f64::consts::TAU]
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
            "schema_version":"kyuubiki.workflow-graph/v1","id":"harmonic-inertia-precision",
            "name":"Harmonic inertia precision","version":"1.0.0",
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
        assert_eq!(
            value["frequencies"][0]["nodes"][1]["displacement_amplitude"].as_f64(),
            Some(1.0)
        );
        assert_eq!(
            value["frequencies"][0]["nodes"][1]["displacement_phase_deg"].as_f64(),
            Some(0.0)
        );
        let stiffness = 1.0 + 3.0 * 2.0_f64.powi(-27) + 2.0_f64.powi(-51);
        assert_eq!(value["max_force"].as_f64(), Some(stiffness));
    }
}

#[test]
fn harmonic_inertia_precision_serialized_dispatch_and_workflow_recover_unit_response() {
    for workflow in [false, true] {
        for dense in [false, true] {
            for reversed in [false, true] {
                assert_response(execute(workflow, model(dense, reversed)).unwrap());
            }
        }
    }
}

#[test]
fn harmonic_inertia_precision_failed_tail_and_late_frequency_cannot_publish_a_prefix() {
    for workflow in [false, true] {
        for dense in [false, true] {
            for late in [false, true] {
                let mut input = model(dense, false);
                if late {
                    input["frequencies_hz"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!(1e200));
                } else {
                    input["nodes"][1]["mass"] = json!(f64::from_bits(1));
                }
                let error = execute(workflow, input.clone()).unwrap_err();
                assert!(error.contains("range"), "{error}");
                assert_eq!(execute(workflow, input).unwrap_err(), error);
                assert_response(execute(workflow, model(dense, false)).unwrap());
            }
        }
    }
}

#[test]
fn harmonic_inertia_precision_cancelled_assembly_and_residual_prevent_workflow_publication() {
    for workflow in [false, true] {
        for dense in [false, true] {
            let input = model(dense, false);
            let n = input["elements"].as_array().unwrap().len() as u64;
            for (stage, completed) in [
                (SolverStage::ElementAssembly, n + 1),
                (SolverStage::ElementAssembly, n + 2),
                (SolverStage::ElementAssembly, n + 3),
                (SolverStage::HarmonicResidual, 0),
            ] {
                let control = SolverControl::default();
                let cancel = control.clone();
                let error = with_solver_observer(
                    &control,
                    move |p| {
                        if p.stage == stage && p.completed_steps == completed {
                            cancel.request_cancel();
                        }
                    },
                    || execute(workflow, input.clone()),
                )
                .unwrap_err();
                assert!(
                    control.was_interrupted() && error.contains("cancelled"),
                    "{error}"
                );
                assert_response(execute(workflow, input.clone()).unwrap());
            }
        }
    }
}
