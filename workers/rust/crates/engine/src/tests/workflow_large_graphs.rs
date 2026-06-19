use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;
use std::time::Instant;

#[test]
fn runs_large_heat_to_thermo_workflow_graph_at_128_nodes() {
    run_large_chain_case(128);
}

#[test]
fn runs_large_heat_to_thermo_workflow_graph_at_256_nodes() {
    run_large_chain_case(256);
}

#[test]
fn runs_large_heat_to_thermo_workflow_graph_at_512_nodes() {
    run_large_chain_case(512);
}

#[test]
fn runs_large_heat_to_thermo_workflow_graph_at_1024_nodes() {
    run_large_chain_case(1024);
}

fn run_large_chain_case(pass_through_count: usize) {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: format!("workflow.large-heat-to-thermo-chain-{pass_through_count}"),
        name: "Large heat to thermo chain".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Stress-test workflow scheduling with a long real operator chain".to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec!["heat_model".to_string()],
        output_nodes: vec!["thermo_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: build_nodes(pass_through_count),
        edges: build_edges(pass_through_count),
    };

    let started_at = Instant::now();
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([("heat_model".to_string(), heat_model_input())]),
    })
    .expect("large workflow graph should run");
    let elapsed = started_at.elapsed();
    eprintln!(
        "workflow_large_graphs[rust]: pass_through_count={pass_through_count} completed_nodes={} elapsed_ms={:.3}",
        run.completed_nodes.len(),
        elapsed.as_secs_f64() * 1000.0
    );

    assert_eq!(
        run.workflow_id,
        format!("workflow.large-heat-to-thermo-chain-{pass_through_count}")
    );
    assert_eq!(run.completed_nodes.len(), pass_through_count + 5);
    assert_eq!(run.skipped_nodes.len(), 0);
    assert_eq!(
        run.completed_nodes.first().map(String::as_str),
        Some("heat_model")
    );
    assert_eq!(
        run.completed_nodes.last().map(String::as_str),
        Some("thermo_output")
    );

    let tail_key = format!("pass_{:03}.result", pass_through_count - 1);
    let bridge_payload = run
        .artifacts
        .get(&tail_key)
        .expect("tail pass-through artifact should exist");
    assert_eq!(bridge_payload["max_temperature"].as_f64(), Some(100.0));

    let thermo_result = run
        .artifacts
        .get("thermo_output.result")
        .expect("thermo output should exist");
    assert!(thermo_result["max_stress"].as_f64().unwrap_or_default() > 0.0);
    assert!(run.progress_events.len() >= run.completed_nodes.len());
    assert!(elapsed.as_secs_f64() < 30.0);
}

fn build_nodes(pass_through_count: usize) -> Vec<WorkflowNode> {
    let mut nodes = vec![
        input_node("heat_model", "model", "study_model/heat_plane_quad_2d"),
        solve_node(
            "solve_heat",
            "solve.heat_plane_quad_2d",
            "model",
            "study_model/heat_plane_quad_2d",
            "result",
            "result/heat_plane_quad_2d",
        ),
    ];

    for index in 0..pass_through_count {
        nodes.push(pass_through_node(&format!("pass_{index:03}")));
    }

    nodes.push(WorkflowNode {
        id: "bridge_temperature".to_string(),
        kind: WorkflowNodeKind::Transform,
        operator_id: Some("bridge.temperature_field_to_thermo_quad_2d".to_string()),
        name: None,
        description: None,
        config: Some(serde_json::json!({
            "seed_model": thermo_seed_model(),
            "contract": {
                "version": "kyuubiki.bridge-contract/v1",
                "source": { "field": "temperature" },
                "transform": { "scale": 1.0, "default_value": 0.0 },
                "target": { "field": "temperature_delta" }
            }
        })),
        cache_policy: None,
        inputs: vec![port("heat_result", "result/heat_plane_quad_2d")],
        outputs: vec![port("thermo_model", "study_model/thermal_plane_quad_2d")],
    });
    nodes.push(solve_node(
        "solve_thermo",
        "solve.thermal_plane_quad_2d",
        "model",
        "study_model/thermal_plane_quad_2d",
        "result",
        "result/thermal_plane_quad_2d",
    ));
    nodes.push(output_node(
        "thermo_output",
        "result",
        "result/thermal_plane_quad_2d",
    ));
    nodes
}

fn build_edges(pass_through_count: usize) -> Vec<WorkflowEdge> {
    let mut edges = vec![edge(
        "edge-heat-input",
        "heat_model",
        "model",
        "solve_heat",
        "model",
        "study_model/heat_plane_quad_2d",
    )];

    for index in 0..pass_through_count {
        let from_node = if index == 0 {
            "solve_heat".to_string()
        } else {
            format!("pass_{:03}", index - 1)
        };
        let from_port = "result";
        let to_node = format!("pass_{index:03}");
        edges.push(edge(
            &format!("edge-pass-{index:03}"),
            &from_node,
            from_port,
            &to_node,
            "input",
            "result/heat_plane_quad_2d",
        ));
    }

    edges.push(edge(
        "edge-tail-to-bridge",
        &format!("pass_{:03}", pass_through_count - 1),
        "result",
        "bridge_temperature",
        "heat_result",
        "result/heat_plane_quad_2d",
    ));
    edges.push(edge(
        "edge-bridge-to-thermo",
        "bridge_temperature",
        "thermo_model",
        "solve_thermo",
        "model",
        "study_model/thermal_plane_quad_2d",
    ));
    edges.push(edge(
        "edge-thermo-output",
        "solve_thermo",
        "result",
        "thermo_output",
        "result",
        "result/thermal_plane_quad_2d",
    ));
    edges
}

fn input_node(id: &str, output_id: &str, artifact_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port(output_id, artifact_type)],
    }
}

fn solve_node(
    id: &str,
    operator_id: &str,
    input_id: &str,
    input_type: &str,
    output_id: &str,
    output_type: &str,
) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Solve,
        operator_id: Some(operator_id.to_string()),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port(input_id, input_type)],
        outputs: vec![port(output_id, output_type)],
    }
}

fn pass_through_node(id: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Transform,
        operator_id: Some("transform.first_available".to_string()),
        name: None,
        description: None,
        config: Some(serde_json::json!({})),
        cache_policy: None,
        inputs: vec![port("input", "result/heat_plane_quad_2d")],
        outputs: vec![port("result", "result/heat_plane_quad_2d")],
    }
}

fn output_node(id: &str, input_id: &str, artifact_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port(input_id, artifact_type)],
        outputs: vec![],
    }
}

fn port(id: &str, artifact_type: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: artifact_type.to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: None,
    }
}

fn edge(
    id: &str,
    from_node: &str,
    from_port: &str,
    to_node: &str,
    to_port: &str,
    artifact_type: &str,
) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        from: WorkflowNodePortRef {
            node: from_node.to_string(),
            port: from_port.to_string(),
        },
        to: WorkflowNodePortRef {
            node: to_node.to_string(),
            port: to_port.to_string(),
        },
        artifact_type: artifact_type.to_string(),
        dataset_value: None,
    }
}

fn heat_model_input() -> serde_json::Value {
    serde_json::json!({
        "nodes": [
            { "id": "h0", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 0.0 },
            { "id": "h1", "x": 1.0, "y": 0.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 0.0 },
            { "id": "h2", "x": 1.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 },
            { "id": "h3", "x": 0.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 }
        ],
        "elements": [
            { "id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 45.0 }
        ]
    })
}

fn thermo_seed_model() -> serde_json::Value {
    serde_json::json!({
        "nodes": [
            { "id": "t0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
            { "id": "t1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
            { "id": "t2", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
            { "id": "t3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 }
        ],
        "elements": [
            { "id": "tq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70_000_000_000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }
        ]
    })
}
