use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use serde_json::{Value, json};

fn model(contact: bool, restrained: bool) -> Value {
    let mut input = json!({
        "nodes": [
            {"id":"base-a", "x":0.0, "fix_x":true, "load_x":0.0},
            {"id":"tip-a", "x":1.0, "fix_x":false, "load_x":0.0},
            {"id":"base-b", "x":2.0, "fix_x":restrained, "load_x":0.0},
            {"id":"tip-b", "x":3.0, "fix_x":false, "load_x":0.0}
        ],
        "elements": [
            {"id":"a", "node_i":0, "node_j":1, "stiffness":2.0, "cubic_stiffness":1.0},
            {"id":"b", "node_i":2, "node_j":3, "stiffness":2.0, "cubic_stiffness":1.0}
        ],
        "load_steps":4, "max_iterations":32, "tolerance":1e-10
    });
    if contact {
        input["contacts"] = json!([{"id":"stop", "node":3, "gap":0.5, "normal_stiffness":10.0}]);
    }
    input
}

fn workflow(family: &str, input: Value) -> WorkflowGraphRunRequest {
    let input_type = format!("study_model/{family}");
    let output_type = format!("result/{family}");
    serde_json::from_value(json!({
        "graph": {
            "schema_version":"kyuubiki.workflow-graph/v1", "id":"spring-topology",
            "name":"Spring topology", "version":"1.0.0",
            "entry_nodes":["input"], "output_nodes":["output"],
            "nodes":[
                {"id":"input", "kind":"input", "inputs":[],
                 "outputs":[{"id":"model", "artifact_type":input_type}]},
                {"id":"solve", "kind":"solve", "operator_id":format!("solve.{family}"),
                 "inputs":[{"id":"model", "artifact_type":input_type}],
                 "outputs":[{"id":"result", "artifact_type":output_type}]},
                {"id":"output", "kind":"output", "inputs":[{"id":"result", "artifact_type":output_type}],
                 "outputs":[]}
            ],
            "edges":[
                {"id":"in", "from":{"node":"input", "port":"model"},
                 "to":{"node":"solve", "port":"model"}, "artifact_type":input_type},
                {"id":"out", "from":{"node":"solve", "port":"result"},
                 "to":{"node":"output", "port":"result"}, "artifact_type":output_type}
            ]
        },
        "input_artifacts":{"input":input}
    }))
    .unwrap()
}

#[test]
fn direct_and_graph_dispatch_propagate_the_operator_restraint_error() {
    for (contact, family) in [(false, "nonlinear_spring_1d"), (true, "contact_gap_1d")] {
        for error in [
            run_solve_operator(&format!("solve.{family}"), model(contact, false)).unwrap_err(),
            run_workflow_graph(workflow(family, model(contact, false))).unwrap_err(),
        ] {
            assert!(error.contains("unrestrained"), "{error}");
            assert!(error.contains("base-b"), "{error}");
        }
    }
}

#[test]
fn correcting_the_model_produces_real_equilibrium_without_engine_special_cases() {
    for (contact, family) in [(false, "nonlinear_spring_1d"), (true, "contact_gap_1d")] {
        assert!(run_workflow_graph(workflow(family, model(contact, false))).is_err());
        let mut input = model(contact, true);
        input["nodes"][1]["load_x"] = json!(-3.0);
        input["nodes"][3]["load_x"] = json!(if contact { 8.0 } else { 3.0 });
        let direct = run_solve_operator(&format!("solve.{family}"), input.clone()).unwrap();
        let run = run_workflow_graph(workflow(family, input)).unwrap();
        assert_eq!(run.artifacts["output.result"], direct);
        assert_eq!(direct["converged"], true);
        assert_eq!(direct["achieved_load_factor"], 1.0);
        assert!((direct["nodes"][1]["ux"].as_f64().unwrap() + 1.0).abs() < 1e-10);
        assert!((direct["nodes"][3]["ux"].as_f64().unwrap() - 1.0).abs() < 1e-10);
    }
}
