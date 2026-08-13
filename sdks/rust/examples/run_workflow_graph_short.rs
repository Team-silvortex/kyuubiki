use std::env;
use std::time::Duration;

use kyuubiki_headless_sdk::{KyuubikiAgentClient, KyuubikiAuth, KyuubikiSession, SdkResult};

fn short_graph() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "kyuubiki.workflow-graph/v1",
        "id": "workflow.heat-to-summary-short",
        "name": "Heat Solve + Summary",
        "version": "1.0.0",
        "description": "Minimal heat pipeline for include_result comparison.",
        "entry_nodes": ["heat_model"],
        "output_nodes": ["heat_summary_output"],
        "nodes": [
            {
                "id": "heat_model",
                "kind": "input",
                "inputs": [],
                "outputs": [
                    {"id": "model", "artifact_type": "study_model/heat_plane_quad_2d"}
                ]
            },
            {
                "id": "solve_heat",
                "kind": "solve",
                "operator_id": "solve.heat_plane_quad_2d",
                "inputs": [
                    {"id": "model", "artifact_type": "study_model/heat_plane_quad_2d"}
                ],
                "outputs": [
                    {"id": "result", "artifact_type": "result/heat_plane_quad_2d"}
                ]
            },
            {
                "id": "heat_summary",
                "kind": "extract",
                "operator_id": "extract.result_summary",
                "config": {"fields": ["max_temperature", "max_heat_flux"]},
                "inputs": [
                    {"id": "result", "artifact_type": "result/heat_plane_quad_2d"}
                ],
                "outputs": [
                    {"id": "summary", "artifact_type": "extract/result_summary"}
                ]
            },
            {
                "id": "heat_summary_output",
                "kind": "output",
                "inputs": [
                    {"id": "summary", "artifact_type": "extract/result_summary"}
                ],
                "outputs": []
            }
        ],
        "edges": [
            {
                "id": "e1",
                "from": {"node": "heat_model", "port": "model"},
                "to": {"node": "solve_heat", "port": "model"},
                "artifact_type": "study_model/heat_plane_quad_2d"
            },
            {
                "id": "e2",
                "from": {"node": "solve_heat", "port": "result"},
                "to": {"node": "heat_summary", "port": "result"},
                "artifact_type": "result/heat_plane_quad_2d"
            },
            {
                "id": "e3",
                "from": {"node": "heat_summary", "port": "summary"},
                "to": {"node": "heat_summary_output", "port": "summary"},
                "artifact_type": "extract/result_summary"
            }
        ]
    })
}

fn short_input_artifacts() -> serde_json::Value {
    serde_json::json!({
        "heat_model": {
            "nodes": [
                {"id": "h0", "x": 0, "y": 0, "fix_temperature": true, "temperature": 100, "heat_load": 0},
                {"id": "h1", "x": 1, "y": 0, "fix_temperature": false, "temperature": 0, "heat_load": 0},
                {"id": "h2", "x": 1, "y": 1, "fix_temperature": true, "temperature": 20, "heat_load": 0},
                {"id": "h3", "x": 0, "y": 1, "fix_temperature": true, "temperature": 20, "heat_load": 0}
            ],
            "elements": [
                {"id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 45}
            ]
        }
    })
}

fn main() -> SdkResult<()> {
    let base_url = env::var("KYUUBIKI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".into());
    let include_result = env::var("INCLUDE_RESULT").as_deref() == Ok("1");
    let auth = env::var("KYUUBIKI_TOKEN")
        .ok()
        .map(KyuubikiAuth::access_token);

    let session = KyuubikiSession::from_control_plane_with_auth(&base_url, auth)?;
    let agent = KyuubikiAgentClient::new(session);

    let graph = short_graph();
    let input_artifacts = short_input_artifacts();
    println!(
        "[workflow] id: {}",
        graph["id"].as_str().unwrap_or("unknown")
    );
    println!(
        "[run] include_result={} poll=500ms timeout=300s",
        include_result
    );

    let outcome = agent.run_workflow_graph(
        &graph,
        &input_artifacts,
        Duration::from_millis(500),
        Duration::from_secs(300),
        include_result,
    )?;

    let terminal_job = outcome.terminal.get("job").and_then(|j| j.as_object());
    let terminal_job_id = terminal_job
        .and_then(|job| job.get("id"))
        .and_then(|id| id.as_str())
        .or_else(|| {
            terminal_job
                .and_then(|job| job.get("job_id"))
                .and_then(|id| id.as_str())
        })
        .unwrap_or("unknown");
    let terminal_status = terminal_job
        .and_then(|job| job.get("status").and_then(|value| value.as_str()))
        .unwrap_or("unknown");
    println!("[result] terminal job_id: {terminal_job_id}");
    println!("[result] terminal status: {terminal_status}");
    println!("[result] history events: {}", outcome.history.len());

    if let Some(runtime) = outcome.workflow_runtime {
        println!(
            "[result] runtime.current_node={}",
            runtime.current_node.as_deref().unwrap_or("null")
        );
        println!(
            "[result] runtime.status={:?}",
            runtime.status.as_deref().unwrap_or("null")
        );
        println!("[result] completed_nodes: {:?}", runtime.completed_nodes);
    }

    if let Some(v) = outcome.output_manifest {
        println!(
            "[result] manifest outputs: {:?}",
            v.outputs
                .iter()
                .map(|output| format!("{}.{}", output.node_id, output.port_id))
                .collect::<Vec<_>>()
        );
    }

    if let Some(validated) = outcome.validated_outputs {
        println!(
            "[result] validated outputs: {:?}",
            validated
                .manifest
                .outputs
                .iter()
                .map(|output| output.key.clone())
                .collect::<Vec<_>>()
        );
        if let Some(nodes) = validated
            .artifacts
            .get("heat_summary_output.summary")
            .and_then(|artifact| artifact.get("nodes").and_then(|n| n.as_array()))
        {
            println!("[artifact] summary nodes: {}", nodes.len());
        }
    }

    if let Some(result) = outcome.result {
        println!(
            "[result] result keys: {:?}",
            result
                .as_object()
                .map(|o| o.keys().collect::<Vec<_>>())
                .unwrap_or_default()
        );
        if let Some(artifacts) = result.get("artifacts").and_then(|v| v.as_object()) {
            println!(
                "[result] artifact keys: {:?}",
                artifacts.keys().collect::<Vec<_>>()
            );
        }
    }

    Ok(())
}
