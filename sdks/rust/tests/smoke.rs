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
