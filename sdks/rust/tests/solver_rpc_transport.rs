use kyuubiki_headless_sdk::{SdkError, SolverRpcClient, SolverRpcLimits};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

fn read_request(stream: &mut TcpStream) -> Value {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut header = [0; 4];
    stream.read_exact(&mut header).unwrap();
    let size = u32::from_be_bytes(header) as usize;
    assert!(size < 65536);
    let mut bytes = vec![0; size];
    stream.read_exact(&mut bytes).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn send(stream: &mut TcpStream, value: &Value) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(value).unwrap();
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(&bytes)
}

fn peer(
    reply: impl FnOnce(Value, &mut TcpStream) + Send + 'static,
) -> (SolverRpcClient, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_request(&mut stream);
        reply(request, &mut stream);
    });
    (SolverRpcClient::new("127.0.0.1", port), worker)
}

#[test]
fn a_successful_reply_for_another_request_is_not_our_result() {
    let (client, worker) = peer(|_, stream| {
        send(
            stream,
            &json!({"rpc_version":1,"id":"another-request","ok":true,"result":{"value":42}}),
        )
        .unwrap();
    });
    let result = client.ping();
    worker.join().unwrap();
    assert!(
        result.is_err(),
        "successful foreign response must not be accepted"
    );
}

#[test]
fn contradictory_success_and_failure_is_not_a_successful_result() {
    let (client, worker) = peer(|request, stream| {
        send(
            stream,
            &json!({"rpc_version":1,"id":request["id"],"ok":true,
            "result":{"value":42},"error":{"code":"failed","message":"calculation failed"}}),
        )
        .unwrap();
    });
    let result = client.ping();
    worker.join().unwrap();
    assert!(result.is_err(), "failure must not be hidden behind ok=true");
}

#[test]
fn request_ids_do_not_depend_on_clock_tick_resolution() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let worker = thread::spawn(move || {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..64 {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            ids.insert(request["id"].as_str().unwrap().to_owned());
            send(
                &mut stream,
                &json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{}}),
            )
            .unwrap();
        }
        ids
    });
    for _ in 0..64 {
        SolverRpcClient::new("127.0.0.1", port).ping().unwrap();
    }
    assert_eq!(worker.join().unwrap().len(), 64);
}

#[test]
fn valid_rpc_failure_retains_its_code_and_message() {
    let (client, worker) = peer(|request, stream| {
        send(
            stream,
            &json!({"rpc_version":1,"id":request["id"],"ok":false,
            "error":{"code":"cancelled","message":"cancelled at checkpoint"}}),
        )
        .unwrap();
    });
    let error = client.ping().err().unwrap();
    worker.join().unwrap();
    assert!(
        matches!(error, SdkError::Rpc {code:Some(code), message, ..} if code == "cancelled" && message == "cancelled at checkpoint")
    );
}

#[test]
fn structured_failure_keeps_request_identity_and_opaque_diagnostics() {
    let (client, worker) = peer(|request, stream| {
        send(
            stream,
            &json!({"rpc_version":1,"id":request["id"],"ok":false,
            "error":{"code":"cancelled","message":"original reason","details":{
                "request_id":request["id"],"private":"diagnostic-not-for-display",
                "automatic_replay_authorized":true,"ratio":1.0000000000000002}}}),
        )
        .unwrap();
    });
    let error = client.ping().err().unwrap();
    worker.join().unwrap();
    assert_eq!(error.to_string(), "cancelled: original reason");
    match error {
        SdkError::Rpc {
            request_id: Some(id),
            details: Some(details),
            ..
        } => {
            assert_eq!(details["request_id"], id);
            assert_eq!(details["private"], "diagnostic-not-for-display");
            assert_eq!(details["automatic_replay_authorized"], true);
            assert_eq!(
                details["ratio"].as_f64().unwrap().to_bits(),
                1.0000000000000002_f64.to_bits()
            );
        }
        other => panic!("expected structured failure: {other}"),
    }
    let (client, worker) = peer(|_, stream| {
        send(
            stream,
            &json!({"rpc_version":1,"id":"foreign","ok":false,
            "error":{"code":"cancelled","message":"foreign reason","details":{"trusted":true}}}),
        )
        .unwrap();
    });
    let error = client.ping().err().unwrap();
    worker.join().unwrap();
    assert!(matches!(error, SdkError::Transport(_)));
}

#[test]
fn local_configuration_errors_do_not_invent_remote_identity_or_details() {
    let error = SolverRpcClient::new("invalid.invalid", 1)
        .solve_study("not_a_solver", json!({}))
        .err()
        .unwrap();
    assert!(matches!(
        error,
        SdkError::Rpc {
            request_id: None,
            details: None,
            ..
        }
    ));
}

fn progress(id: &Value) -> Value {
    json!({"rpc_version":1, "id":id, "event":"heartbeat",
        "progress":{"job_id":"probe", "stage":"solving", "progress":0.5}})
}

#[test]
fn malformed_envelopes_and_progress_never_reach_a_success() {
    let mutations = [
        ("/rpc_version", json!(2)),
        ("/rpc_version", Value::Null),
        ("/id", Value::Null),
        ("/ok", json!("true")),
        ("/result", Value::Null),
        ("/ok", Value::Null),
        ("/ok", json!(false)),
    ];
    for (field, value) in mutations {
        let (client, worker) = peer(move |request, stream| {
            let mut frame = json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{}});
            *frame.pointer_mut(field).unwrap() = value;
            let _ = send(stream, &frame);
        });
        let rejected = client.ping().is_err();
        worker.join().unwrap();
        assert!(rejected, "{field}");
    }
    for (field, value) in [
        ("id", json!("foreign")),
        ("rpc_version", json!(2)),
        ("event", json!("unknown")),
        ("ok", json!(true)),
        ("result", json!({})),
        ("progress", Value::Null),
        (
            "progress",
            json!({"job_id":"probe","stage":"solving","progress":1.1}),
        ),
        (
            "progress",
            json!({"job_id":"probe","stage":"completed","progress":0.5}),
        ),
        (
            "progress",
            json!({"job_id":"probe","stage":"solving","progress":0.5,"iteration":-1}),
        ),
        (
            "progress",
            json!({"job_id":"probe","stage":"solving","progress":0.5,"residual":-1}),
        ),
        (
            "progress",
            json!({"job_id":"","stage":"solving","progress":0.5}),
        ),
        (
            "progress",
            json!({"job_id":"probe","stage":"invented","progress":0.5}),
        ),
    ] {
        let (client, worker) = peer(move |request, stream| {
            let mut frame = progress(&request["id"]);
            frame[field] = value;
            let _ = send(stream, &frame);
            let _ = send(
                stream,
                &json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{}}),
            );
        });
        let rejected = client.ping().is_err();
        worker.join().unwrap();
        assert!(rejected, "{field}");
    }
}

#[test]
fn duplicate_envelope_keys_and_truncated_json_are_rejected() {
    for duplicate in [true, false] {
        let (client, worker) = peer(move |request, stream| {
            let id = serde_json::to_string(&request["id"]).unwrap();
            let frame = if duplicate {
                format!(r#"{{"rpc_version":1,"id":{id},"ok":false,"ok":true,"result":{{}}}}"#)
            } else {
                "{\"rpc_version\":".into()
            };
            stream
                .write_all(&(frame.len() as u32).to_be_bytes())
                .unwrap();
            let _ = stream.write_all(frame.as_bytes());
        });
        let rejected = client.ping().is_err();
        worker.join().unwrap();
        assert!(rejected);
    }
}

#[test]
fn oversized_or_empty_frame_headers_are_rejected_before_body_allocation() {
    for size in [0, 513, u32::MAX] {
        let (client, worker) = peer(move |_, stream| {
            stream.write_all(&size.to_be_bytes()).unwrap();
        });
        let limits = SolverRpcLimits {
            max_response_bytes: 512,
            ..Default::default()
        };
        let error = client.with_limits(limits).unwrap().ping().err().unwrap();
        worker.join().unwrap();
        assert!(error.to_string().contains("frame"), "{error}");
    }
}

#[test]
fn request_limit_and_invalid_policy_fail_before_connecting() {
    let client = SolverRpcClient::new("invalid.invalid", 1)
        .with_limits(SolverRpcLimits {
            max_request_bytes: 100,
            ..Default::default()
        })
        .unwrap();
    let error = client
        .call("ping", json!({"large":"x".repeat(200)}))
        .err()
        .unwrap();
    assert!(error.to_string().contains("not dispatched"), "{error}");
    for limits in [
        SolverRpcLimits {
            max_request_bytes: 0,
            ..Default::default()
        },
        SolverRpcLimits {
            max_response_bytes: 0,
            ..Default::default()
        },
        SolverRpcLimits {
            io_timeout: Duration::ZERO,
            ..Default::default()
        },
        SolverRpcLimits {
            call_timeout: Duration::ZERO,
            ..Default::default()
        },
        SolverRpcLimits {
            call_timeout: Duration::MAX,
            ..Default::default()
        },
    ] {
        assert!(
            SolverRpcClient::new("invalid.invalid", 1)
                .with_limits(limits)
                .is_err()
        );
    }
}

#[test]
fn both_progress_frame_and_cumulative_byte_limits_are_enforced() {
    for by_bytes in [false, true] {
        let (client, worker) = peer(|request, stream| {
            for _ in 0..4 {
                if send(stream, &progress(&request["id"])).is_err() {
                    return;
                }
            }
            let _ = send(
                stream,
                &json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{}}),
            );
        });
        let limits = if by_bytes {
            SolverRpcLimits {
                max_progress_bytes: 512,
                ..Default::default()
            }
        } else {
            SolverRpcLimits {
                max_progress_frames: 2,
                ..Default::default()
            }
        };
        let error = client.with_limits(limits).unwrap().ping().err().unwrap();
        worker.join().unwrap();
        assert!(
            error.to_string().contains(if by_bytes {
                "byte budget"
            } else {
                "frame budget"
            }),
            "{error}"
        );
    }
}

#[test]
fn valid_progress_and_terminal_result_preserve_data_at_exact_limits() {
    let (client, worker) = peer(|request, stream| {
        for event in ["progress", "heartbeat"] {
            let mut frame = progress(&request["id"]);
            frame["event"] = json!(event);
            send(stream, &frame).unwrap();
        }
        send(stream, &json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{"x":1.0000000000000002,"zero":-0.0},"error":null})).unwrap();
    });
    let outcome = client
        .with_limits(SolverRpcLimits {
            max_progress_frames: 2,
            ..Default::default()
        })
        .unwrap()
        .ping()
        .unwrap();
    worker.join().unwrap();
    assert_eq!(outcome.progress_frames.len(), 2);
    assert_eq!(outcome.progress_frames[1]["event"], "heartbeat");
    assert_eq!(
        outcome.result["x"].as_f64().unwrap().to_bits(),
        1.0000000000000002_f64.to_bits()
    );
    assert_eq!(
        outcome.result["zero"].as_f64().unwrap().to_bits(),
        (-0.0_f64).to_bits()
    );
}

#[test]
fn heartbeats_and_partial_headers_cannot_extend_the_total_deadline() {
    for partial in [false, true] {
        let (client, worker) = peer(move |request, stream| {
            for _ in 0..100 {
                let written = if partial {
                    stream.write_all(&[0])
                } else {
                    send(stream, &progress(&request["id"]))
                };
                if written.is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(60));
            }
        });
        let limits = SolverRpcLimits {
            io_timeout: Duration::from_secs(2),
            call_timeout: Duration::from_millis(150),
            ..Default::default()
        };
        let started = std::time::Instant::now();
        let error = client.with_limits(limits).unwrap().ping().err().unwrap();
        let elapsed = started.elapsed();
        worker.join().unwrap();
        assert!(matches!(error, SdkError::Timeout(_)), "{error}");
        assert!(
            elapsed < Duration::from_secs(3),
            "deadline not enforced: {elapsed:?}"
        );
    }
}

#[test]
fn exact_response_and_progress_byte_budgets_still_accept_valid_frames() {
    let (client, worker) = peer(|request, stream| {
        for _ in 0..2 {
            let mut frame = progress(&request["id"]);
            frame["extension"] = json!("");
            let length = serde_json::to_vec(&frame).unwrap().len();
            frame["extension"] = json!("x".repeat(256 - length));
            assert_eq!(serde_json::to_vec(&frame).unwrap().len(), 256);
            send(stream, &frame).unwrap();
        }
        let mut frame =
            json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{"padding":""}});
        let length = serde_json::to_vec(&frame).unwrap().len();
        frame["result"]["padding"] = json!("x".repeat(256 - length));
        assert_eq!(serde_json::to_vec(&frame).unwrap().len(), 256);
        send(stream, &frame).unwrap();
    });
    let limits = SolverRpcLimits {
        max_response_bytes: 256,
        max_progress_bytes: 512,
        max_progress_frames: 2,
        ..Default::default()
    };
    let result = client.with_limits(limits).unwrap().ping().unwrap();
    worker.join().unwrap();
    assert_eq!(result.progress_frames.len(), 2);
    assert!(
        result
            .progress_frames
            .iter()
            .all(|frame| frame["extension"].as_str().is_some_and(|s| !s.is_empty()))
    );
    assert!(result.result["padding"].is_string());
}

#[test]
fn partial_header_or_body_disconnect_is_not_a_result() {
    for body in [false, true] {
        let (client, worker) = peer(move |_, stream| {
            if body {
                stream.write_all(&100_u32.to_be_bytes()).unwrap();
                stream.write_all(b"{\"rpc_version\":1").unwrap();
            } else {
                stream.write_all(&[0, 0]).unwrap();
            }
        });
        let error = client.ping().err().unwrap();
        worker.join().unwrap();
        assert!(
            matches!(error, SdkError::Io(ref error) if error.kind() == std::io::ErrorKind::UnexpectedEof)
        );
    }
}
