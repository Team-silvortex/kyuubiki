use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use serde_json::{Value, json};

fn model(strong: f64, weak: f64) -> Value {
    let nodes: Vec<_> = (0..=1025)
        .map(|index| {
            json!({
                "id": format!("n{index}"), "x": index, "fix_x": index == 0,
                "load_x": match index { 0 | 3..=5 => 0.0, 2 => weak, _ => strong }
            })
        })
        .collect();
    let mut elements: Vec<_> = (1..=1025)
        .map(|index| {
            json!({
                "id": format!("anchor{index}"), "node_i": 0, "node_j": index,
                "stiffness": if (2..=5).contains(&index) { weak } else { strong }
            })
        })
        .collect();
    elements.extend((3..=5).map(|leaf| {
        json!({
            "id": format!("branch{leaf}"), "node_i": 2, "node_j": leaf, "stiffness": weak
        })
    }));
    json!({"nodes": nodes, "elements": elements})
}

fn workflow(input: Value) -> WorkflowGraphRunRequest {
    serde_json::from_value(json!({
        "graph": {
            "schema_version": "kyuubiki.workflow-graph/v1",
            "id": "mixed-scale-spring", "name": "Mixed-scale spring", "version": "1.0.0",
            "entry_nodes": ["input"], "output_nodes": ["output"],
            "nodes": [
                {"id": "input", "kind": "input", "inputs": [],
                 "outputs": [{"id": "model", "artifact_type": "study_model/spring_1d"}]},
                {"id": "solve", "kind": "solve", "operator_id": "solve.spring_1d",
                 "inputs": [{"id": "model", "artifact_type": "study_model/spring_1d"}],
                 "outputs": [{"id": "result", "artifact_type": "result/spring_1d"}]},
                {"id": "output", "kind": "output", "outputs": [],
                 "inputs": [{"id": "result", "artifact_type": "result/spring_1d"}]}
            ],
            "edges": [
                {"id": "in", "from": {"node": "input", "port": "model"},
                 "to": {"node": "solve", "port": "model"}, "artifact_type": "study_model/spring_1d"},
                {"id": "out", "from": {"node": "solve", "port": "result"},
                 "to": {"node": "output", "port": "result"}, "artifact_type": "result/spring_1d"}
            ]
        },
        "input_artifacts": {"input": input}
    })).unwrap()
}

#[test]
fn workflow_weak_equilibrium_survives_json_dispatch_and_failed_range_replay() {
    let mut input = model(1e300, 1e-300);
    // Unlike a tiny inner product, this load requires an unrepresentable displacement.
    input["nodes"][2]["load_x"] = json!(1e300);
    assert!(run_solve_operator("solve.spring_1d", input.clone()).is_err());
    assert!(run_workflow_graph(workflow(input)).is_err());

    for (strong, weak) in [(1e20, 1e-20), (1e300, 1e-300)] {
        let input = model(strong, weak);
        let direct = run_solve_operator("solve.spring_1d", input.clone()).unwrap();
        let run = run_workflow_graph(workflow(input)).unwrap();
        assert_eq!(run.artifacts["output.result"], direct);
        let decoded: Value = serde_json::from_slice(&serde_json::to_vec(&direct).unwrap()).unwrap();
        for (index, expected) in [(1, 1.0), (2, 0.4), (3, 0.2), (4, 0.2), (5, 0.2)] {
            let actual = decoded["nodes"][index]["ux"].as_f64().unwrap();
            assert!((actual - expected).abs() < 1e-8, "node {index}: {actual}");
        }
    }
}
