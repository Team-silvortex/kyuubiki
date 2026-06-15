use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use kyuubiki_headless_sdk::{KyuubikiAgentClient, KyuubikiSession};

#[test]
fn agent_client_runs_study_and_browses_chunks() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");

    let server = thread::spawn(move || {
        for _ in 0..4 {
            let (mut stream, _) = listener.accept().expect("accept");
            let request = read_http_request(&mut stream);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");

            let (status, body) = match path {
                "/api/v1/fem/truss-2d/jobs" => (
                    202,
                    r#"{"job":{"job_id":"job-smoke","status":"queued"}}"#.to_string(),
                ),
                "/api/v1/jobs/job-smoke" => (
                    200,
                    r#"{"job":{"job_id":"job-smoke","status":"completed","progress":1.0}}"#
                        .to_string(),
                ),
                "/api/v1/results/job-smoke" => (
                    200,
                    r#"{"job_id":"job-smoke","result":{"nodes":[{"index":0,"id":"n0"},{"index":1,"id":"n1"},{"index":2,"id":"n2"}],"elements":[{"index":0,"id":"e0"}],"max_displacement":0.000001,"max_stress":70000.0}}"#
                        .to_string(),
                ),
                "/api/v1/results/job-smoke/chunks/nodes?offset=0&limit=2" => (
                    200,
                    r#"{"job_id":"job-smoke","kind":"nodes","offset":0,"limit":2,"returned":2,"total":3,"items":[{"index":0,"id":"n0"},{"index":1,"id":"n1"}]}"#
                        .to_string(),
                ),
                _ => (404, r#"{"error":"not_found"}"#.to_string()),
            };

            let response = format!(
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write response");
        }
    });

    let base_url = format!("http://{}", addr);
    let session = KyuubikiSession::from_control_plane(&base_url, None).expect("session");
    let agent = KyuubikiAgentClient::new(session);

    let outcome = agent
        .run_study(
            "truss_2d",
            &serde_json::json!({"nodes": [], "elements": []}),
            Duration::from_millis(10),
            Duration::from_secs(2),
            true,
        )
        .expect("run study");

    assert_eq!(
        outcome
            .terminal
            .get("job")
            .and_then(|job| job.get("status"))
            .and_then(|status| status.as_str()),
        Some("completed")
    );
    assert!(outcome.result.is_some());

    let page = agent
        .browse_result_chunks("job-smoke", "nodes", 0, 2)
        .expect("chunk page");
    assert_eq!(page.get("returned").and_then(|value| value.as_u64()), Some(2));
    assert_eq!(page.get("total").and_then(|value| value.as_u64()), Some(3));

    server.join().expect("server thread");
}

#[test]
fn control_plane_lists_and_fetches_workflow_operators() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");

    let server = thread::spawn(move || {
        for _ in 0..4 {
            let (mut stream, _) = listener.accept().expect("accept");
            let request = read_http_request(&mut stream);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");

            let (status, body) = match path {
                "/api/v1/workflows/catalog/workflow.test-graph" => (
                    200,
                    r#"{"workflow":{"id":"workflow.test-graph","graph":{"schema_version":"kyuubiki.workflow-graph/v1","id":"workflow.test-graph","name":"Test Graph","version":"1.0.0","entry_nodes":["input"],"output_nodes":["output"],"nodes":[{"id":"input","kind":"input","inputs":[],"outputs":[{"id":"mesh","artifact_type":"mesh.input"}]},{"id":"output","kind":"output","inputs":[{"id":"mesh_result","artifact_type":"mesh.result"}],"outputs":[]}],"edges":[]}}}"#.to_string(),
                ),
                "/api/v1/operators" => (
                    200,
                    r#"{"operators":[{"id":"solver.truss_2d","version":"1.0.0","domain":"structural","family":"solver","kind":"solver","summary":"Smoke operator"}]}"#
                        .to_string(),
                ),
                "/api/v1/operators?domain=structural&family=solver" => (
                    200,
                    r#"{"operators":[{"id":"solver.truss_2d","version":"1.0.0","domain":"structural","family":"solver","kind":"solver","summary":"Smoke operator"}]}"#
                        .to_string(),
                ),
                "/api/v1/operators/solver.truss_2d" => (
                    200,
                    r#"{"operator":{"id":"solver.truss_2d","version":"1.0.0","domain":"structural","family":"solver","kind":"solver","summary":"Smoke operator"}}"#
                        .to_string(),
                ),
                _ => (404, r#"{"error":"not_found"}"#.to_string()),
            };

            let response = format!(
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write response");
        }
    });

    let client = kyuubiki_headless_sdk::ControlPlaneClient::new(&format!("http://{}", addr)).expect("control plane client");

    let workflow = client
        .fetch_workflow_catalog_workflow("workflow.test-graph")
        .expect("workflow descriptor");
    assert_eq!(workflow["workflow"]["graph"]["id"].as_str(), Some("workflow.test-graph"));

    let operators = client.list_workflow_operators().expect("operator list");
    assert_eq!(
        operators["operators"][0]["id"].as_str(),
        Some("solver.truss_2d")
    );

    let filtered = client
        .list_workflow_operators_with_query(Some(&[
            ("domain", "structural".to_string()),
            ("family", "solver".to_string()),
        ]))
        .expect("filtered operator list");
    assert_eq!(
        filtered["operators"][0]["family"].as_str(),
        Some("solver")
    );

    let operator = client
        .fetch_workflow_operator("solver.truss_2d")
        .expect("operator descriptor");
    assert_eq!(operator["operator"]["kind"].as_str(), Some("solver"));

    server.join().expect("server thread");
}

#[test]
fn agent_client_runs_workflow_jobs() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");

    let server = thread::spawn(move || {
        for _ in 0..7 {
            let (mut stream, _) = listener.accept().expect("accept");
            let request = read_http_request(&mut stream);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");

            let (status, body) = match path {
                "/api/v1/workflows/catalog/workflow.test-graph" => (
                    200,
                    r#"{"workflow":{"id":"workflow.test-graph","graph":{"schema_version":"kyuubiki.workflow-graph/v1","id":"workflow.test-graph","name":"Test Graph","version":"1.0.0","entry_nodes":["input"],"output_nodes":["output"],"nodes":[{"id":"input","kind":"input","inputs":[],"outputs":[{"id":"mesh","artifact_type":"mesh.input"}]},{"id":"output","kind":"output","inputs":[{"id":"mesh_result","artifact_type":"mesh.result"}],"outputs":[]}],"edges":[]}}}"#.to_string(),
                ),
                "/api/v1/workflows/catalog/workflow.test-graph/jobs" => (
                    202,
                    r#"{"job":{"job_id":"workflow-catalog-job","status":"queued"}}"#.to_string(),
                ),
                "/api/v1/jobs/workflow-catalog-job" => (
                    200,
                    r#"{"job":{"job_id":"workflow-catalog-job","status":"completed","progress":1.0,"current_node":"output","completed_nodes":["input","output"],"progress_events":[{"node_id":"output","status":"completed"}]}}"#.to_string(),
                ),
                "/api/v1/results/workflow-catalog-job" => (
                    200,
                    r#"{"job_id":"workflow-catalog-job","result":{"workflow_id":"workflow.test-graph","run_id":"run-workflow-catalog","status":"completed","current_node":"output","completed_nodes":["input","output"],"progress_events":[{"node_id":"output","status":"completed"}],"artifacts":{"mesh.result":{"artifact_id":"artifact.catalog.result"}}}}"#.to_string(),
                ),
                "/api/v1/workflows/graph/jobs" => (
                    202,
                    r#"{"job":{"job_id":"workflow-graph-job","status":"queued"}}"#.to_string(),
                ),
                "/api/v1/jobs/workflow-graph-job" => (
                    200,
                    r#"{"job":{"job_id":"workflow-graph-job","status":"completed","progress":1.0,"current_node":"output","completed_nodes":["input","solve","output"],"progress_events":[{"node_id":"solve","status":"completed"}]}}"#.to_string(),
                ),
                "/api/v1/results/workflow-graph-job" => (
                    200,
                    r#"{"job_id":"workflow-graph-job","result":{"workflow_id":"workflow.test-inline","run_id":"run-workflow-graph","status":"completed","current_node":"output","completed_nodes":["input","solve","output"],"progress_events":[{"node_id":"solve","status":"completed"}],"artifacts":{"mesh.result":{"artifact_id":"artifact.graph.result"}}}}"#.to_string(),
                ),
                _ => (404, r#"{"error":"not_found"}"#.to_string()),
            };

            let response = format!(
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write response");
        }
    });

    let base_url = format!("http://{}", addr);
    let session = KyuubikiSession::from_control_plane(&base_url, None).expect("session");
    let agent = KyuubikiAgentClient::new(session);

    let catalog_outcome = agent
        .run_workflow_catalog(
            "workflow.test-graph",
            &serde_json::json!({"mesh": {"project_id": "demo"}}),
            None,
            Duration::from_millis(10),
            Duration::from_secs(2),
            true,
        )
        .expect("run workflow catalog");
    assert_eq!(
        catalog_outcome.result.as_ref().and_then(|value| value["result"]["artifacts"]["mesh.result"]["artifact_id"].as_str()),
        Some("artifact.catalog.result")
    );
    assert_eq!(
        catalog_outcome
            .validated_outputs
            .as_ref()
            .and_then(|value| value.artifacts["output.mesh_result"]["artifact_id"].as_str()),
        Some("artifact.catalog.result")
    );
    assert_eq!(
        catalog_outcome.workflow_runtime.as_ref().and_then(|value| value.run_id.as_deref()),
        Some("run-workflow-catalog")
    );
    assert_eq!(
        catalog_outcome
            .workflow_progression
            .as_ref()
            .and_then(|value| value.snapshots.first())
            .and_then(|value| value.current_node.as_deref()),
        Some("output")
    );

    let graph_definition = serde_json::json!({
        "schema_version": "kyuubiki.workflow-graph/v1",
        "id": "workflow.test-inline",
        "name": "Inline Graph",
        "version": "1.0.0",
        "entry_nodes": ["input"],
        "output_nodes": ["output"],
        "nodes": [
            {"id": "input", "kind": "input", "inputs": [], "outputs": [{"id": "mesh", "artifact_type": "mesh.input"}]},
            {"id": "solve", "kind": "solve", "operator_id": "solver.truss_2d", "inputs": [], "outputs": []},
            {"id": "output", "kind": "output", "inputs": [{"id": "mesh_result", "artifact_type": "mesh.result"}], "outputs": []}
        ],
        "edges": []
    });
    let graph_outcome = agent
        .run_workflow_graph(
            &graph_definition,
            &serde_json::json!({"mesh": {"project_id": "demo"}}),
            Duration::from_millis(10),
            Duration::from_secs(2),
            true,
        )
        .expect("run workflow graph");
    assert_eq!(
        graph_outcome.result.as_ref().and_then(|value| value["result"]["artifacts"]["mesh.result"]["artifact_id"].as_str()),
        Some("artifact.graph.result")
    );
    assert_eq!(
        graph_outcome.output_manifest.as_ref().map(|value| value.graph_id.as_str()),
        Some("workflow.test-inline")
    );
    assert_eq!(
        graph_outcome
            .validated_outputs
            .as_ref()
            .and_then(|value| value.artifacts["output.mesh_result"]["artifact_id"].as_str()),
        Some("artifact.graph.result")
    );
    assert_eq!(
        graph_outcome.workflow_runtime.as_ref().and_then(|value| value.current_node.as_deref()),
        Some("output")
    );
    assert_eq!(
        graph_outcome
            .workflow_progression
            .as_ref()
            .and_then(|value| value.latest.as_ref())
            .and_then(|value| value["run_id"].as_str()),
        Some("run-workflow-graph")
    );

    server.join().expect("server thread");
}

#[test]
fn session_supports_expanded_solve_kinds() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");

    let server = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept");
            let request = read_http_request(&mut stream);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");

            let (status, body) = match path {
                "/api/v1/fem/axial-bar/jobs" => (
                    202,
                    r#"{"job":{"job_id":"job-axial","status":"queued"}}"#.to_string(),
                ),
                "/api/v1/fem/thermal-frame-3d/jobs" => (
                    202,
                    r#"{"job":{"job_id":"job-thermal-frame-3d","status":"queued"}}"#.to_string(),
                ),
                _ => (404, r#"{"error":"not_found"}"#.to_string()),
            };

            let response = format!(
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write response");
        }
    });

    let session = KyuubikiSession::from_control_plane(&format!("http://{}", addr), None).expect("session");
    let axial = session
        .submit_job("axial_bar_1d", &serde_json::json!({"nodes": [], "elements": []}))
        .expect("axial alias submit");
    assert_eq!(axial["job"]["job_id"].as_str(), Some("job-axial"));

    let thermal_frame = session
        .submit_job("thermal_frame_3d", &serde_json::json!({"nodes": [], "elements": []}))
        .expect("thermal frame submit");
    assert_eq!(
        thermal_frame["job"]["job_id"].as_str(),
        Some("job-thermal-frame-3d")
    );

    server.join().expect("server thread");
}

#[test]
fn session_supports_direct_rpc_for_expanded_solve_kinds() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind rpc listener");
    let addr = listener.local_addr().expect("rpc listener addr");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let request = read_rpc_request(&mut stream);
        assert_eq!(
            request["method"].as_str(),
            Some("solve_electrostatic_plane_quad_2d")
        );

        let body = serde_json::json!({
            "ok": true,
            "result": {
                "solver": "electrostatic_plane_quad_2d",
                "input": request["params"].clone(),
            }
        });
        write_rpc_response(&mut stream, &body);
    });

    let session = KyuubikiSession::new(None, None).with_solver_rpc("127.0.0.1", addr.port());
    let result = session
        .solve_direct(
            "electrostatic_plane_quad_2d",
            serde_json::json!({"nodes": [], "elements": []}),
        )
        .expect("direct solve");
    assert_eq!(
        result["solver"].as_str(),
        Some("electrostatic_plane_quad_2d")
    );

    server.join().expect("server thread");
}

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];

    loop {
        let size = stream.read(&mut chunk).expect("read request chunk");
        if size == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..size]);

        if let Some(header_end) = find_bytes(&buffer, b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&buffer[..header_end + 4]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let lower = line.to_ascii_lowercase();
                    lower
                        .strip_prefix("content-length: ")
                        .and_then(|value| value.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            let body_received = buffer.len().saturating_sub(header_end + 4);
            if body_received >= content_length {
                break;
            }
        }
    }

    String::from_utf8_lossy(&buffer).into_owned()
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn read_rpc_request(stream: &mut std::net::TcpStream) -> serde_json::Value {
    let mut header = [0_u8; 4];
    stream.read_exact(&mut header).expect("read rpc header");
    let size = u32::from_be_bytes(header) as usize;
    let mut payload = vec![0_u8; size];
    stream.read_exact(&mut payload).expect("read rpc body");
    serde_json::from_slice(&payload).expect("decode rpc request")
}

fn write_rpc_response(stream: &mut std::net::TcpStream, body: &serde_json::Value) {
    let payload = serde_json::to_vec(body).expect("encode rpc response");
    let size = (payload.len() as u32).to_be_bytes();
    stream.write_all(&size).expect("write rpc size");
    stream.write_all(&payload).expect("write rpc payload");
}
