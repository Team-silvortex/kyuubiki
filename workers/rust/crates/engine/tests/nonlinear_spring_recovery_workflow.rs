use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use serde_json::{Value, json};

fn model(contact: bool, iterations: usize) -> Value {
    let mut value = json!({
        "nodes":[{"id":"base", "x":0.0, "fix_x":true, "load_x":0.0},
                 {"id":"tip", "x":1.0, "fix_x":false, "load_x":2.0}],
        "elements":[{"id":"spring", "node_i":0, "node_j":1, "stiffness":1.0,
                     "cubic_stiffness":if contact { 0.0 } else { 1.0 }}],
        "load_steps":if contact { 4 } else { 1 }, "max_iterations":iterations, "tolerance":1e-12
    });
    if contact {
        value["contacts"] = json!([{"id":"stop", "node":1, "gap":1.0, "normal_stiffness":10.0}]);
    }
    value
}

fn run(contact: bool, input: Value) -> Value {
    let family = if contact {
        "contact_gap_1d"
    } else {
        "nonlinear_spring_1d"
    };
    let input_type = format!("study_model/{family}");
    let output_type = format!("result/{family}");
    let request: WorkflowGraphRunRequest = serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1", "id":"spring-recovery",
            "name":"Spring recovery", "version":"1.0.0", "entry_nodes":["input"], "output_nodes":["output"],
            "nodes":[
                {"id":"input", "kind":"input", "inputs":[], "outputs":[{"id":"model", "artifact_type":input_type}]},
                {"id":"solve", "kind":"solve", "operator_id":format!("solve.{family}"),
                 "inputs":[{"id":"model", "artifact_type":input_type}], "outputs":[{"id":"result", "artifact_type":output_type}]},
                {"id":"output", "kind":"output", "inputs":[{"id":"result", "artifact_type":output_type}], "outputs":[]}
            ],
            "edges":[
                {"id":"in", "from":{"node":"input", "port":"model"}, "to":{"node":"solve", "port":"model"}, "artifact_type":input_type},
                {"id":"out", "from":{"node":"solve", "port":"result"}, "to":{"node":"output", "port":"result"}, "artifact_type":output_type}
            ]
        },
        "input_artifacts":{"input":input}
    })).unwrap();
    run_workflow_graph(request)
        .unwrap()
        .artifacts
        .remove("output.result")
        .unwrap()
}

#[test]
fn workflow_preserves_partial_load_and_failed_trial_diagnostics() {
    for contact in [false, true] {
        let result = run(contact, model(contact, 1));
        assert_eq!(result["converged"], false);
        assert_eq!(
            result["achieved_load_factor"],
            if contact { 0.5 } else { 0.0 }
        );
        assert_eq!(result["nodes"][1]["ux"], if contact { 1.0 } else { 0.0 });
        assert_eq!(result["residual_norm"], 0.0);
        assert_eq!(
            result["steps"].as_array().unwrap().last().unwrap()["residual_norm"],
            if contact { 5.0 } else { 8.0 }
        );
        if contact {
            assert_eq!(result["contacts"][0]["active"], false);
        }
    }
}

#[test]
fn workflow_replay_reaches_full_load_and_matches_direct_engine_execution() {
    for contact in [false, true] {
        let _ = run(contact, model(contact, 1));
        let input = model(contact, 32);
        let result = run(contact, input.clone());
        assert_eq!(result["converged"], true);
        assert_eq!(result["achieved_load_factor"], 1.0);
        let operator = if contact {
            "solve.contact_gap_1d"
        } else {
            "solve.nonlinear_spring_1d"
        };
        assert_eq!(result, run_solve_operator(operator, input).unwrap());
    }
}
