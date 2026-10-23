use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use serde_json::{Value, json};

#[test]
fn workflow_branched_erased_load_fails_admission_and_replays_with_a_resolved_load() {
    for sign in [-1.0, 1.0] {
        let mut input = model(4.0, 4.0);
        for node in input["nodes"].as_array_mut().unwrap() {
            node["load_x"] = json!(node["load_x"].as_f64().unwrap() * sign);
        }
        input["nodes"][1025]["load_x"] = json!(sign * f64::from_bits(1));
        let decoded = serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
        let direct = run_solve_operator("solve.spring_1d", decoded);
        assert!(
            direct.is_err(),
            "dispatch accepted an erased independent load"
        );
        assert!(direct.unwrap_err().contains("residual validation"));
        let run = run_workflow_graph(workflow(input.clone()));
        assert!(
            run.is_err(),
            "workflow published an erased independent load"
        );
        assert!(run.unwrap_err().contains("residual validation"));
        input["nodes"][1025]["load_x"] = json!(sign * 4.0);
        let direct = run_solve_operator("solve.spring_1d", input.clone()).unwrap();
        let run = run_workflow_graph(workflow(input)).unwrap();
        assert!(run.failed_nodes.is_empty());
        assert_eq!(run.artifacts["output.result"], direct);
        let decoded: Value = serde_json::from_slice(&serde_json::to_vec(&direct).unwrap()).unwrap();
        assert!((decoded["nodes"][1025]["ux"].as_f64().unwrap() - sign).abs() < 1e-8);
    }
}

#[test]
fn workflow_preserves_subnormal_back_substitution_and_force_balance() {
    for bits in [1, 3, 7] {
        for sign in [-1.0, 1.0] {
            let load = sign * f64::from_bits(bits);
            let input = json!({
                "nodes": [
                    {"id":"base", "x":0.0, "fix_x":true, "load_x":0.0},
                    {"id":"middle", "x":1.0, "fix_x":false, "load_x":0.0},
                    {"id":"tip", "x":2.0, "fix_x":false, "load_x":load}
                ],
                "elements": [
                    {"id":"anchor", "node_i":0, "node_j":1, "stiffness":0.5},
                    {"id":"coupling", "node_i":1, "node_j":2, "stiffness":0.25}
                ]
            });
            let decoded = serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
            let direct = run_solve_operator("solve.spring_1d", decoded).unwrap();
            let run = run_workflow_graph(workflow(input)).unwrap();
            assert!(run.failed_nodes.is_empty());
            assert_eq!(run.artifacts["output.result"], direct);
            let roundtrip: Value =
                serde_json::from_slice(&serde_json::to_vec(&direct).unwrap()).unwrap();
            assert_eq!(roundtrip["nodes"][1]["ux"].as_f64().unwrap(), 2.0 * load);
            assert_eq!(roundtrip["nodes"][2]["ux"].as_f64().unwrap(), 6.0 * load);
            for index in [0, 1] {
                assert_eq!(
                    roundtrip["elements"][index]["force"].as_f64().unwrap(),
                    load
                );
            }
        }
    }
}

#[test]
fn workflow_preserves_subnormal_forward_work_through_json_and_dispatch() {
    for bits in [1, 3, 7] {
        for sign in [-1.0, 1.0] {
            let load = sign * f64::from_bits(bits);
            let input = json!({
                "nodes": [
                    {"id":"base", "x":0.0, "fix_x":true, "load_x":0.0},
                    {"id":"loaded", "x":1.0, "fix_x":false, "load_x":load},
                    {"id":"tip", "x":2.0, "fix_x":false, "load_x":0.0}
                ],
                "elements": [
                    {"id":"anchor", "node_i":0, "node_j":1, "stiffness":0.5},
                    {"id":"coupling", "node_i":1, "node_j":2, "stiffness":0.5}
                ]
            });
            let decoded = serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
            let direct = run_solve_operator("solve.spring_1d", decoded).unwrap();
            let run = run_workflow_graph(workflow(input)).unwrap();
            assert!(run.failed_nodes.is_empty());
            assert_eq!(run.artifacts["output.result"], direct);
            let roundtrip: Value =
                serde_json::from_slice(&serde_json::to_vec(&direct).unwrap()).unwrap();
            for index in [1, 2] {
                assert_eq!(
                    roundtrip["nodes"][index]["ux"].as_f64().unwrap(),
                    2.0 * load
                );
            }
            assert_eq!(roundtrip["elements"][0]["force"].as_f64().unwrap(), load);
            assert_eq!(roundtrip["elements"][1]["force"].as_f64().unwrap(), 0.0);
        }
    }
}

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

#[test]
fn chain_weak_response_range_failure_cannot_publish_workflow_success() {
    let quantum = f64::from_bits(1);
    for sign in [-1.0, 1.0] {
        let mut input = json!({
            "nodes": [
                {"id":"base", "x":0.0, "fix_x":true, "load_x":0.0},
                {"id":"strong", "x":1.0, "fix_x":false, "load_x":sign},
                {"id":"weak", "x":2.0, "fix_x":false, "load_x":sign * quantum}
            ],
            "elements": [
                {"id":"strong", "node_i":0, "node_j":1, "stiffness":1.0},
                {"id":"weak", "node_i":0, "node_j":2, "stiffness":4.0}
            ]
        });
        let decoded = serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
        let error = run_solve_operator("solve.spring_1d", decoded).unwrap_err();
        assert!(error.contains("backward-error"), "{error}");
        let error = run_workflow_graph(workflow(input.clone())).unwrap_err();
        assert!(error.contains("workflow node solve failed"), "{error}");
        assert!(error.contains("backward-error"), "{error}");

        input["nodes"][2]["load_x"] = json!(sign * 4.0 * quantum);
        let direct = run_solve_operator("solve.spring_1d", input.clone()).unwrap();
        let run = run_workflow_graph(workflow(input)).unwrap();
        assert!(run.failed_nodes.is_empty());
        assert_eq!(run.artifacts["output.result"], direct);
        assert_eq!(direct["nodes"][1]["ux"], sign);
        assert_eq!(direct["nodes"][2]["ux"], sign * quantum);
    }
}
