use std::env;
use std::time::Duration;

use kyuubiki_headless_sdk::{KyuubikiAgentClient, KyuubikiAuth, KyuubikiSession, SdkResult};

fn main() -> SdkResult<()> {
    let base_url = env::var("KYUUBIKI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".into());
    let include_result = env::var("INCLUDE_RESULT").as_deref() == Ok("1");
    let auth = env::var("KYUUBIKI_TOKEN")
        .ok()
        .map(KyuubikiAuth::access_token);

    let session = KyuubikiSession::from_control_plane_with_auth(&base_url, auth)?;
    let agent = KyuubikiAgentClient::new(session);

    let graph = serde_json::json!({
        "schema_version": "kyuubiki.workflow-graph/v1",
        "id": "workflow.heat-to-thermo-quad-2d-coupled",
        "name": "Heat->Thermo Coupled + Heat Summary",
        "version": "1.0.0",
        "description": "Coupled heat/thermal workflow with branching summary extract",
        "entry_nodes": ["heat_model"],
        "output_nodes": ["heat_summary_output", "thermo_summary"],
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
                    {"id": "summary", "artifact_type": "report/summary"}
                ]
            },
            {
                "id": "heat_summary_output",
                "kind": "output",
                "inputs": [
                    {"id": "summary", "artifact_type": "report/summary"}
                ],
                "outputs": []
            },
            {
                "id": "bridge_temperature",
                "kind": "transform",
                "operator_id": "bridge.temperature_field_to_thermo_quad_2d",
                "config": {
                    "nodes": [
                        {"id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0},
                        {"id": "n1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0},
                        {"id": "n2", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0},
                        {"id": "n3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0}
                    ],
                    "elements": [
                        {"id": "tq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011}
                    ]
                },
                "inputs": [
                    {"id": "heat_result", "artifact_type": "result/heat_plane_quad_2d"}
                ],
                "outputs": [
                    {"id": "thermo_model", "artifact_type": "study_model/thermal_plane_quad_2d"}
                ]
            },
            {
                "id": "solve_thermo",
                "kind": "solve",
                "operator_id": "solve.thermal_plane_quad_2d",
                "inputs": [
                    {"id": "model", "artifact_type": "study_model/thermal_plane_quad_2d"}
                ],
                "outputs": [
                    {"id": "result", "artifact_type": "result/thermal_plane_quad_2d"}
                ]
            },
            {
                "id": "thermo_summary",
                "kind": "output",
                "inputs": [
                    {"id": "result", "artifact_type": "result/thermal_plane_quad_2d"}
                ],
                "outputs": []
            }
        ],
        "edges": [
            {
                "id": "edge-heat-input",
                "from": {"node": "heat_model", "port": "model"},
                "to": {"node": "solve_heat", "port": "model"},
                "artifact_type": "study_model/heat_plane_quad_2d"
            },
            {
                "id": "edge-heat-result-to-summary",
                "from": {"node": "solve_heat", "port": "result"},
                "to": {"node": "heat_summary", "port": "result"},
                "artifact_type": "result/heat_plane_quad_2d"
            },
            {
                "id": "edge-summary-output",
                "from": {"node": "heat_summary", "port": "summary"},
                "to": {"node": "heat_summary_output", "port": "summary"},
                "artifact_type": "report/summary"
            },
            {
                "id": "edge-heat-result-to-bridge",
                "from": {"node": "solve_heat", "port": "result"},
                "to": {"node": "bridge_temperature", "port": "heat_result"},
                "artifact_type": "result/heat_plane_quad_2d"
            },
            {
                "id": "edge-thermo-model",
                "from": {"node": "bridge_temperature", "port": "thermo_model"},
                "to": {"node": "solve_thermo", "port": "model"},
                "artifact_type": "study_model/thermal_plane_quad_2d"
            },
            {
                "id": "edge-thermo-result",
                "from": {"node": "solve_thermo", "port": "result"},
                "to": {"node": "thermo_summary", "port": "result"},
                "artifact_type": "result/thermal_plane_quad_2d"
            }
        ]
    });

    let input_artifacts = serde_json::json!({
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
    });

    println!(
        "[workflow] id: {}",
        graph["id"].as_str().unwrap_or("unknown")
    );
    println!("[run] include_result={include_result} poll_interval=500ms timeout=300s");

    match agent.run_workflow_graph(
        &graph,
        &input_artifacts,
        Duration::from_millis(500),
        Duration::from_secs(300),
        include_result,
    ) {
        Ok(outcome) => {
            let terminal_job = outcome.terminal.get("job").and_then(|j| j.as_object());
            let terminal_job_id = terminal_job
                .and_then(|job| job.get("id").and_then(|value| value.as_str()))
                .or_else(|| {
                    terminal_job.and_then(|job| job.get("job_id").and_then(|value| value.as_str()))
                })
                .unwrap_or("unknown");
            let terminal_status = terminal_job
                .and_then(|job| job.get("status").and_then(|value| value.as_str()))
                .unwrap_or("unknown");
            println!("[result] terminal job_id: {terminal_job_id}");
            println!("[result] terminal status: {terminal_status}");
            if let Some(runtime) = outcome.workflow_runtime {
                println!(
                    "[result] workflow_runtime.current_node: {}",
                    runtime.current_node.as_deref().unwrap_or("null")
                );
                println!(
                    "[result] workflow_runtime.completed_nodes: {:?}",
                    runtime.completed_nodes
                );
                println!("[result] workflow_runtime.status: {:?}", runtime.status);
            }
            println!("[result] history events: {}", outcome.history.len());
            if let Some(progression) = outcome.workflow_progression {
                println!(
                    "[result] progression snapshots: {}",
                    progression.snapshots.len()
                );
            }
            if include_result {
                if let Some(validated) = outcome.validated_outputs {
                    let keys: Vec<String> = validated
                        .manifest
                        .outputs
                        .iter()
                        .map(|output| output.key.clone())
                        .collect();
                    println!("[result] validated artifact keys: {:?}", keys);
                }
                if let Some(output_manifest) = outcome.output_manifest {
                    println!(
                        "[result] output manifest graph_id: {}, outputs: {:?}",
                        output_manifest.graph_id,
                        output_manifest
                            .outputs
                            .iter()
                            .map(|artifact| format!("{}.{}", artifact.node_id, artifact.port_id))
                            .collect::<Vec<_>>()
                    );
                }
                if let Some(ref result) = outcome.result {
                    println!(
                        "[result] result keys: {:?}",
                        result.as_object().map(|o| o.keys().collect::<Vec<_>>())
                    );
                }
            }
            let result_artifacts = outcome.result.as_ref().and_then(|result| {
                result
                    .get("artifacts")
                    .and_then(|artifacts| artifacts.as_object())
                    .or_else(|| {
                        result
                            .get("result")
                            .and_then(|result| result.get("artifacts"))
                            .and_then(|artifacts| artifacts.as_object())
                    })
            });
            if let Some(artifact_value) =
                result_artifacts.and_then(|artifacts| artifacts.get("heat_summary.summary"))
            {
                println!(
                    "[artifact] heat_summary.summary has keys: {:?}",
                    artifact_value
                        .as_object()
                        .map(|o| o.keys().collect::<Vec<_>>())
                );
            }
            if let Some(artifact_value) = result_artifacts
                .and_then(|artifacts| artifacts.get("bridge_temperature.thermo_model"))
            {
                if let Some(nodes) = artifact_value.get("nodes").and_then(|v| v.as_array()) {
                    println!("[artifact] bridge model nodes: {}", nodes.len());
                }
            }
            if let Some(artifact_value) =
                result_artifacts.and_then(|artifacts| artifacts.get("thermo_summary.result"))
            {
                if let Some(temp_delta) = artifact_value
                    .get("nodes")
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.get("temperature_delta"))
                {
                    println!(
                        "[artifact] thermo first node temperature_delta: {}",
                        temp_delta
                    );
                }
            }
        }
        Err(error) => {
            eprintln!("[error] run_workflow_graph failed: {error}");
            return Err(error);
        }
    }

    Ok(())
}
