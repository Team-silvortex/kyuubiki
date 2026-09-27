use crate::workflow_security::validate_workflow_artifact_budget;
use crate::{run_workflow_graph, run_workflow_graph_with_options};
use kyuubiki_protocol::{
    WorkflowArtifactProjection, WorkflowGraphRunOptions, WorkflowNodeRunStatus,
};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/workflow-artifact-contract.json"
    ))
    .unwrap()
}

fn value(recipe: &Value) -> Value {
    let size = recipe["size"].as_u64().unwrap_or_default() as usize;
    let text = recipe["text"].as_str().unwrap_or("x");
    match recipe["kind"].as_str().unwrap() {
        "literal" => recipe["value"].clone(),
        "string" => json!(text.repeat(size)),
        "key" => json!({text.repeat(size): 0}),
        "array" => json!(vec![0; size]),
        "nested" => (0..size).fold(json!(0), |inner, _| json!([inner])),
        "late_nul" => {
            let mut values = vec![json!(0); size];
            values.push(json!({"bad": "\0"}));
            json!(values)
        }
        kind => panic!("unknown shared output recipe {kind}"),
    }
}

#[test]
fn shared_output_artifact_budget_boundaries() {
    for recipe in fixture()["cases"].as_array().unwrap() {
        let result = validate_workflow_artifact_budget("test output", &value(recipe));
        assert_eq!(
            result.is_ok(),
            recipe["accept"] == true,
            "{}: {result:?}",
            recipe["id"]
        );
        if let Some(expected) = recipe["error"].as_str() {
            assert!(result.unwrap_err().contains(expected));
        }
    }
}

#[test]
fn shared_output_publication_is_atomic_under_recovery_and_projection() {
    let fixture = fixture();
    for export in [false, true] {
        for oversized in [false, true] {
            for skip in [false, true] {
                for reverse in [false, true] {
                    for projection in [
                        WorkflowArtifactProjection::All,
                        WorkflowArtifactProjection::Outputs,
                        WorkflowArtifactProjection::None,
                    ] {
                        let mut request = fixture["request"].clone();
                        let producer = &mut request["graph"]["nodes"][1];
                        producer["config"]["on_error"] = json!(if skip { "skip" } else { "fail" });
                        if export {
                            producer["kind"] = json!("export");
                            producer["operator_id"] = json!("export.summary_json");
                        }
                        request["input_artifacts"]["input"] = if !oversized {
                            json!({"text": "healthy"})
                        } else if export {
                            json!({"text": "\n".repeat(250_000)})
                        } else {
                            json!({"text": "x".repeat(500_001)})
                        };
                        if reverse {
                            request["graph"]["nodes"].as_array_mut().unwrap().reverse();
                        }
                        let result = run_workflow_graph_with_options(
                            serde_json::from_value(request.clone()).unwrap(),
                            WorkflowGraphRunOptions {
                                artifact_projection: projection,
                            },
                        );
                        if oversized && !skip {
                            let error =
                                result.expect_err("invalid output must fail before publication");
                            assert!(
                                error.contains("producer")
                                    && error.contains("length security budget")
                            );
                            continue;
                        }
                        let result = result.unwrap();
                        if oversized {
                            assert_eq!(result.failed_nodes, ["producer"]);
                            assert_eq!(result.skipped_nodes.len(), 2);
                            assert_eq!(result.completed_nodes.len(), 2);
                            let trace = result
                                .node_runs
                                .iter()
                                .find(|run| run.node_id == "producer")
                                .unwrap();
                            assert_eq!(trace.status, WorkflowNodeRunStatus::Failed);
                            assert!(trace.produced_artifacts.is_empty());
                            assert!(
                                trace
                                    .error_message
                                    .as_deref()
                                    .unwrap()
                                    .contains("length security budget")
                            );
                            assert!(
                                !result
                                    .artifact_lineage
                                    .iter()
                                    .any(|entry| entry.node_id == "producer")
                            );
                            for key in [
                                "producer.payload",
                                "producer.aux",
                                "consumer.payload",
                                "output.payload",
                            ] {
                                assert!(!result.artifacts.contains_key(key));
                            }
                        } else {
                            assert!(result.failed_nodes.is_empty());
                            if projection != WorkflowArtifactProjection::None {
                                let actual = &result.artifacts["output.payload"];
                                if export {
                                    assert_eq!(
                                        serde_json::from_str::<Value>(
                                            actual["content"].as_str().unwrap()
                                        )
                                        .unwrap(),
                                        json!({"text": "healthy"})
                                    );
                                } else {
                                    assert_eq!(actual, &json!({"text": "healthy"}));
                                }
                            }
                        }
                        if projection == WorkflowArtifactProjection::None {
                            assert!(result.artifacts.is_empty());
                        } else {
                            assert_eq!(
                                result.artifacts["raw.payload"],
                                request["input_artifacts"]["input"]
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn raw_input_and_condition_forwarding_keep_the_input_budget() {
    let mut request = fixture()["request"].clone();
    let producer = &mut request["graph"]["nodes"][1];
    producer["kind"] = json!("condition");
    producer["config"] = json!({"predicate": {"operator": "truthy"}});
    request["graph"]["nodes"].as_array_mut().unwrap().remove(2);
    request["graph"]["edges"][1]["to"]["node"] = json!("output");
    request["graph"]["edges"].as_array_mut().unwrap().remove(2);
    request["input_artifacts"]["input"] = json!({"text": "x".repeat(500_001)});
    let result = run_workflow_graph(serde_json::from_value(request.clone()).unwrap()).unwrap();
    assert!(result.failed_nodes.is_empty());
    assert_eq!(
        result.artifacts["output.payload"],
        request["input_artifacts"]["input"]
    );
}
