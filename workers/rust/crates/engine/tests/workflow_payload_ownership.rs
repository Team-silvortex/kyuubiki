use kyuubiki_engine::run_workflow_graph_with_options;
use kyuubiki_protocol::{
    WorkflowArtifactProjection, WorkflowGraphRunOptions, WorkflowGraphRunRequest,
};
use serde_json::{Value, json};

fn chain(count: usize, ephemeral: bool) -> WorkflowGraphRunRequest {
    let port = |id: &str| json!({"id":id,"artifact_type":"artifact/json"});
    let mut nodes = vec![json!({
        "id":"input","kind":"input","inputs":[],"outputs":[port("payload")]
    })];
    let mut edges = Vec::new();
    let mut previous = "input".to_string();
    for index in 0..count {
        let id = format!("pass-{index}");
        nodes.push(json!({
            "id":id,"kind":"transform","operator_id":"transform.first_available",
            "inputs":[port("payload")],"outputs":[port("payload")]
        }));
        edges.push(json!({
            "id":format!("link-{index}"),
            "from":{"node":previous,"port":"payload"},"to":{"node":id,"port":"payload"},
            "artifact_type":"artifact/json"
        }));
        previous = id;
    }
    nodes.push(json!({"id":"output","kind":"output","inputs":[port("payload")],"outputs":[]}));
    edges.push(json!({
        "id":"final","from":{"node":previous,"port":"payload"},
        "to":{"node":"output","port":"payload"},"artifact_type":"artifact/json"
    }));
    serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1","id":"workflow.payload-ownership",
            "name":"Payload ownership","version":"1.0.0","entry_nodes":["input"],
            "output_nodes":["output"],"defaults":{"cache_policy":if ephemeral {"ephemeral"} else {"cached"}},
            "nodes":nodes,"edges":edges
        },"input_artifacts":{"input":{"text":"x".repeat(128*1024),"items":[1,2,3]}}
    })).unwrap()
}

fn buffer(value: &Value) -> usize {
    value["text"].as_str().unwrap().as_ptr() as usize
}

#[test]
fn workflow_payload_ownership_compact_and_ephemeral_chains_move_the_original_buffer() {
    for count in [0, 1, 20] {
        for (ephemeral, projection) in [
            (false, WorkflowArtifactProjection::Outputs),
            (true, WorkflowArtifactProjection::All),
        ] {
            let request = chain(count, ephemeral);
            let original = buffer(&request.input_artifacts["input"]);
            let result = run_workflow_graph_with_options(
                request,
                WorkflowGraphRunOptions {
                    artifact_projection: projection,
                },
            )
            .unwrap();
            assert_eq!(result.artifacts.len(), 1);
            assert_eq!(result.completed_nodes.len(), count + 2);
            assert_eq!(buffer(&result.artifacts["output.payload"]), original);
        }
    }
}

#[test]
fn workflow_payload_ownership_cached_input_keeps_the_original_without_aliasing_outputs() {
    let request = chain(2, false);
    let original = buffer(&request.input_artifacts["input"]);
    let mut result = run_workflow_graph_with_options(
        request,
        WorkflowGraphRunOptions {
            artifact_projection: WorkflowArtifactProjection::All,
        },
    )
    .unwrap();
    assert_eq!(buffer(&result.artifacts["input.payload"]), original);
    assert_eq!(result.artifacts.len(), 4);
    assert_ne!(buffer(&result.artifacts["output.payload"]), original);
    result.artifacts.get_mut("output.payload").unwrap()["items"][0] = json!(99);
    assert_eq!(result.artifacts["input.payload"]["items"], json!([1, 2, 3]));
    assert_eq!(
        result.artifacts["pass-1.payload"]["items"],
        json!([1, 2, 3])
    );
}

#[test]
fn workflow_payload_ownership_last_shared_consumer_moves_without_aliasing_other_outputs() {
    let mut request = chain(0, false);
    let mut other = request.graph.nodes.last().unwrap().clone();
    other.id = "other".into();
    request.graph.nodes.push(other);
    request.graph.output_nodes.push("other".into());
    let mut edge = request.graph.edges[0].clone();
    edge.id = "other-edge".into();
    edge.to.node = "other".into();
    request.graph.edges.push(edge);
    let original = buffer(&request.input_artifacts["input"]);
    let mut result = run_workflow_graph_with_options(
        request,
        WorkflowGraphRunOptions {
            artifact_projection: WorkflowArtifactProjection::Outputs,
        },
    )
    .unwrap();
    assert_eq!(result.artifacts.len(), 2);
    assert_ne!(buffer(&result.artifacts["output.payload"]), original);
    assert_eq!(buffer(&result.artifacts["other.payload"]), original);
    result.artifacts.get_mut("output.payload").unwrap()["items"][0] = json!(99);
    assert_eq!(result.artifacts["other.payload"]["items"], json!([1, 2, 3]));
}

#[test]
fn workflow_payload_ownership_projection_cache_and_declaration_order_preserve_evidence() {
    for ephemeral in [false, true] {
        let full = run_workflow_graph_with_options(
            chain(4, ephemeral),
            WorkflowGraphRunOptions {
                artifact_projection: WorkflowArtifactProjection::All,
            },
        )
        .unwrap();
        for projection in [
            WorkflowArtifactProjection::Outputs,
            WorkflowArtifactProjection::None,
        ] {
            for reverse in [false, true] {
                let mut request = chain(4, ephemeral);
                if reverse {
                    request.graph.nodes.reverse();
                    request.graph.edges.reverse();
                }
                let run = run_workflow_graph_with_options(
                    request,
                    WorkflowGraphRunOptions {
                        artifact_projection: projection,
                    },
                )
                .unwrap();
                assert_eq!(run.completed_nodes, full.completed_nodes);
                assert_eq!(run.skipped_nodes, full.skipped_nodes);
                assert_eq!(run.failed_nodes, full.failed_nodes);
                assert_eq!(run.node_runs, full.node_runs);
                assert_eq!(run.artifact_lineage, full.artifact_lineage);
                assert_eq!(run.branch_decisions, full.branch_decisions);
                if projection == WorkflowArtifactProjection::None {
                    assert!(run.artifacts.is_empty());
                } else {
                    assert_eq!(run.artifacts.len(), 1);
                    assert_eq!(
                        run.artifacts["output.payload"],
                        full.artifacts["output.payload"]
                    );
                }
            }
        }
    }
}

#[test]
fn workflow_payload_ownership_failed_generated_output_retains_raw_evidence_and_clean_reuse() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-artifact-contract.json"
    ))
    .unwrap();
    for projection in [
        WorkflowArtifactProjection::All,
        WorkflowArtifactProjection::Outputs,
        WorkflowArtifactProjection::None,
    ] {
        let mut request: WorkflowGraphRunRequest =
            serde_json::from_value(fixture["request"].clone()).unwrap();
        request
            .input_artifacts
            .insert("input".into(), json!({"text":"x".repeat(500_001)}));
        let original = buffer(&request.input_artifacts["input"]);
        let run = run_workflow_graph_with_options(
            request,
            WorkflowGraphRunOptions {
                artifact_projection: projection,
            },
        )
        .unwrap();
        assert_eq!(run.failed_nodes, ["producer"]);
        assert_eq!(run.skipped_nodes.len(), 2);
        let failed = run
            .node_runs
            .iter()
            .find(|node| node.node_id == "producer")
            .unwrap();
        assert!(failed.produced_artifacts.is_empty());
        assert!(
            failed
                .error_message
                .as_deref()
                .unwrap()
                .contains("length security budget")
        );
        assert!(
            !run.artifact_lineage
                .iter()
                .any(|entry| entry.node_id == "producer")
        );
        assert!(!run.artifacts.contains_key("producer.payload"));
        assert!(!run.artifacts.contains_key("producer.aux"));
        if projection == WorkflowArtifactProjection::All {
            assert_eq!(buffer(&run.artifacts["input.payload"]), original);
            assert_eq!(
                run.artifacts["raw.payload"]["text"].as_str().unwrap().len(),
                500_001
            );
        } else if projection == WorkflowArtifactProjection::Outputs {
            assert_eq!(buffer(&run.artifacts["raw.payload"]), original);
            assert_eq!(run.artifacts.len(), 1);
        } else {
            assert!(run.artifacts.is_empty());
        }
        let healthy = run_workflow_graph_with_options(
            serde_json::from_value(fixture["request"].clone()).unwrap(),
            WorkflowGraphRunOptions {
                artifact_projection: projection,
            },
        )
        .unwrap();
        assert!(healthy.failed_nodes.is_empty());
        if projection != WorkflowArtifactProjection::None {
            assert_eq!(healthy.artifacts["output.payload"], json!({"value":42}));
        }
    }
}

#[test]
fn workflow_payload_ownership_raw_and_condition_forwarding_keep_the_existing_input_budget() {
    for condition in [false, true] {
        let mut request = chain(usize::from(condition), false);
        request
            .input_artifacts
            .insert("input".into(), json!({"text":"x".repeat(500_001)}));
        if condition {
            let branch = &mut request.graph.nodes[1];
            branch.kind = kyuubiki_protocol::WorkflowNodeKind::Condition;
            branch.operator_id = None;
            branch.config = Some(json!({"predicate":{"operator":"truthy"}}));
        }
        let original = buffer(&request.input_artifacts["input"]);
        let result = run_workflow_graph_with_options(
            request,
            WorkflowGraphRunOptions {
                artifact_projection: WorkflowArtifactProjection::Outputs,
            },
        )
        .unwrap();
        assert!(result.failed_nodes.is_empty());
        assert_eq!(buffer(&result.artifacts["output.payload"]), original);
        assert_eq!(
            result.artifacts["output.payload"]["text"]
                .as_str()
                .unwrap()
                .len(),
            500_001
        );
    }
    let mut generated = chain(1, false);
    generated
        .input_artifacts
        .insert("input".into(), json!({"text":"x".repeat(500_001)}));
    let error = run_workflow_graph_with_options(
        generated,
        WorkflowGraphRunOptions {
            artifact_projection: WorkflowArtifactProjection::None,
        },
    )
    .unwrap_err();
    assert!(
        error.contains("workflow node pass-0 output") && error.contains("length security budget"),
        "{error}"
    );
}

#[test]
fn workflow_payload_ownership_zero_output_producers_still_validate_generated_results() {
    let mut request = chain(1, false);
    request.graph.nodes.retain(|node| node.id != "output");
    request.graph.nodes[1].outputs.clear();
    request.graph.edges.retain(|edge| edge.to.node != "output");
    request.graph.output_nodes.clear();
    request
        .input_artifacts
        .insert("input".into(), json!({"text":"x".repeat(500_001)}));
    let error = run_workflow_graph_with_options(
        request.clone(),
        WorkflowGraphRunOptions {
            artifact_projection: WorkflowArtifactProjection::None,
        },
    )
    .unwrap_err();
    assert!(
        error.contains("workflow node pass-0 output") && error.contains("length security budget"),
        "{error}"
    );
    request.graph.nodes[1].config = Some(json!({"on_error":"skip"}));
    let result = run_workflow_graph_with_options(
        request,
        WorkflowGraphRunOptions {
            artifact_projection: WorkflowArtifactProjection::None,
        },
    )
    .unwrap();
    assert_eq!(result.failed_nodes, ["pass-0"]);
    assert!(result.artifacts.is_empty());
    assert!(
        result
            .node_runs
            .last()
            .unwrap()
            .produced_artifacts
            .is_empty()
    );
    assert!(
        !result
            .artifact_lineage
            .iter()
            .any(|entry| entry.node_id == "pass-0")
    );
}
