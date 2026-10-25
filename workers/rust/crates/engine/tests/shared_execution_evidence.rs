use kyuubiki_engine::run_workflow_graph_with_options;
use kyuubiki_protocol::{
    WorkflowArtifactProjection, WorkflowCachePolicy, WorkflowGraphRunOptions,
    WorkflowGraphRunRequest, WorkflowGraphRunResult, WorkflowNodeRunStatus,
};
use serde_json::{Value, json};

const PROJECTIONS: [WorkflowArtifactProjection; 3] = [
    WorkflowArtifactProjection::All,
    WorkflowArtifactProjection::Outputs,
    WorkflowArtifactProjection::None,
];

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-execution-evidence.json"
    ))
    .unwrap()
}

fn request(value: Value, reverse: bool, backup_first: bool) -> WorkflowGraphRunRequest {
    let mut input: WorkflowGraphRunRequest =
        serde_json::from_value(fixture()["request"].clone()).unwrap();
    input.input_artifacts.insert("a".into(), value.clone());
    input.input_artifacts.insert("b".into(), value);
    if reverse {
        input.graph.nodes.reverse();
    }
    if backup_first {
        input.graph.edges.swap(0, 1);
    }
    input
}

fn run(
    input: WorkflowGraphRunRequest,
    projection: WorkflowArtifactProjection,
) -> WorkflowGraphRunResult {
    run_workflow_graph_with_options(
        input,
        WorkflowGraphRunOptions {
            artifact_projection: projection,
        },
    )
    .unwrap()
}

fn assert_publications(result: &WorkflowGraphRunResult) {
    for trace in &result.node_runs {
        let mut expected = result
            .artifact_lineage
            .iter()
            .filter(|entry| entry.node_id == trace.node_id)
            .map(|entry| entry.artifact_key.clone())
            .collect::<Vec<_>>();
        let mut actual = trace.produced_artifacts.clone();
        expected.sort();
        actual.sort();
        assert_eq!(actual, expected, "{}", trace.node_id);
        if trace.status != WorkflowNodeRunStatus::Completed {
            assert!(actual.is_empty());
        }
    }
}

#[test]
fn shared_execution_evidence_conditions_record_only_actual_publications_in_all_projections() {
    for sample in fixture()["cases"].as_array().unwrap() {
        for reverse in [false, true] {
            for backup_first in [false, true] {
                for projection in PROJECTIONS {
                    for cache in [WorkflowCachePolicy::Cached, WorkflowCachePolicy::Ephemeral] {
                        let mut input = request(sample["value"].clone(), reverse, backup_first);
                        input.graph.defaults.cache_policy = Some(cache);
                        let result = run(input, projection);
                        let branch = result
                            .node_runs
                            .iter()
                            .find(|node| node.node_id == "branch")
                            .unwrap();
                        let key = format!("branch.{}", sample["chosen_output"].as_str().unwrap());
                        assert_eq!(branch.produced_artifacts, [key.clone()]);
                        let choose = result
                            .node_runs
                            .iter()
                            .find(|node| node.node_id == "choose")
                            .unwrap();
                        assert_eq!(
                            choose.consumed_artifacts,
                            [if backup_first {
                                "b.payload"
                            } else {
                                "a.payload"
                            }]
                        );
                        assert!(result.failed_nodes.is_empty());
                        assert_publications(&result);
                        if projection == WorkflowArtifactProjection::All {
                            if cache == WorkflowCachePolicy::Cached {
                                assert_eq!(result.artifacts.get(&key), Some(&sample["value"]));
                            }
                            assert!(!result.artifacts.contains_key("branch.unused"));
                        }
                        if projection != WorkflowArtifactProjection::None {
                            let output = if sample["chosen_output"] == "if_true" {
                                "yes"
                            } else {
                                "no"
                            };
                            assert_eq!(
                                result.artifacts.get(&format!("{output}.payload")),
                                Some(&sample["value"])
                            );
                        } else {
                            assert!(result.artifacts.is_empty());
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn shared_execution_evidence_failed_fallbacks_name_only_selected_and_aliased_sources() {
    for (selected, aliased) in [("a", false), ("b", false), ("a", true)] {
        for reverse in [false, true] {
            for nested in [false, true] {
                for projection in PROJECTIONS {
                    let mut input = request(json!(true), reverse, selected == "b");
                    input
                        .input_artifacts
                        .insert(selected.into(), json!({"text":"x".repeat(500_001)}));
                    if aliased {
                        input
                            .graph
                            .edges
                            .iter_mut()
                            .find(|edge| edge.id == "b-choose")
                            .unwrap()
                            .from
                            .node = "a".into();
                    }
                    if nested {
                        input
                            .graph
                            .nodes
                            .iter_mut()
                            .find(|node| node.id == "choose")
                            .unwrap()
                            .config = Some(json!({"recovery":{"on_error":"skip"}}));
                    }
                    let result = run(input, projection);
                    assert_eq!(result.failed_nodes, ["choose"]);
                    assert_eq!(result.skipped_nodes.len(), 3);
                    let trace = result
                        .node_runs
                        .iter()
                        .find(|node| node.node_id == "choose")
                        .unwrap();
                    assert_eq!(trace.consumed_artifacts, [format!("{selected}.payload")]);
                    assert!(
                        trace
                            .error_message
                            .as_deref()
                            .unwrap()
                            .contains("length security budget")
                    );
                    assert!(result.branch_decisions.is_empty());
                    assert_publications(&result);
                    if projection != WorkflowArtifactProjection::None {
                        assert_eq!(
                            result.artifacts[&format!("raw_{selected}.payload")]["text"]
                                .as_str()
                                .unwrap()
                                .len(),
                            500_001
                        );
                    } else {
                        assert!(result.artifacts.is_empty());
                    }
                }
            }
        }
    }
}

#[test]
fn shared_execution_evidence_failed_named_inputs_keep_both_real_bindings() {
    for aliased in [false, true] {
        let mut input = request(Value::Null, false, false);
        input
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.id == "choose")
            .unwrap()
            .operator_id = Some("transform.benchmark_structural_pair".into());
        if aliased {
            input
                .graph
                .edges
                .iter_mut()
                .find(|edge| edge.id == "b-choose")
                .unwrap()
                .from
                .node = "a".into();
        }
        let result = run(input, WorkflowArtifactProjection::All);
        assert_eq!(result.failed_nodes, ["choose"]);
        let trace = result
            .node_runs
            .iter()
            .find(|node| node.node_id == "choose")
            .unwrap();
        assert_eq!(
            trace.consumed_artifacts,
            ["a.payload", if aliased { "a.payload" } else { "b.payload" }]
        );
        assert_publications(&result);
    }
}

fn skipped_request(value: Value, mode: &str, reverse: bool) -> WorkflowGraphRunRequest {
    let mut input: WorkflowGraphRunRequest = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-skipped-input-evidence.json"
    ))
    .unwrap();
    input.input_artifacts.insert("a".into(), value);
    match mode {
        "inactive" => (),
        "active" => {
            input.input_artifacts.insert("flag".into(), json!(true));
        }
        "missing" => {
            input.input_artifacts.remove("flag");
        }
        "failed" => {
            input
                .graph
                .nodes
                .iter_mut()
                .find(|node| node.id == "gate")
                .unwrap()
                .config
                .as_mut()
                .unwrap()["predicate"]["operator"] = json!("invalid");
        }
        _ => unreachable!(),
    }
    if reverse {
        input.graph.nodes.reverse();
    }
    input
}

#[test]
fn shared_execution_evidence_skipped_joins_do_not_consume_available_inputs() {
    for value in [json!(false), json!(0), Value::Null] {
        for mode in ["inactive", "failed", "missing"] {
            for reverse in [false, true] {
                for projection in PROJECTIONS {
                    for cache in [WorkflowCachePolicy::Cached, WorkflowCachePolicy::Ephemeral] {
                        let mut input = skipped_request(value.clone(), mode, reverse);
                        input.graph.defaults.cache_policy = Some(cache);
                        let result = run(input, projection);
                        let expected_failures: &[&str] = match mode {
                            "inactive" => &[],
                            "failed" => &["gate"],
                            _ => &["flag"],
                        };
                        assert_eq!(result.failed_nodes, expected_failures);
                        for id in ["strict", "blocked", "strict_result"] {
                            assert!(result.skipped_nodes.iter().any(|node| node == id));
                        }
                        for trace in &result.node_runs {
                            if trace.status == WorkflowNodeRunStatus::Skipped {
                                assert!(
                                    trace.consumed_artifacts.is_empty(),
                                    "{}: {:?}",
                                    trace.node_id,
                                    trace.consumed_artifacts
                                );
                                assert!(trace.produced_artifacts.is_empty());
                                assert!(
                                    !result
                                        .artifact_lineage
                                        .iter()
                                        .any(|entry| entry.node_id == trace.node_id)
                                );
                            }
                            if trace.node_id == "fallback" {
                                assert_eq!(trace.consumed_artifacts, ["a.payload"]);
                            }
                        }
                        assert_publications(&result);
                        if projection == WorkflowArtifactProjection::None {
                            assert!(result.artifacts.is_empty());
                        } else {
                            assert_eq!(result.artifacts.get("result.payload"), Some(&value));
                            assert_eq!(result.artifacts.get("raw.payload"), Some(&value));
                            if projection == WorkflowArtifactProjection::Outputs
                                || cache == WorkflowCachePolicy::Ephemeral
                            {
                                assert_eq!(result.artifacts.len(), 2);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn shared_execution_evidence_executed_failures_still_record_inputs() {
    for projection in PROJECTIONS {
        for reverse in [false, true] {
            let result = run(skipped_request(Value::Null, "active", reverse), projection);
            assert_eq!(result.failed_nodes, ["strict"]);
            assert_eq!(result.skipped_nodes, ["strict_result"]);
            for trace in &result.node_runs {
                match trace.node_id.as_str() {
                    "strict" | "blocked" => {
                        assert_eq!(trace.consumed_artifacts, ["a.payload", "gate.if_true"])
                    }
                    "strict_result" => assert!(trace.consumed_artifacts.is_empty()),
                    "fallback" => assert_eq!(trace.consumed_artifacts, ["gate.if_true"]),
                    _ => (),
                }
            }
            assert_publications(&result);
            if projection != WorkflowArtifactProjection::None {
                assert_eq!(result.artifacts.get("result.payload"), Some(&json!(true)));
                assert_eq!(result.artifacts.get("blocked.left"), Some(&Value::Null));
                assert_eq!(result.artifacts.get("blocked.right"), Some(&json!(true)));
            }
        }
    }
}
