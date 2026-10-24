use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};

fn model(sign: f64, reversed: bool, late_failure: bool) -> Value {
    let delta = 2.0_f64.powi(-27);
    let input = json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"middle","x":1.0,"fix_x":false,"mass":1.0,"load_x":sign*2.0*(1.0-delta)},
            {"id":"tip","x":2.0,"fix_x":false,"mass":1.0,"load_x":sign*(-1.0+2.0_f64.powi(-20))}
        ],
        "elements":[
            {"id":"support","node_i":usize::from(reversed),"node_j":usize::from(!reversed),"stiffness":1.0-delta},
            {"id":"coupling","node_i":if reversed {2}else{1},"node_j":if reversed {1}else{2},"stiffness":1.0+delta}
        ],"frequencies_hz":if late_failure {vec![0.0,1e200]}else{vec![0.0]}
    });
    serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap()
}

fn graph(input: Value) -> WorkflowGraphRunRequest {
    let model_type = "study_model/harmonic_spring_1d";
    let result_type = "result/harmonic_spring_1d";
    let port = |id, kind| json!({"id":id,"artifact_type":kind});
    serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1","id":"harmonic-row-update",
            "name":"Harmonic row update","version":"1.0.0",
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
    for result in [result, decoded] {
        let actual = result["frequencies"][0]["nodes"][2]["displacement_amplitude"]
            .as_f64()
            .unwrap();
        assert!((actual / f64::from_bits(0x3ebffffffff80000) - 1.0).abs() <= 4.0 * f64::EPSILON);
        assert!(result["max_force"].as_f64().unwrap().is_finite());
    }
}

#[test]
fn harmonic_row_update_serialized_dispatch_and_workflow_recover_the_weak_tip_response() {
    for workflow in [false, true] {
        for sign in [-1.0, 1.0] {
            for reversed in [false, true] {
                assert_response(execute(workflow, model(sign, reversed, false)).unwrap());
            }
        }
    }
}

#[test]
fn harmonic_row_update_late_failure_prevents_weak_response_workflow_publication() {
    for workflow in [false, true] {
        let bad = model(1.0, false, true);
        let error = execute(workflow, bad.clone()).unwrap_err();
        assert!(
            error.contains("frequency 1") && error.contains("range"),
            "{error}"
        );
        assert_eq!(execute(workflow, bad).unwrap_err(), error);
        assert_response(execute(workflow, model(1.0, false, false)).unwrap());
    }
}

#[test]
fn harmonic_row_update_cancelled_factor_and_substitution_boundaries_prevent_publication() {
    for workflow in [false, true] {
        for (stage, completed) in [
            (SolverStage::TridiagonalFactor, 0),
            (SolverStage::TridiagonalFactor, 1),
            (SolverStage::TridiagonalSubstitution, 2),
            (SolverStage::HarmonicSweep, 1),
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
                || execute(workflow, model(1.0, false, false)),
            )
            .unwrap_err();
            assert!(
                control.was_interrupted() && error.contains("cancelled"),
                "{error}"
            );
            assert_response(execute(workflow, model(1.0, false, false)).unwrap());
        }
    }
}
