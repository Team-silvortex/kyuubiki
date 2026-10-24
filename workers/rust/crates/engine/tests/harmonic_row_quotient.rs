use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};

fn model(sign: f64, reversed: bool, late: bool) -> Value {
    let input = json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"middle","x":1.0,"fix_x":false,"mass":1.0,"load_x":sign*(-1.0+2.0_f64.powi(-40))},
            {"id":"tip","x":2.0,"fix_x":false,"mass":1.0,"load_x":sign}
        ],"elements":[
            {"id":"left","node_i":usize::from(reversed),"node_j":usize::from(!reversed),"stiffness":1.0},
            {"id":"right","node_i":if reversed {2}else{1},"node_j":if reversed {1}else{2},"stiffness":1.0}
        ],"frequencies_hz":if late {vec![0.0,1e200]}else{vec![0.0]}
    });
    serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap()
}

fn execute(workflow: bool, input: Value) -> Result<Value, String> {
    if !workflow {
        return run_solve_operator("solve.harmonic_spring_1d", input);
    }
    let model_type = "study_model/harmonic_spring_1d";
    let result_type = "result/harmonic_spring_1d";
    let port = |id, kind| json!({"id":id,"artifact_type":kind});
    let request: WorkflowGraphRunRequest = serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1","id":"harmonic-row-quotient",
            "name":"Harmonic replayed row quotient","version":"1.0.0",
            "entry_nodes":["input"],"output_nodes":["output"],
            "nodes":[
                {"id":"input","kind":"input","inputs":[],"outputs":[port("model",model_type)]},
                {"id":"solve","kind":"solve","operator_id":"solve.harmonic_spring_1d","inputs":[port("model",model_type)],"outputs":[port("result",result_type)]},
                {"id":"output","kind":"output","inputs":[port("result",result_type)],"outputs":[]}
            ],"edges":[
                {"id":"in","from":{"node":"input","port":"model"},"to":{"node":"solve","port":"model"},"artifact_type":model_type},
                {"id":"out","from":{"node":"solve","port":"result"},"to":{"node":"output","port":"result"},"artifact_type":result_type}
            ]
        },"input_artifacts":{"input":input}
    })).unwrap();
    let result = run_workflow_graph(request)?;
    assert!(result.failed_nodes.is_empty());
    Ok(result.artifacts["output.result"].clone())
}

fn assert_response(result: Value) {
    let decoded: Value = serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    for result in [result, decoded] {
        assert_eq!(
            result["frequencies"][0]["nodes"][1]["displacement_amplitude"]
                .as_f64()
                .unwrap(),
            2.0_f64.powi(-40)
        );
        assert_eq!(
            result["frequencies"][0]["nodes"][2]["displacement_amplitude"]
                .as_f64()
                .unwrap(),
            1.0 + 2.0_f64.powi(-40)
        );
        assert_eq!(result["max_force"].as_f64().unwrap(), 1.0);
    }
}

#[test]
fn harmonic_row_quotient_serialized_dispatch_and_workflow_reach_the_integer_replay() {
    use std::{cell::Cell, rc::Rc};
    for workflow in [false, true] {
        for sign in [-1.0, 1.0] {
            for reversed in [false, true] {
                let events = Rc::new(Cell::new(0));
                let observed = Rc::clone(&events);
                let result = with_solver_observer(
                    &SolverControl::default(),
                    move |p| {
                        if p.stage == SolverStage::TridiagonalSubstitutionRange
                            && p.completed_steps == 0
                        {
                            observed.set(observed.get() + 1);
                        }
                    },
                    || execute(workflow, model(sign, reversed, false)),
                )
                .unwrap();
                assert!(
                    events.get() >= 7,
                    "new integer replay phases were not reached"
                );
                assert_response(result);
            }
        }
    }
}

#[test]
fn harmonic_row_quotient_late_failure_prevents_workflow_publication_then_reuses() {
    for workflow in [false, true] {
        let error = execute(workflow, model(1.0, false, true)).unwrap_err();
        assert!(
            error.contains("frequency 1") && error.contains("range"),
            "{error}"
        );
        assert_eq!(
            execute(workflow, model(1.0, false, true)).unwrap_err(),
            error
        );
        assert_response(execute(workflow, model(1.0, false, false)).unwrap());
    }
}

#[test]
fn harmonic_row_quotient_cancelled_integer_phases_cannot_publish_a_workflow_prefix() {
    use std::cell::Cell;
    for workflow in [false, true] {
        for (completed, occurrence) in [(0, 4), (0, 5), (0, 6), (0, 7), (65, 1)] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let seen = Cell::new(0);
            let error = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == SolverStage::TridiagonalSubstitutionRange
                        && p.completed_steps == completed
                    {
                        seen.set(seen.get() + 1);
                        if seen.get() == occurrence {
                            cancel.request_cancel();
                        }
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
