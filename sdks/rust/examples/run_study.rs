use std::env;
use std::time::Duration;

use kyuubiki_headless_sdk::{KyuubikiAgentClient, KyuubikiAuth, KyuubikiSession, SdkResult};
use serde_json::json;

fn minimal_truss_2d_payload() -> serde_json::Value {
    json!({
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
                "x": 0.5,
                "y": 0.75,
                "fix_x": false,
                "fix_y": false,
                "load_x": 0.0,
                "load_y": -1000.0
            }
        ],
        "elements": [
            {
                "id": "e0",
                "node_i": 0,
                "node_j": 1,
                "area": 0.01,
                "youngs_modulus": 7.0e10
            },
            {
                "id": "e1",
                "node_i": 1,
                "node_j": 2,
                "area": 0.01,
                "youngs_modulus": 7.0e10
            },
            {
                "id": "e2",
                "node_i": 2,
                "node_j": 0,
                "area": 0.01,
                "youngs_modulus": 7.0e10
            }
        ]
    })
}

fn main() -> SdkResult<()> {
    let base_url = env::var("KYUUBIKI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:4000".into());
    let auth = env::var("KYUUBIKI_TOKEN").ok().map(KyuubikiAuth::access_token);

    let session = KyuubikiSession::from_control_plane_with_auth(&base_url, auth)?;
    let agent = KyuubikiAgentClient::new(session);

    let outcome = agent.run_study(
        "truss_2d",
        &minimal_truss_2d_payload(),
        Duration::from_secs(1),
        Duration::from_secs(60),
        true,
    )?;

    println!("terminal:");
    println!("{}", serde_json::to_string_pretty(&outcome.terminal)?);

    let job_id = outcome
        .terminal
        .get("job")
        .and_then(|job| job.get("job_id"))
        .and_then(|job_id| job_id.as_str())
        .expect("job_id in terminal payload");

    let first_page = agent.browse_result_chunks(job_id, "nodes", 0, 2)?;
    println!("\nfirst nodes page:");
    println!("{}", serde_json::to_string_pretty(&first_page)?);

    println!("\niterating element pages:");
    for page in agent.iter_result_chunks(job_id, "elements", 2, 0, None) {
        println!("{}", serde_json::to_string_pretty(&page?)?);
    }

    Ok(())
}
