use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;
use std::time::Instant;

const PASS_COUNT: usize = 1024;

#[test]
fn runs_cached_scheduler_only_chain_at_1024_nodes() {
    run_scheduler_only_chain(false);
}

#[test]
fn runs_ephemeral_scheduler_only_chain_at_1024_nodes() {
    run_scheduler_only_chain(true);
}

fn run_scheduler_only_chain(ephemeral_passes: bool) {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: format!("workflow.scheduler-only-{ephemeral_passes}"),
        name: "Scheduler-only identity chain".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Isolate workflow scheduling and artifact retention cost".to_string()),
        dataset_contract: None,
        entry_nodes: vec!["input".to_string()],
        output_nodes: vec!["output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(false),
        },
        nodes: build_nodes(ephemeral_passes),
        edges: build_edges(),
    };
    let started_at = Instant::now();
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "input".to_string(),
            serde_json::json!({"value": 42, "label": "scheduler-hot-path"}),
        )]),
    })
    .expect("scheduler-only chain should run");
    let elapsed = started_at.elapsed();

    eprintln!(
        "workflow_scheduler_hot_path[rust]: pass_count={PASS_COUNT} ephemeral_passes={ephemeral_passes} completed_nodes={} retained_artifacts={} elapsed_ms={:.3}",
        run.completed_nodes.len(),
        run.artifacts.len(),
        elapsed.as_secs_f64() * 1000.0
    );
    assert_eq!(run.completed_nodes.len(), PASS_COUNT + 2);
    assert_eq!(run.node_runs.len(), PASS_COUNT + 2);
    assert_eq!(run.artifact_lineage.len(), PASS_COUNT + 2);
    assert_eq!(run.skipped_nodes.len(), 0);
    assert_eq!(run.failed_nodes.len(), 0);
    assert_eq!(run.artifacts["output.result"]["value"], 42);
    assert_eq!(
        run.artifacts.len(),
        if ephemeral_passes { 2 } else { PASS_COUNT + 2 }
    );
    assert!(elapsed.as_secs_f64() < 30.0);
}

fn build_nodes(ephemeral_passes: bool) -> Vec<WorkflowNode> {
    let mut nodes = Vec::with_capacity(PASS_COUNT + 2);
    nodes.push(WorkflowNode {
        id: "input".to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("value")],
    });
    for index in 0..PASS_COUNT {
        nodes.push(WorkflowNode {
            id: format!("pass_{index:04}"),
            kind: WorkflowNodeKind::Transform,
            operator_id: Some("transform.first_available".to_string()),
            name: None,
            description: None,
            config: None,
            cache_policy: ephemeral_passes.then_some(WorkflowCachePolicy::Ephemeral),
            inputs: vec![port("input")],
            outputs: vec![port("result")],
        });
    }
    nodes.push(WorkflowNode {
        id: "output".to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("result")],
        outputs: vec![],
    });
    nodes
}

fn build_edges() -> Vec<WorkflowEdge> {
    let mut edges = Vec::with_capacity(PASS_COUNT + 1);
    for index in 0..PASS_COUNT {
        let source_node = if index == 0 {
            "input".to_string()
        } else {
            format!("pass_{:04}", index - 1)
        };
        let source_port = if index == 0 { "value" } else { "result" };
        edges.push(edge(
            &format!("edge_{index:04}"),
            &source_node,
            source_port,
            &format!("pass_{index:04}"),
            "input",
        ));
    }
    edges.push(edge(
        "edge_output",
        &format!("pass_{:04}", PASS_COUNT - 1),
        "result",
        "output",
        "result",
    ));
    edges
}

fn port(id: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: "artifact/json".to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: None,
    }
}

fn edge(
    id: &str,
    source: &str,
    source_port: &str,
    target: &str,
    target_port: &str,
) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        from: WorkflowNodePortRef {
            node: source.to_string(),
            port: source_port.to_string(),
        },
        to: WorkflowNodePortRef {
            node: target.to_string(),
            port: target_port.to_string(),
        },
        artifact_type: "artifact/json".to_string(),
        dataset_value: None,
    }
}
