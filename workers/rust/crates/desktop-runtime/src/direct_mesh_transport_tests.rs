use super::*;
use std::net::TcpListener;
use std::thread;

fn exchange(
    method: &str,
    frames: impl FnOnce(&str) -> Vec<Value> + Send + 'static,
) -> Result<(Value, Vec<Value>), String> {
    exchange_peer(method, RPC_TIMEOUT, move |mut stream, input| {
        for frame in frames(input["id"].as_str().unwrap()) {
            if send_frame(&mut stream, &frame).is_err() {
                break;
            }
        }
    })
    .0
}

fn send_frame(stream: &mut TcpStream, frame: &Value) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(frame).unwrap();
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(&bytes)
}

type Exchange = (Result<(Value, Vec<Value>), String>, Duration);
fn exchange_peer(
    method: &str,
    timeout: Duration,
    respond: impl FnOnce(TcpStream, Value) + Send + 'static,
) -> Exchange {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let input = read_frame(&mut stream, "test-peer").unwrap();
        respond(stream, input);
    });
    let start = Instant::now();
    let result = request_with_timeout(&endpoint, method, json!({}), timeout);
    let elapsed = start.elapsed();
    peer.join().unwrap();
    (result, elapsed)
}

#[test]
fn recovery_transport_rejects_invalid_terminal_envelopes() {
    for defect in [
        "wrong_version",
        "missing_version",
        "contradictory",
        "missing_result",
        "null_result",
    ] {
        let result = exchange("fetch_durable_agent_execution", move |id| {
            let mut frame =
                json!({"rpc_version":1,"id":id,"ok":true,"result":{"status":"not_found"}});
            match defect {
                "wrong_version" => frame["rpc_version"] = json!(2),
                "missing_version" => {
                    frame.as_object_mut().unwrap().remove("rpc_version");
                }
                "contradictory" => {
                    frame["error"] = json!({"code":"failure","message":"not success"})
                }
                "missing_result" => {
                    frame.as_object_mut().unwrap().remove("result");
                }
                _ => frame["result"] = Value::Null,
            }
            vec![frame]
        });
        assert!(result.is_err(), "accepted {defect}: {result:?}");
    }
}

#[test]
fn recovery_control_never_accepts_a_progress_stream() {
    for method in [
        "describe_agent",
        "submit_durable_agent_task",
        "fetch_durable_agent_execution",
        "list_durable_agent_executions",
        "cancel_durable_agent_execution",
    ] {
        let result = exchange(method, |id| {
            vec![
                progress(id, 0, "normal heartbeat"),
                json!({"rpc_version":1,"id":id,"ok":true,"result":{}}),
            ]
        });
        assert!(result.is_err(), "accepted control progress for {method}");
    }
}

fn progress(id: &str, iteration: usize, message: &str) -> Value {
    json!({"rpc_version":1,"id":id,"event":"progress", "progress":{
        "job_id":"solve", "stage":"solving", "progress":0.5,
        "iteration":iteration,"message":message,
    }})
}

#[test]
fn fragmented_frames_cannot_renew_control_or_solver_frame_deadlines() {
    for method in [
        "describe_agent",
        "fetch_durable_agent_execution",
        "solve_bar_1d",
    ] {
        for phase in ["header", "payload"] {
            let (result, elapsed) = exchange_peer(
                method,
                Duration::from_millis(180),
                move |mut stream, request| {
                    let bytes = serde_json::to_vec(
                        &json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{}}),
                    )
                    .unwrap();
                    let header = (bytes.len() as u32).to_be_bytes();
                    if phase == "header" {
                        for byte in header {
                            if stream.write_all(&[byte]).is_err() {
                                return;
                            }
                            thread::sleep(Duration::from_millis(85));
                        }
                        let _ = stream.write_all(&bytes);
                    } else {
                        stream.write_all(&header).unwrap();
                        for byte in bytes {
                            if stream.write_all(&[byte]).is_err() {
                                return;
                            }
                            thread::sleep(Duration::from_millis(25));
                        }
                    }
                },
            );
            assert!(result.is_err(), "accepted slow {phase} on {method}");
            assert!(
                elapsed < Duration::from_secs(1),
                "{method}/{phase}: {elapsed:?}"
            );
        }
    }
}

#[test]
fn header_and_payload_share_one_control_deadline() {
    let (result, elapsed) = exchange_peer(
        "fetch_durable_agent_execution",
        Duration::from_millis(300),
        |mut stream, request| {
            let bytes = serde_json::to_vec(
                &json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{}}),
            )
            .unwrap();
            thread::sleep(Duration::from_millis(200));
            if stream
                .write_all(&(bytes.len() as u32).to_be_bytes())
                .is_err()
            {
                return;
            }
            thread::sleep(Duration::from_millis(200));
            let _ = stream.write_all(&bytes);
        },
    );
    assert!(result.is_err());
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn healthy_long_solve_progress_does_not_inherit_the_control_total_deadline() {
    let (result, elapsed) = exchange_peer(
        "solve_bar_1d",
        Duration::from_millis(300),
        |mut stream, request| {
            for iteration in 0..4 {
                thread::sleep(Duration::from_millis(120));
                send_frame(
                    &mut stream,
                    &progress(request["id"].as_str().unwrap(), iteration, "solving"),
                )
                .unwrap();
            }
            send_frame(
                &mut stream,
                &json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{"max_stress":10.0}}),
            )
            .unwrap();
        },
    );
    let (result, progress) = result.unwrap();
    assert!(elapsed > Duration::from_millis(300));
    assert_eq!(result["max_stress"], 10.0);
    assert_eq!(progress.len(), 4);
}

#[test]
fn reply_frame_limits_reject_headers_before_allocating_the_declared_payload() {
    for (method, max) in [
        ("describe_agent", 1024 * 1024),
        ("list_durable_agent_executions", 1024 * 1024),
        ("cancel_durable_agent_execution", 1024 * 1024),
        ("submit_durable_agent_task", 9 * 1024 * 1024),
        ("fetch_durable_agent_execution", 9 * 1024 * 1024),
        ("solve_bar_1d", MAX_FRAME_BYTES),
    ] {
        for length in [0, max + 1] {
            let (result, _) =
                exchange_peer(method, Duration::from_secs(1), move |mut stream, _| {
                    stream.write_all(&(length as u32).to_be_bytes()).unwrap();
                });
            assert!(
                result.unwrap_err().contains("frame exceeds limit"),
                "{method}"
            );
        }
    }
}

#[test]
fn solver_progress_retention_is_bounded_by_both_count_and_encoded_bytes() {
    for large in [false, true] {
        let (result, retained) = exchange("solve_bar_1d", move |id| {
            let message = if large {
                "\0".repeat(4096)
            } else {
                "solving".into()
            };
            let mut frames: Vec<_> = (0..512).map(|i| progress(id, i, &message)).collect();
            frames.push(json!({"rpc_version":1,"id":id,"ok":true,"result":{"max_stress":10.0}}));
            frames
        })
        .unwrap();
        assert_eq!(result["max_stress"], 10.0);
        assert!(!retained.is_empty() && retained.len() <= MAX_PROGRESS_FRAMES);
        assert_eq!(retained.last().unwrap()["progress"]["iteration"], 511);
        let bytes: usize = retained
            .iter()
            .map(|value| serde_json::to_vec(value).unwrap().len())
            .sum();
        assert!(bytes <= MAX_PROGRESS_BYTES);
        if large {
            assert!(retained.len() < MAX_PROGRESS_FRAMES);
        } else {
            assert_eq!(retained.len(), MAX_PROGRESS_FRAMES);
        }
    }
}

#[test]
fn invalid_progress_cannot_keep_a_solve_transport_alive() {
    for defect in [
        "version", "event", "identity", "range", "message", "terminal",
    ] {
        let result = exchange("solve_bar_1d", move |id| {
            let mut frame = progress(id, 0, "solving");
            match defect {
                "version" => frame["rpc_version"] = json!(2),
                "event" => frame["event"] = json!("unknown"),
                "identity" => frame["id"] = json!("another-request"),
                "range" => frame["progress"]["progress"] = json!(2),
                "message" => frame["progress"]["message"] = json!("x".repeat(4097)),
                _ => frame["ok"] = json!(true),
            }
            vec![
                frame,
                json!({"rpc_version":1,"id":id,"ok":true,"result":{}}),
            ]
        });
        assert!(result.is_err(), "accepted {defect}");
    }
}

#[test]
fn stalled_control_connection_does_not_block_another_agents_query() {
    let (started, observed) = std::sync::mpsc::channel();
    let (release, held) = std::sync::mpsc::channel();
    let stalled = thread::spawn(move || {
        exchange_peer(
            "fetch_durable_agent_execution",
            Duration::from_millis(500),
            move |_stream, _| {
                started.send(()).unwrap();
                let _ = held.recv_timeout(Duration::from_secs(3));
            },
        )
    });
    observed.recv_timeout(Duration::from_secs(2)).unwrap();
    let healthy = exchange("fetch_durable_agent_execution", |id| {
        vec![json!({"rpc_version":1,"id":id,"ok":true,"result":{"status":"outcome_unknown"}})]
    })
    .unwrap();
    assert_eq!(healthy.0["status"], "outcome_unknown");
    release.send(()).unwrap();
    assert!(stalled.join().unwrap().0.is_err());
    assert!(
        exchange("describe_agent", |id| vec![
            json!({"rpc_version":1,"id":id,"ok":true,"result":{}})
        ])
        .is_ok()
    );
}
