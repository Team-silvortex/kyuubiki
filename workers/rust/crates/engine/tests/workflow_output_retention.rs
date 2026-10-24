use kyuubiki_engine::run_workflow_graph_with_options;
use kyuubiki_protocol::{
    WorkflowArtifactProjection, WorkflowCachePolicy, WorkflowGraphRunOptions,
    WorkflowGraphRunRequest, WorkflowGraphRunResult,
};
use serde_json::{Value, json};

const PROJECTIONS: [WorkflowArtifactProjection; 3] = [
    WorkflowArtifactProjection::All,
    WorkflowArtifactProjection::Outputs,
    WorkflowArtifactProjection::None,
];

fn request(explicit_output_ports: bool) -> WorkflowGraphRunRequest {
    serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1", "id":"workflow.output-retention",
            "name":"Output retention", "version":"1.0.0", "entry_nodes":["source"],
            "output_nodes":["published"],
            "nodes":[
                {"id":"source", "kind":"input", "inputs":[], "outputs":[port("payload")]},
                {"id":"published", "kind":"output", "inputs":[port("payload")],
                 "outputs":if explicit_output_ports {vec![port("payload")]} else {vec![]}}
            ],
            "edges":[edge("source-published", "source", "published", "payload")]
        },
        "input_artifacts":{"source":{"text":"x".repeat(128*1024), "items":[1,2,3]}}
    }))
    .unwrap()
}

fn port(id: &str) -> Value {
    json!({"id":id, "artifact_type":"artifact/json"})
}

fn edge(id: &str, from: &str, to: &str, target: &str) -> Value {
    json!({"id":id, "artifact_type":"artifact/json",
        "from":{"node":from,"port":"payload"}, "to":{"node":to,"port":target}})
}

fn downstream(request: &mut WorkflowGraphRunRequest) {
    request.graph.nodes.extend([
        serde_json::from_value(json!({
            "id":"pass", "kind":"transform", "operator_id":"transform.first_available",
            "inputs":[port("payload")], "outputs":[port("payload")]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "id":"final", "kind":"output", "inputs":[port("payload")], "outputs":[]
        }))
        .unwrap(),
    ]);
    request.graph.edges.extend([
        serde_json::from_value(edge("published-pass", "published", "pass", "payload")).unwrap(),
        serde_json::from_value(edge("pass-final", "pass", "final", "payload")).unwrap(),
    ]);
    request.graph.output_nodes.push("final".into());
}

fn policies(
    request: &mut WorkflowGraphRunRequest,
    default: WorkflowCachePolicy,
    output: Option<WorkflowCachePolicy>,
    reverse: bool,
) {
    request.graph.defaults.cache_policy = Some(default);
    request
        .graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "published")
        .unwrap()
        .cache_policy = output;
    if reverse {
        request.graph.nodes.reverse();
        request.graph.edges.reverse();
    }
}

fn run(
    request: WorkflowGraphRunRequest,
    projection: WorkflowArtifactProjection,
) -> WorkflowGraphRunResult {
    run_workflow_graph_with_options(
        request,
        WorkflowGraphRunOptions {
            artifact_projection: projection,
        },
    )
    .unwrap()
}

fn same_evidence(left: &WorkflowGraphRunResult, right: &WorkflowGraphRunResult) {
    assert_eq!(left.completed_nodes, right.completed_nodes);
    assert_eq!(left.skipped_nodes, right.skipped_nodes);
    assert_eq!(left.failed_nodes, right.failed_nodes);
    assert_eq!(left.node_runs, right.node_runs);
    assert_eq!(left.artifact_lineage, right.artifact_lineage);
    assert_eq!(left.branch_decisions, right.branch_decisions);
}

fn buffer(value: &Value) -> usize {
    value["text"].as_str().unwrap().as_ptr() as usize
}

#[test]
fn workflow_output_retention_explicit_terminal_ports_do_not_change_projected_results() {
    for default in [WorkflowCachePolicy::Cached, WorkflowCachePolicy::Ephemeral] {
        for output in [
            None,
            Some(WorkflowCachePolicy::Cached),
            Some(WorkflowCachePolicy::Ephemeral),
        ] {
            for reverse in [false, true] {
                let mut reference = request(false);
                policies(&mut reference, default, output, reverse);
                let reference = run(reference, WorkflowArtifactProjection::All);
                for projection in PROJECTIONS {
                    let mut input = request(true);
                    policies(&mut input, default, output, reverse);
                    let result = run(input, projection);
                    same_evidence(&result, &reference);
                    if projection == WorkflowArtifactProjection::None {
                        assert!(result.artifacts.is_empty());
                    } else {
                        assert_eq!(
                            result.artifacts["published.payload"],
                            reference.artifacts["published.payload"]
                        );
                        if projection == WorkflowArtifactProjection::Outputs {
                            assert_eq!(result.artifacts.len(), 1);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn workflow_output_retention_declared_outputs_survive_downstream_consumption() {
    for default in [WorkflowCachePolicy::Cached, WorkflowCachePolicy::Ephemeral] {
        for output in [
            None,
            Some(WorkflowCachePolicy::Cached),
            Some(WorkflowCachePolicy::Ephemeral),
        ] {
            for reverse in [false, true] {
                let mut reference = request(true);
                downstream(&mut reference);
                policies(&mut reference, default, output, reverse);
                let reference = run(reference, WorkflowArtifactProjection::All);
                for projection in PROJECTIONS {
                    let mut input = request(true);
                    downstream(&mut input);
                    policies(&mut input, default, output, reverse);
                    let original = buffer(&input.input_artifacts["source"]);
                    let mut result = run(input, projection);
                    same_evidence(&result, &reference);
                    if projection == WorkflowArtifactProjection::None {
                        assert!(result.artifacts.is_empty());
                        continue;
                    }
                    assert_eq!(
                        result.artifacts["published.payload"],
                        result.artifacts["final.payload"]
                    );
                    assert_ne!(
                        buffer(&result.artifacts["published.payload"]),
                        buffer(&result.artifacts["final.payload"])
                    );
                    if projection == WorkflowArtifactProjection::Outputs {
                        assert_eq!(result.artifacts.len(), 2);
                        assert_eq!(buffer(&result.artifacts["published.payload"]), original);
                    }
                    result.artifacts.get_mut("final.payload").unwrap()["items"][0] = json!(99);
                    assert_eq!(
                        result.artifacts["published.payload"]["items"],
                        json!([1, 2, 3])
                    );
                }
            }
        }
    }
}

#[test]
fn workflow_output_retention_unlisted_outputs_still_release_and_forward_owned_data() {
    for default in [WorkflowCachePolicy::Cached, WorkflowCachePolicy::Ephemeral] {
        for reverse in [false, true] {
            let mut reference = request(true);
            downstream(&mut reference);
            reference.graph.output_nodes = vec!["final".into()];
            policies(
                &mut reference,
                default,
                Some(WorkflowCachePolicy::Cached),
                reverse,
            );
            let reference = run(reference, WorkflowArtifactProjection::All);
            for projection in PROJECTIONS {
                let mut input = request(true);
                downstream(&mut input);
                input.graph.output_nodes = vec!["final".into()];
                policies(
                    &mut input,
                    default,
                    Some(WorkflowCachePolicy::Cached),
                    reverse,
                );
                let original = buffer(&input.input_artifacts["source"]);
                let result = run(input, projection);
                same_evidence(&result, &reference);
                if projection == WorkflowArtifactProjection::None {
                    assert!(result.artifacts.is_empty());
                } else {
                    assert_eq!(
                        result.artifacts["final.payload"],
                        reference.artifacts["final.payload"]
                    );
                    if projection == WorkflowArtifactProjection::Outputs {
                        assert_eq!(result.artifacts.len(), 1);
                        assert_eq!(buffer(&result.artifacts["final.payload"]), original);
                    }
                }
            }
        }
    }
}

#[test]
fn workflow_output_retention_aliased_ports_keep_separate_values_and_direct_lineage() {
    for default in [WorkflowCachePolicy::Cached, WorkflowCachePolicy::Ephemeral] {
        for projection in PROJECTIONS {
            let mut input = request(true);
            policies(
                &mut input,
                default,
                Some(WorkflowCachePolicy::Ephemeral),
                false,
            );
            let output = input
                .graph
                .nodes
                .iter_mut()
                .find(|node| node.id == "published")
                .unwrap();
            output
                .inputs
                .push(serde_json::from_value(port("alias")).unwrap());
            output
                .outputs
                .push(serde_json::from_value(port("alias")).unwrap());
            input.graph.edges.push(
                serde_json::from_value(edge("source-alias", "source", "published", "alias"))
                    .unwrap(),
            );
            let mut result = run(input, projection);
            let trace = result
                .node_runs
                .iter()
                .find(|node| node.node_id == "published")
                .unwrap();
            assert_eq!(
                trace.consumed_artifacts,
                ["source.payload", "source.payload"]
            );
            assert_eq!(
                trace.produced_artifacts,
                ["published.payload", "published.alias"]
            );
            let lineage = result
                .artifact_lineage
                .iter()
                .filter(|entry| entry.node_id == "published")
                .collect::<Vec<_>>();
            assert_eq!(lineage.len(), 2);
            assert!(
                lineage
                    .iter()
                    .all(|entry| entry.source_artifacts == ["source.payload"])
            );
            if projection == WorkflowArtifactProjection::None {
                assert!(result.artifacts.is_empty());
            } else {
                assert_eq!(
                    result.artifacts["published.payload"],
                    result.artifacts["published.alias"]
                );
                if projection == WorkflowArtifactProjection::Outputs {
                    assert_eq!(result.artifacts.len(), 2);
                }
                result.artifacts.get_mut("published.alias").unwrap()["items"][0] = json!(99);
                assert_eq!(
                    result.artifacts["published.payload"]["items"],
                    json!([1, 2, 3])
                );
            }
        }
    }
}

#[test]
fn workflow_output_retention_recovered_failure_cannot_discard_a_declared_raw_output() {
    for default in [WorkflowCachePolicy::Cached, WorkflowCachePolicy::Ephemeral] {
        for projection in PROJECTIONS {
            let mut input = request(true);
            downstream(&mut input);
            policies(
                &mut input,
                default,
                Some(WorkflowCachePolicy::Ephemeral),
                false,
            );
            input.input_artifacts.get_mut("source").unwrap()["text"] = json!("x".repeat(500_001));
            input
                .graph
                .nodes
                .iter_mut()
                .find(|node| node.id == "pass")
                .unwrap()
                .config = Some(json!({"on_error":"skip"}));
            let result = run(input, projection);
            assert_eq!(result.failed_nodes, ["pass"]);
            assert_eq!(result.skipped_nodes, ["final"]);
            assert!(!result.artifacts.contains_key("final.payload"));
            assert!(
                !result
                    .artifact_lineage
                    .iter()
                    .any(|entry| entry.node_id == "pass")
            );
            if projection == WorkflowArtifactProjection::None {
                assert!(result.artifacts.is_empty());
            } else {
                assert_eq!(
                    result.artifacts["published.payload"]["text"]
                        .as_str()
                        .unwrap()
                        .len(),
                    500_001
                );
                if projection == WorkflowArtifactProjection::Outputs {
                    assert_eq!(result.artifacts.len(), 1);
                }
            }
            let mut healthy = request(true);
            downstream(&mut healthy);
            policies(
                &mut healthy,
                default,
                Some(WorkflowCachePolicy::Ephemeral),
                false,
            );
            let recovered = run(healthy, projection);
            assert!(recovered.failed_nodes.is_empty());
            if projection != WorkflowArtifactProjection::None {
                assert_eq!(
                    recovered.artifacts["published.payload"],
                    recovered.artifacts["final.payload"]
                );
            }
        }
    }
}
