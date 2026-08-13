use std::env;
use std::time::Duration;

use kyuubiki_headless_sdk::{KyuubikiAgentClient, KyuubikiAuth, KyuubikiSession, SdkResult};

fn plane_quad_input() -> serde_json::Value {
    serde_json::json!({
        "plane_quad_2d_model": {
            "nodes": [
                {
                    "id": "n0",
                    "x": 0.0,
                    "y": 0.0,
                    "fix_x": true,
                    "fix_y": true,
                    "load_x": 0.0,
                    "load_y": 0.0
                },
                {
                    "id": "n1",
                    "x": 1.0,
                    "y": 0.0,
                    "fix_x": false,
                    "fix_y": true,
                    "load_x": 0.0,
                    "load_y": 0.0
                },
                {
                    "id": "n2",
                    "x": 1.0,
                    "y": 1.0,
                    "fix_x": false,
                    "fix_y": false,
                    "load_x": 0.0,
                    "load_y": -1000.0
                },
                {
                    "id": "n3",
                    "x": 0.0,
                    "y": 1.0,
                    "fix_x": true,
                    "fix_y": false,
                    "load_x": 0.0,
                    "load_y": 0.0
                }
            ],
            "elements": [
                {
                    "id": "q0",
                    "node_i": 0,
                    "node_j": 1,
                    "node_k": 2,
                    "node_l": 3,
                    "thickness": 0.02,
                    "youngs_modulus": 7.0e10,
                    "poisson_ratio": 0.33
                }
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

    let workflow_id = "workflow.plane-quad-2d-summary-json";
    let input_artifacts = plane_quad_input();

    let outcome = agent.run_workflow_catalog(
        workflow_id,
        &input_artifacts,
        None,
        Duration::from_millis(500),
        Duration::from_secs(300),
        include_result,
    )?;

    let terminal_job = outcome.terminal.get("job").and_then(|job| job.as_object());
    let terminal_job_id = terminal_job
        .and_then(|job| job.get("id").and_then(|value| value.as_str()))
        .or_else(|| terminal_job.and_then(|job| job.get("job_id").and_then(|value| value.as_str())))
        .unwrap_or("unknown");
    let terminal_status = outcome
        .terminal
        .get("job")
        .and_then(|job| job.get("status"))
        .and_then(|status| status.as_str())
        .unwrap_or("unknown");
    println!(
        "[workflow-catalog] id={workflow_id}, terminal_job_id={terminal_job_id}, terminal={terminal_status}"
    );
    println!(
        "[workflow-catalog] history events={}",
        outcome.history.len()
    );
    println!("[workflow-catalog] include_result={include_result}");

    if let Some(runtime) = outcome.workflow_runtime {
        println!(
            "[workflow-catalog] workflow_runtime current_node={}",
            runtime.current_node.as_deref().unwrap_or("null")
        );
        println!(
            "[workflow-catalog] workflow_runtime status={:?}",
            runtime.status.as_deref().unwrap_or("null")
        );
    }

    if let Some(manifest) = outcome.output_manifest {
        println!(
            "[workflow-catalog] output manifest: {}::{}",
            manifest.graph_id,
            manifest.outputs.len()
        );
        println!(
            "[workflow-catalog] output keys: {:?}",
            manifest
                .outputs
                .iter()
                .map(|output| format!("{}.{}", output.node_id, output.port_id))
                .collect::<Vec<_>>()
        );
    }

    if let Some(validated) = outcome.validated_outputs {
        println!(
            "[workflow-catalog] validated keys: {:?}",
            validated
                .manifest
                .outputs
                .iter()
                .map(|output| output.key.clone())
                .collect::<Vec<_>>()
        );
    }

    if let Some(result) = outcome.result {
        if let Some(artifacts) = result.get("artifacts").and_then(|value| value.as_object()) {
            println!(
                "[workflow-catalog] result artifact keys: {:?}",
                artifacts.keys().collect::<Vec<_>>()
            );
        }
        if let Some(json_output) = result
            .get("artifacts")
            .and_then(|artifacts| artifacts.get("json_output.json"))
            .and_then(|value| value.get("output"))
        {
            let text = json_output.to_string();
            println!(
                "[workflow-catalog] json_output.output snippet: {}",
                &text[..text.len().min(180)]
            );
        }
        if let Some(result_root) = result.as_object() {
            println!(
                "[workflow-catalog] result keys: {:?}",
                result_root.keys().collect::<Vec<_>>()
            );
        }
    }

    Ok(())
}
