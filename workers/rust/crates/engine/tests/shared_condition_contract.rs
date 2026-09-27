use kyuubiki_engine::run_workflow_graph;
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-condition-contract.json"
    ))
    .unwrap()
}

fn request(fixture: &Value, sample: &Value, policy: &str, reverse: bool) -> Value {
    let mut request = fixture["request"].clone();
    request["input_artifacts"]["input"] = sample["payload"].clone();
    let nodes = request["graph"]["nodes"].as_array_mut().unwrap();
    let gate = nodes.iter_mut().find(|node| node["id"] == "gate").unwrap();
    gate["config"] = sample["config"].clone();
    if let Some(config) = gate["config"].as_object_mut() {
        config.insert("on_error".into(), policy.into());
    }
    if reverse {
        nodes.reverse();
    }
    request
}

#[test]
fn shared_condition_decisions_and_validation_match_contract() {
    let fixture = fixture();
    for sample in fixture["cases"].as_array().unwrap() {
        for reverse in [false, true] {
            let result = run_workflow_graph(
                serde_json::from_value(request(&fixture, sample, "fail", reverse)).unwrap(),
            );
            if let Some(path) = sample["error"].as_str() {
                let error = result.expect_err(&format!("{} must fail", sample["id"]));
                assert!(
                    error.contains("gate") && error.contains(path),
                    "{}: {error}",
                    sample["id"]
                );
            } else {
                let result = result.unwrap_or_else(|error| panic!("{}: {error}", sample["id"]));
                let decision = sample["decision"].as_bool().unwrap();
                assert!(result.failed_nodes.is_empty());
                assert_eq!(result.artifacts["raw.payload"], sample["payload"]);
                assert_eq!(result.branch_decisions.len(), 1);
                assert_eq!(
                    result.branch_decisions[0].predicate_result, decision,
                    "{}",
                    sample["id"]
                );
                assert_eq!(
                    result.branch_decisions[0].chosen_output,
                    if decision { "if_true" } else { "if_false" }
                );
                assert_eq!(result.artifacts.contains_key("accepted.payload"), decision);
                assert_eq!(result.artifacts.contains_key("rejected.payload"), !decision);
                if decision {
                    assert_eq!(result.artifacts["accepted.payload"], sample["payload"]);
                }
            }
        }
    }
}

#[test]
fn shared_invalid_conditions_isolate_without_branch_artifacts_and_replay_cleanly() {
    let fixture = fixture();
    for sample in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|sample| sample["error"].is_string() && sample["config"].is_object())
    {
        for reverse in [false, true] {
            let result = run_workflow_graph(
                serde_json::from_value(request(&fixture, sample, "skip", reverse)).unwrap(),
            )
            .unwrap();
            assert_eq!(result.failed_nodes, ["gate"], "{}", sample["id"]);
            let mut skipped = result.skipped_nodes.clone();
            skipped.sort();
            assert_eq!(skipped, ["accepted", "consumer", "rejected"]);
            assert!(result.branch_decisions.is_empty());
            assert_eq!(result.artifacts.len(), 2);
            assert_eq!(result.artifacts["raw.payload"], sample["payload"]);
            assert!(result.artifacts.contains_key("input.payload"));
            assert!(
                !result
                    .artifact_lineage
                    .iter()
                    .any(|lineage| lineage.node_id == "gate")
            );
            let trace = result
                .node_runs
                .iter()
                .find(|trace| trace.node_id == "gate")
                .unwrap();
            assert!(trace.produced_artifacts.is_empty());
            assert!(
                trace
                    .error_message
                    .as_ref()
                    .unwrap()
                    .contains(sample["error"].as_str().unwrap())
            );
        }
    }
    let sample = json!({"payload": true, "config": {"predicate": {"operator": "truthy"}}});
    let clean = run_workflow_graph(
        serde_json::from_value(request(&fixture, &sample, "fail", false)).unwrap(),
    )
    .unwrap();
    assert!(clean.failed_nodes.is_empty());
    assert!(clean.artifacts.contains_key("accepted.payload"));
}
