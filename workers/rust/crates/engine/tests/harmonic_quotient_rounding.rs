use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};

fn model(sign: f64, reversed: bool, late_failure: bool) -> Value {
    let stiffness = f64::from_bits((2046_u64 << 52) | 1);
    let input = json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "mass":1.0, "load_x":0.0},
            {"id":"tip", "x":1.0, "fix_x":false,
             "mass":if late_failure {stiffness} else {1.0},
             "load_x":sign*f64::from_bits((1023_u64 << 52) | 4)}
        ],
        "elements":[{"id":"spring", "node_i":usize::from(reversed), "node_j":usize::from(!reversed), "stiffness":stiffness}],
        "frequencies_hz":if late_failure {vec![0.0, 1.0/std::f64::consts::TAU]} else {vec![0.0]}
    });
    serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap()
}

fn graph(input: Value) -> WorkflowGraphRunRequest {
    let model_type = "study_model/harmonic_spring_1d";
    let result_type = "result/harmonic_spring_1d";
    let port = |id, kind| json!({"id":id, "artifact_type":kind});
    serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1", "id":"harmonic-quotient-rounding",
            "name":"Harmonic quotient rounding", "version":"1.0.0",
            "entry_nodes":["input"], "output_nodes":["output"],
            "nodes":[
                {"id":"input", "kind":"input", "inputs":[], "outputs":[port("model", model_type)]},
                {"id":"solve", "kind":"solve", "operator_id":"solve.harmonic_spring_1d",
                 "inputs":[port("model", model_type)], "outputs":[port("result", result_type)]},
                {"id":"output", "kind":"output", "inputs":[port("result", result_type)], "outputs":[]}
            ],
            "edges":[
                {"id":"in", "from":{"node":"input", "port":"model"}, "to":{"node":"solve", "port":"model"}, "artifact_type":model_type},
                {"id":"out", "from":{"node":"solve", "port":"result"}, "to":{"node":"output", "port":"result"}, "artifact_type":result_type}
            ]
        }, "input_artifacts":{"input":input}
    })).unwrap()
}

fn execute(workflow: bool, input: Value) -> Result<Value, String> {
    if workflow {
        let run = run_workflow_graph(graph(input))?;
        assert!(run.failed_nodes.is_empty());
        Ok(run.artifacts["output.result"].clone())
    } else {
        run_solve_operator("solve.harmonic_spring_1d", input)
    }
}

fn assert_rounded(result: &Value) {
    let amplitude = result["frequencies"][0]["nodes"][1]["displacement_amplitude"]
        .as_f64()
        .unwrap();
    assert_eq!(amplitude.to_bits(), (1_u64 << 51) + 1);
}

#[test]
fn harmonic_quotient_rounding_serialized_dispatch_and_workflows_preserve_exact_response_bits() {
    for workflow in [false, true] {
        for sign in [-1.0, 1.0] {
            for reversed in [false, true] {
                let result = execute(workflow, model(sign, reversed, false)).unwrap();
                assert_rounded(&result);
                let restored =
                    serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
                assert_rounded(&restored);
            }
        }
    }
}

#[test]
fn harmonic_quotient_rounding_late_failure_cannot_publish_a_workflow_prefix() {
    for workflow in [false, true] {
        let bad = model(1.0, false, true);
        let error = execute(workflow, bad.clone()).unwrap_err();
        assert!(
            error.contains("frequency 1") && error.contains("singular"),
            "{error}"
        );
        assert_eq!(execute(workflow, bad).unwrap_err(), error);
        assert_rounded(&execute(workflow, model(1.0, false, false)).unwrap());
    }
}

#[test]
fn harmonic_quotient_rounding_component_cancellation_blocks_dispatch_and_workflow_publication() {
    for workflow in [false, true] {
        for completed in 0..=2 {
            let control = SolverControl::default();
            let cancel = control.clone();
            let error = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == SolverStage::TridiagonalSubstitutionRange
                        && point.completed_steps == completed
                    {
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
            assert_rounded(&execute(workflow, model(1.0, false, false)).unwrap());
        }
    }
}
