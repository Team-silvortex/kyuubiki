use kyuubiki_headless_sdk::{
    SdkError, WorkflowGraphDefinition, build_workflow_output_manifest,
    normalize_workflow_progression, normalize_workflow_runtime,
    validate_workflow_result_against_graph,
};

#[test]
fn builds_output_manifest_from_graph() {
    let graph: WorkflowGraphDefinition = serde_json::from_str(include_str!(
        "../../../schemas/examples.workflow-graph.json"
    ))
    .expect("graph example");
    let manifest = build_workflow_output_manifest(&graph).expect("manifest");
    assert_eq!(manifest.graph_id, "workflow.heat-to-thermo-quad-2d");
    assert_eq!(manifest.outputs[0].key, "thermo_summary.result");
}

#[test]
fn validates_result_payload_with_artifact_type_fallback() {
    let graph: WorkflowGraphDefinition = serde_json::from_str(include_str!(
        "../../../schemas/examples.workflow-graph.json"
    ))
    .expect("graph example");
    let payload = serde_json::json!({
        "result": {
            "workflow_id": "workflow.demo",
            "run_id": "run-1",
            "status": "completed",
            "current_node": "solve_thermo",
            "completed_nodes": ["input", "solve_thermo"],
            "progress_events": [{"node_id": "solve_thermo", "status": "completed"}],
            "artifacts": {
                "result/thermal_plane_quad_2d": {
                    "artifact_id": "artifact.thermo.result",
                    "artifact_type": "result/thermal_plane_quad_2d",
                    "dataset_value": "thermo_result"
                }
            }
        }
    });
    let validated =
        validate_workflow_result_against_graph(&graph, &payload).expect("validated payload");
    assert_eq!(
        validated.artifacts["thermo_summary.result"]["artifact_id"].as_str(),
        Some("artifact.thermo.result")
    );
    assert_eq!(validated.workflow_runtime.run_id.as_deref(), Some("run-1"));
}

#[test]
fn rejects_missing_required_output_artifact() {
    let graph: WorkflowGraphDefinition = serde_json::from_str(include_str!(
        "../../../schemas/examples.workflow-graph.json"
    ))
    .expect("graph example");
    let payload = serde_json::json!({"result": {"artifacts": {}}});
    match validate_workflow_result_against_graph(&graph, &payload) {
        Err(SdkError::Validation { errors }) => {
            assert!(
                errors
                    .iter()
                    .any(|item| item.contains("thermo_summary.result"))
            );
        }
        other => panic!("expected validation error, got {other:?}"),
    }
}

#[test]
fn normalizes_workflow_runtime() {
    let payload = serde_json::json!({
        "result": {
            "workflow_id": "workflow.demo",
            "run_id": "run-1",
            "status": "running",
            "current_node": "solve",
            "completed_nodes": ["input"],
            "progress_events": [{"node_id": "input", "status": "completed"}]
        }
    });
    let runtime = normalize_workflow_runtime(&payload).expect("runtime");
    assert_eq!(runtime.current_node.as_deref(), Some("solve"));
}

#[test]
fn allows_null_current_node_in_workflow_runtime() {
    let payload = serde_json::json!({
        "result": {
            "workflow_id": "workflow.demo",
            "run_id": "run-null-current-node",
            "status": "completed",
            "current_node": null,
            "completed_nodes": ["input", "solve"],
            "progress_events": [{"node_id": "solve", "status": "completed"}]
        }
    });
    let runtime = normalize_workflow_runtime(&payload).expect("runtime");
    assert_eq!(runtime.current_node, None);
}

#[test]
fn allows_omitted_graph_ports_and_null_progress_current_node() {
    let graph: WorkflowGraphDefinition = serde_json::from_value(serde_json::json!({
        "schema_version": "kyuubiki.workflow-graph/v1",
        "id": "workflow.optional-ports",
        "name": "Optional ports",
        "version": "1.0.0",
        "entry_nodes": ["solve"],
        "output_nodes": ["solve"],
        "nodes": [{"id": "solve", "kind": "solve"}],
        "edges": []
    }))
    .expect("ports default to empty lists");
    assert!(graph.nodes[0].inputs.is_empty());
    assert!(graph.nodes[0].outputs.is_empty());

    let progression = normalize_workflow_progression(
        &[serde_json::json!({
            "job": {
                "job_id": "job-complete",
                "status": "completed",
                "current_node": null,
                "completed_nodes": ["solve"],
                "progress_events": []
            }
        })],
        None,
    )
    .expect("completed history accepts a null current node");
    assert_eq!(progression.snapshots[0].current_node, None);
}

#[test]
fn normalizes_workflow_progression() {
    let history = vec![serde_json::json!({
        "job": {
            "job_id": "job-1",
            "status": "running",
            "progress": 0.5,
            "current_node": "solve",
            "completed_nodes": ["input"],
            "progress_events": [{"node_id": "input", "status": "completed"}]
        }
    })];
    let result_payload = serde_json::json!({
        "result": {
            "workflow_id": "workflow.demo",
            "run_id": "run-1",
            "status": "completed",
            "current_node": "output",
            "completed_nodes": ["input", "solve", "output"],
            "progress_events": [{"node_id": "output", "status": "completed"}],
            "artifacts": {}
        }
    });
    let progression =
        normalize_workflow_progression(&history, Some(&result_payload)).expect("progression");
    assert_eq!(
        progression.snapshots[0].current_node.as_deref(),
        Some("solve")
    );
    assert_eq!(
        progression
            .latest
            .as_ref()
            .and_then(|value| value["run_id"].as_str()),
        Some("run-1")
    );
}
