use kyuubiki_engine::run_workflow_graph_with_options;
use kyuubiki_protocol::{
    WorkflowArtifactProjection, WorkflowCachePolicy, WorkflowGraphRunOptions,
    WorkflowGraphRunRequest, WorkflowGraphRunResult,
};
use serde_json::{Value, json};

fn request(
    primary: Value,
    backup: Value,
    reverse_nodes: bool,
    reverse_edges: bool,
) -> WorkflowGraphRunRequest {
    let port = |id| json!({"id":id,"artifact_type":"artifact/json"});
    let edge = |id, from, to, target| {
        json!({"id":id,"artifact_type":"artifact/json",
        "from":{"node":from,"port":"payload"},"to":{"node":to,"port":target}})
    };
    let mut value = json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1","id":"workflow.selected-source",
            "name":"Selected source","version":"1.0.0","entry_nodes":["primary","backup"],
            "output_nodes":["output","raw_primary","raw_backup"],
            "nodes":[
                {"id":"primary","kind":"input","inputs":[],"outputs":[port("payload")]},
                {"id":"join","kind":"transform","operator_id":"transform.first_available",
                 "inputs":[port("left"),port("right")],"outputs":[port("payload")]},
                {"id":"backup","kind":"input","inputs":[],"outputs":[port("payload")]},
                {"id":"output","kind":"output","inputs":[port("payload")],"outputs":[]},
                {"id":"raw_primary","kind":"output","inputs":[port("payload")],"outputs":[]},
                {"id":"raw_backup","kind":"output","inputs":[port("payload")],"outputs":[]}
            ],"edges":[edge("primary-join","primary","join","left"),edge("backup-join","backup","join","right"),
                edge("join-output","join","output","payload"),edge("primary-raw","primary","raw_primary","payload"),
                edge("backup-raw","backup","raw_backup","payload")]
        },"input_artifacts":{"primary":primary,"backup":backup}
    });
    if reverse_nodes {
        value["graph"]["nodes"].as_array_mut().unwrap().reverse();
    }
    if reverse_edges {
        value["graph"]["edges"].as_array_mut().unwrap().reverse();
    }
    serde_json::from_value(value).unwrap()
}

fn run(
    input: WorkflowGraphRunRequest,
    artifact_projection: WorkflowArtifactProjection,
) -> Result<WorkflowGraphRunResult, String> {
    run_workflow_graph_with_options(
        input,
        WorkflowGraphRunOptions {
            artifact_projection,
        },
    )
}

#[test]
fn workflow_selected_input_priority_and_lineage_name_only_the_selected_source() {
    for projection in [
        WorkflowArtifactProjection::All,
        WorkflowArtifactProjection::Outputs,
        WorkflowArtifactProjection::None,
    ] {
        for reverse_nodes in [false, true] {
            for reverse_edges in [false, true] {
                let expected = if reverse_edges { "backup" } else { "primary" };
                let result = run(
                    request(
                        json!({"source":"primary"}),
                        json!({"source":"backup"}),
                        reverse_nodes,
                        reverse_edges,
                    ),
                    projection,
                )
                .unwrap();
                let key = format!("{expected}.payload");
                let trace = result
                    .node_runs
                    .iter()
                    .find(|node| node.node_id == "join")
                    .unwrap();
                assert_eq!(trace.consumed_artifacts, [key.clone()]);
                let lineage = result
                    .artifact_lineage
                    .iter()
                    .find(|entry| entry.node_id == "join")
                    .unwrap();
                assert_eq!(lineage.source_artifacts, [key]);
                assert!(
                    result
                        .completed_nodes
                        .iter()
                        .position(|node| node == "join")
                        .unwrap()
                        > result
                            .completed_nodes
                            .iter()
                            .position(|node| node == "backup")
                            .unwrap()
                );
                if projection != WorkflowArtifactProjection::None {
                    assert_eq!(result.artifacts["output.payload"]["source"], expected);
                    assert_eq!(result.artifacts["raw_primary.payload"]["source"], "primary");
                    assert_eq!(result.artifacts["raw_backup.payload"]["source"], "backup");
                }
            }
        }
    }
}

#[test]
fn workflow_selected_input_equal_false_and_null_values_do_not_erase_source_identity() {
    for value in [Value::Null, json!(false), json!(0), json!({"same":42})] {
        for reverse in [false, true] {
            let result = run(
                request(value.clone(), value.clone(), false, reverse),
                WorkflowArtifactProjection::Outputs,
            )
            .unwrap();
            let expected = if reverse {
                "backup.payload"
            } else {
                "primary.payload"
            };
            assert_eq!(result.artifacts["output.payload"], value);
            let trace = result
                .node_runs
                .iter()
                .find(|node| node.node_id == "join")
                .unwrap();
            assert_eq!(trace.consumed_artifacts, [expected]);
            let lineage = result
                .artifact_lineage
                .iter()
                .find(|entry| entry.node_id == "join")
                .unwrap();
            assert_eq!(lineage.source_artifacts, [expected]);
        }
    }
}

fn text_buffer(value: &Value) -> usize {
    value["text"].as_str().unwrap().as_ptr() as usize
}

#[test]
fn workflow_selected_input_unused_and_aliased_edges_keep_later_shared_consumers_alive() {
    for aliased in [false, true] {
        for ephemeral in [false, true] {
            for projection in [
                WorkflowArtifactProjection::All,
                WorkflowArtifactProjection::Outputs,
                WorkflowArtifactProjection::None,
            ] {
                let mut input = request(
                    json!({"text":"a".repeat(128*1024)}),
                    json!({"text":"b".repeat(128*1024)}),
                    false,
                    false,
                );
                input.graph.defaults.cache_policy = Some(if ephemeral {
                    WorkflowCachePolicy::Ephemeral
                } else {
                    WorkflowCachePolicy::Cached
                });
                if aliased {
                    input
                        .graph
                        .edges
                        .iter_mut()
                        .find(|edge| edge.id == "backup-join")
                        .unwrap()
                        .from
                        .node = "primary".into();
                }
                let primary = text_buffer(&input.input_artifacts["primary"]);
                let backup = text_buffer(&input.input_artifacts["backup"]);
                let result = run(input, projection).unwrap();
                let trace = result
                    .node_runs
                    .iter()
                    .find(|node| node.node_id == "join")
                    .unwrap();
                assert_eq!(trace.consumed_artifacts, ["primary.payload"]);
                if projection == WorkflowArtifactProjection::None {
                    assert!(result.artifacts.is_empty());
                } else {
                    assert_eq!(
                        result.artifacts["output.payload"],
                        result.artifacts["raw_primary.payload"]
                    );
                    assert_eq!(
                        result.artifacts["raw_backup.payload"]["text"],
                        "b".repeat(128 * 1024)
                    );
                    if ephemeral || projection == WorkflowArtifactProjection::Outputs {
                        assert_eq!(result.artifacts.len(), 3);
                        assert_eq!(
                            text_buffer(&result.artifacts["raw_primary.payload"]),
                            primary
                        );
                        assert_eq!(text_buffer(&result.artifacts["raw_backup.payload"]), backup);
                    }
                }
            }
        }
    }
}

#[test]
fn workflow_selected_input_failed_selection_does_not_retry_a_different_present_source() {
    for projection in [
        WorkflowArtifactProjection::All,
        WorkflowArtifactProjection::Outputs,
        WorkflowArtifactProjection::None,
    ] {
        let mut input = request(
            json!({"text":"x".repeat(500_001)}),
            json!({"healthy":true}),
            false,
            false,
        );
        input
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.id == "join")
            .unwrap()
            .config = Some(json!({"on_error":"skip"}));
        let result = run(input, projection).unwrap();
        assert_eq!(result.failed_nodes, ["join"]);
        assert_eq!(result.skipped_nodes, ["output"]);
        let failed = result
            .node_runs
            .iter()
            .find(|node| node.node_id == "join")
            .unwrap();
        assert_eq!(failed.consumed_artifacts, ["primary.payload"]);
        assert!(failed.produced_artifacts.is_empty());
        assert!(
            failed
                .error_message
                .as_deref()
                .unwrap()
                .contains("length security budget")
        );
        assert!(
            !result
                .artifact_lineage
                .iter()
                .any(|entry| entry.node_id == "join")
        );
        assert!(!result.artifacts.contains_key("output.payload"));
        if projection != WorkflowArtifactProjection::None {
            assert_eq!(
                result.artifacts["raw_backup.payload"],
                json!({"healthy":true})
            );
            assert_eq!(
                result.artifacts["raw_primary.payload"]["text"]
                    .as_str()
                    .unwrap()
                    .len(),
                500_001
            );
        }
        let clean = run(
            request(
                json!({"healthy":true}),
                json!({"text":"x".repeat(500_001)}),
                false,
                false,
            ),
            projection,
        )
        .unwrap();
        assert!(clean.failed_nodes.is_empty());
        if projection != WorkflowArtifactProjection::None {
            assert_eq!(clean.artifacts["output.payload"], json!({"healthy":true}));
        }
    }
}

#[test]
fn workflow_selected_input_nested_joins_record_direct_selected_edges_without_flattening() {
    let mut input = request(
        json!({"source":"primary"}),
        json!({"source":"backup"}),
        false,
        false,
    );
    let mut second = input
        .graph
        .nodes
        .iter()
        .find(|node| node.id == "join")
        .unwrap()
        .clone();
    second.id = "second".into();
    input.graph.nodes.push(second);
    input
        .graph
        .edges
        .iter_mut()
        .find(|edge| edge.to.node == "output")
        .unwrap()
        .from
        .node = "second".into();
    let mut left = input
        .graph
        .edges
        .iter()
        .find(|edge| edge.id == "primary-join")
        .unwrap()
        .clone();
    left.id = "join-second".into();
    left.from.node = "join".into();
    left.to.node = "second".into();
    input.graph.edges.push(left);
    let mut right = input
        .graph
        .edges
        .iter()
        .find(|edge| edge.id == "backup-join")
        .unwrap()
        .clone();
    right.id = "backup-second".into();
    right.to.node = "second".into();
    input.graph.edges.push(right);
    let result = run(input, WorkflowArtifactProjection::Outputs).unwrap();
    for (node, expected) in [("join", "primary.payload"), ("second", "join.payload")] {
        let trace = result
            .node_runs
            .iter()
            .find(|entry| entry.node_id == node)
            .unwrap();
        let lineage = result
            .artifact_lineage
            .iter()
            .find(|entry| entry.node_id == node)
            .unwrap();
        assert_eq!(trace.consumed_artifacts, [expected]);
        assert_eq!(lineage.source_artifacts, [expected]);
    }
    assert_eq!(result.artifacts["output.payload"]["source"], "primary");
}

#[test]
fn workflow_selected_input_output_metadata_cannot_enable_partial_readiness() {
    for projection in [
        WorkflowArtifactProjection::All,
        WorkflowArtifactProjection::Outputs,
        WorkflowArtifactProjection::None,
    ] {
        let mut input = request(
            json!({"source":"primary"}),
            json!({"source":"backup"}),
            false,
            false,
        );
        input.input_artifacts.remove("backup");
        input
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.id == "backup")
            .unwrap()
            .config = Some(json!({"on_error":"skip"}));
        input.graph.nodes.retain(|node| {
            node.id != "join" && node.id != "raw_primary" && node.id != "raw_backup"
        });
        input.graph.edges.retain(|edge| edge.to.node == "join");
        for edge in &mut input.graph.edges {
            edge.to.node = "output".into();
        }
        input.graph.output_nodes = vec!["output".into()];
        let output = input
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.id == "output")
            .unwrap();
        output.operator_id = Some("transform.first_available".into());
        let mut second = output.inputs[0].clone();
        second.id = "right".into();
        output.inputs[0].id = "left".into();
        output.inputs.push(second);
        let mut complete = input.clone();
        complete
            .input_artifacts
            .insert("backup".into(), json!({"source":"backup"}));
        let mut plain = complete.clone();
        plain
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.id == "output")
            .unwrap()
            .operator_id = None;
        let complete = run(complete, projection).unwrap();
        let plain = run(plain, projection).unwrap();
        assert!(complete.failed_nodes.is_empty() && complete.skipped_nodes.is_empty());
        assert_eq!(complete.artifacts, plain.artifacts);
        assert_eq!(complete.artifact_lineage, plain.artifact_lineage);
        let trace = complete
            .node_runs
            .iter()
            .find(|node| node.node_id == "output")
            .unwrap();
        assert_eq!(
            trace.consumed_artifacts,
            ["primary.payload", "backup.payload"]
        );
        for (port, source) in [("left", "primary.payload"), ("right", "backup.payload")] {
            let lineage = complete
                .artifact_lineage
                .iter()
                .find(|entry| entry.node_id == "output" && entry.port_id == port)
                .unwrap();
            assert_eq!(lineage.source_artifacts, [source]);
        }
        let result = run(input, projection).unwrap();
        assert_eq!(result.failed_nodes, ["backup"]);
        assert_eq!(result.skipped_nodes, ["output"]);
        assert!(
            !result
                .artifacts
                .keys()
                .any(|key| key.starts_with("output."))
        );
        assert!(
            !result
                .artifact_lineage
                .iter()
                .any(|entry| entry.node_id == "output")
        );
    }
    let mut empty = request(Value::Null, Value::Null, false, false);
    empty.graph.nodes.retain(|node| node.id == "output");
    empty.graph.nodes[0].operator_id = Some("transform.first_available".into());
    empty.graph.edges.clear();
    empty.graph.entry_nodes.clear();
    empty.graph.output_nodes = vec!["output".into()];
    empty.input_artifacts.clear();
    let result = run(empty, WorkflowArtifactProjection::All).unwrap();
    assert_eq!(result.completed_nodes, ["output"]);
    assert!(result.skipped_nodes.is_empty() && result.failed_nodes.is_empty());
    assert!(result.artifacts.is_empty());
}
