use super::*;
use crate::service_executor::{request_json, request_json_with_timeout};
use crate::service_executor_ack_loss_tests::{observe, run};
use crate::service_executor_deadline::write_parts_before_deadline;
use crate::service_executor_response::OUTCOME_UNKNOWN;
use serde_json::json;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

fn response() -> &'static [u8] {
    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}"
}

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut wire = Vec::new();
    let mut expected = None;
    let mut buffer = [0; 4096];
    loop {
        let count = stream.read(&mut buffer).unwrap();
        assert!(count > 0);
        wire.extend_from_slice(&buffer[..count]);
        assert!(wire.len() <= MAX_INLINE_JSON_BYTES + 16 * 1024);
        if expected.is_none() {
            if let Some(end) = wire.windows(4).position(|part| part == b"\r\n\r\n") {
                let head = std::str::from_utf8(&wire[..end]).unwrap();
                let length = head
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("Content-Length: ")
                            .map(|length| length.parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                expected = Some(end + 4 + length);
            }
        }
        if expected.is_some_and(|length| wire.len() >= length) {
            assert_eq!(Some(wire.len()), expected);
            return wire;
        }
    }
}

fn accept(listener: &TcpListener) -> TcpStream {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).unwrap();
                return stream;
            }
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(2))
            }
            Err(error) => panic!("bounded test accept: {error}"),
        }
    }
}

fn listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    (listener, url)
}

#[test]
fn bounded_serializer_matches_json_bytes_and_exact_counts_across_value_shapes() {
    let values = [
        Value::Null,
        json!(true),
        json!(false),
        json!(i64::MIN),
        json!(u64::MAX),
        json!(-0.0),
        json!(1.0 / 3.0),
        json!(""),
        json!("\"\\\n\r\t\u{0}"),
        json!("中文 日本語 العربية é 😀"),
        json!([1, null, "{{steps.1.result}}"]),
        json!({"nested":{"array":[1,2,3]},"escaped\nkey":"literal"}),
    ];
    for value in values {
        let expected = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            serialize_inline_json("/test", value.clone()).unwrap(),
            expected
        );
        for limit in [0, 1, 2, 3, 7, 31, 255, 4096] {
            let mut writer = InlineJsonWriter::new(limit);
            serde_json::to_writer(&mut writer, &value).unwrap();
            assert_eq!(writer.size_bytes, expected.len());
            assert!(writer.bytes.capacity() <= limit);
            if expected.len() <= limit {
                assert_eq!(writer.bytes, expected);
            } else {
                assert!(writer.bytes.is_empty());
                assert_eq!(writer.bytes.capacity(), 0);
            }
        }
    }
}

#[test]
fn buffer_growth_stays_bounded_and_overflow_releases_retained_bytes() {
    let mut writer = InlineJsonWriter::new(257);
    for _ in 0..257 {
        writer.write_all(b"x").unwrap();
        assert!(writer.bytes.capacity() <= 257);
    }
    assert_eq!(writer.bytes.len(), 257);
    writer.write_all(b"overflow").unwrap();
    assert_eq!(writer.size_bytes, 265);
    assert_eq!(writer.bytes.capacity(), 0);
    writer.write_all(&[b'x'; 4096]).unwrap();
    assert_eq!(writer.size_bytes, 4361);
    assert_eq!(writer.bytes.capacity(), 0);
    writer.flush().unwrap();
}

#[test]
fn exact_inline_boundary_is_accepted_and_one_byte_overflow_keeps_size_diagnostic() {
    let bytes =
        serialize_inline_json("/test", json!("x".repeat(MAX_INLINE_JSON_BYTES - 2))).unwrap();
    assert_eq!(bytes.len(), MAX_INLINE_JSON_BYTES);
    assert_eq!(bytes.first(), Some(&b'"'));
    assert_eq!(bytes.last(), Some(&b'"'));
    let error =
        serialize_inline_json("/test", json!("x".repeat(MAX_INLINE_JSON_BYTES - 1))).unwrap_err();
    assert_eq!(
        error,
        validate_inline_json_size("/test", MAX_INLINE_JSON_BYTES + 1).unwrap_err()
    );
    assert!(!error.message.starts_with(OUTCOME_UNKNOWN));
}

#[test]
fn oversized_escaped_unicode_json_reports_full_wire_size_without_retaining_it() {
    let text = "中\n\u{0}".repeat(800_000);
    let expected_size = 800_000 * (3 + 2 + 6) + 2;
    let mut writer = InlineJsonWriter::new(MAX_INLINE_JSON_BYTES);
    serde_json::to_writer(&mut writer, &text).unwrap();
    assert_eq!(writer.size_bytes, expected_size);
    assert_eq!(writer.bytes.capacity(), 0);
    let error = serialize_inline_json("/test", Value::String(text)).unwrap_err();
    assert!(
        error
            .message
            .contains(&format!("size_bytes={expected_size}"))
    );
    assert!(!error.message.contains('中'));
}

#[test]
fn byte_counter_overflow_fails_without_wrapping_or_mutating_buffer() {
    let mut writer = InlineJsonWriter::new(2);
    writer.size_bytes = usize::MAX;
    let error = writer.write(b"x").unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert_eq!(writer.size_bytes, usize::MAX);
    assert!(writer.bytes.is_empty());
}

#[test]
fn request_head_retains_wire_headers_without_holding_any_body() {
    let head = build_request_head(
        "POST",
        "127.0.0.1",
        "/test",
        MAX_INLINE_JSON_BYTES,
        Some("fixture"),
    );
    assert_eq!(
        head,
        concat!(
            "POST /test HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: application/json\r\n",
            "Connection: close\r\nAuthorization: Bearer fixture\r\n",
            "Content-Type: application/json\r\nContent-Length: 8000000\r\n\r\n"
        )
    );
    assert!(head.capacity() < 1024);
    assert_eq!(
        build_request_head("GET", "127.0.0.1", "/test", 0, None),
        concat!(
            "GET /test HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: application/json\r\n",
            "Connection: close\r\n\r\n"
        )
    );
}

#[test]
fn socket_requests_preserve_absent_null_unicode_and_escaped_bodies_byte_for_byte() {
    for (method, body) in [
        ("GET", None),
        ("HEAD", None),
        ("OPTIONS", None),
        ("POST", Some(Value::Null)),
        (
            "PUT",
            Some(json!({"note":"中文\nالعربية\t\"\\","binding":"{{steps.1.result}}"})),
        ),
        ("PATCH", Some(json!([1, true, null]))),
        ("DELETE", Some(json!({}))),
    ] {
        let expected = body
            .as_ref()
            .map(|value| serde_json::to_vec(value).unwrap())
            .unwrap_or_default();
        let (result, wire) = observe(response(), |url| {
            request_json(url, Some("fixture"), method, "test", body)
        });
        assert_eq!(result.unwrap(), json!({}));
        let (head, actual) = wire.split_once("\r\n\r\n").unwrap();
        assert!(head.starts_with(&format!("{method} /test HTTP/1.1\r\n")));
        assert!(head.contains("Authorization: Bearer fixture"));
        assert_eq!(actual.as_bytes(), expected);
        if expected.is_empty() {
            assert!(!head.contains("Content-Length:"));
            assert!(!head.contains("Content-Type:"));
        } else {
            assert!(head.contains(&format!("Content-Length: {}", expected.len())));
        }
    }
}

#[test]
fn exact_limit_unicode_body_arrives_complete_across_fragmented_socket_reads() {
    let text = format!("{}xxx", "中".repeat((MAX_INLINE_JSON_BYTES - 5) / 3));
    let payload = Value::String(text);
    let expected = serde_json::to_vec(&payload).unwrap();
    assert_eq!(expected.len(), MAX_INLINE_JSON_BYTES);
    let (listener, url) = listener();
    let server = std::thread::spawn(move || {
        let mut stream = accept(&listener);
        let wire = read_request(&mut stream);
        stream.write_all(response()).unwrap();
        let end = wire
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
            .unwrap()
            + 4;
        assert_eq!(&wire[end..], expected);
        assert!(
            std::str::from_utf8(&wire[..end])
                .unwrap()
                .contains("Content-Length: 8000000\r\n")
        );
    });
    let result = request_json(&url, None, "POST", "/test", Some(payload));
    server.join().unwrap();
    assert_eq!(result.unwrap(), json!({}));
}

#[test]
fn oversize_invalid_path_and_token_are_rejected_before_connecting() {
    let (listener, url) = listener();
    for (path, token, value) in [
        ("/test", None, json!("x".repeat(MAX_INLINE_JSON_BYTES))),
        ("/test\r\nInjected: yes", None, json!({})),
        ("/test", Some("fixture\r\nInjected: yes"), json!({})),
    ] {
        let error = request_json(&url, token, "POST", path, Some(value)).unwrap_err();
        assert!(!error.message.starts_with(OUTCOME_UNKNOWN));
        assert!(!error.message.contains("fixture"));
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }
}

#[test]
fn expired_explicit_deadline_never_opens_a_connection_or_marks_a_write_unknown() {
    let (listener, url) = listener();
    let error = request_json_with_timeout(
        &url,
        None,
        "POST",
        "/test",
        Some(json!({})),
        Some(Instant::now() - Duration::from_millis(1)),
        Duration::from_secs(1),
    )
    .unwrap_err();
    assert!(!error.message.starts_with(OUTCOME_UNKNOWN));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}

#[test]
fn shared_deadline_is_not_renewed_after_sending_body_and_loss_remains_unknown() {
    let (listener, url) = listener();
    let payload = json!({"note":"x".repeat(256 * 1024)});
    let expected = serde_json::to_vec(&payload).unwrap();
    let server = std::thread::spawn(move || {
        let mut stream = accept(&listener);
        let wire = read_request(&mut stream);
        let end = wire
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
            .unwrap()
            + 4;
        assert_eq!(&wire[end..], expected);
        let mut byte = [0];
        assert_eq!(stream.read(&mut byte).unwrap(), 0);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    });
    let started = Instant::now();
    let error = request_json_with_timeout(
        &url,
        None,
        "POST",
        "/test",
        Some(payload),
        Some(started + Duration::from_millis(500)),
        Duration::from_secs(3),
    )
    .unwrap_err();
    server.join().unwrap();
    assert!(error.message.starts_with(OUTCOME_UNKNOWN));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn body_phase_disconnect_is_unknown_and_does_not_open_a_replay_connection() {
    let (listener, url) = listener();
    let server = std::thread::spawn(move || {
        let mut stream = accept(&listener);
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            head.push(byte[0]);
            assert!(head.len() < 4096);
        }
        drop(stream);
        listener
    });
    let error = request_json(
        &url,
        None,
        "POST",
        "/test",
        Some(json!("x".repeat(MAX_INLINE_JSON_BYTES - 2))),
    )
    .unwrap_err();
    let listener = server.join().unwrap();
    assert!(error.message.starts_with(OUTCOME_UNKNOWN));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}

#[test]
fn lost_unicode_write_ack_stops_following_steps_without_replaying() {
    let payload = json!({"name":"request-body-fixture","description":"中文\nالعربية\"\\"});
    let ((report, calls), wire) = observe(b"", |url| run(url, "project_create", payload));
    assert_eq!(report.status, "failed");
    assert_eq!(calls, ["project_create"]);
    assert_eq!(report.executed_step_count, 0);
    assert!(
        report
            .execution_summary
            .failure
            .unwrap()
            .message
            .starts_with(OUTCOME_UNKNOWN)
    );
    let (_, body) = wire.split_once("\r\n\r\n").unwrap();
    let received: Value = serde_json::from_str(body).unwrap();
    assert_eq!(received["description"], "中文\nالعربية\"\\");
}

#[test]
fn vectored_sender_handles_empty_segments_and_preserves_original_body_buffer() {
    let body = vec![b'x'; 256 * 1024];
    let pointer = body.as_ptr();
    let length = body.len();
    let (listener, url) = listener();
    let server = std::thread::spawn(move || {
        let mut stream = accept(&listener);
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut actual = vec![0; length + 5];
        stream.read_exact(&mut actual).unwrap();
        assert_eq!(&actual[..2], b"ab");
        assert!(actual[2..length + 2].iter().all(|byte| *byte == b'x'));
        assert_eq!(&actual[length + 2..], b"end");
    });
    let mut stream = TcpStream::connect(url.strip_prefix("http://").unwrap()).unwrap();
    write_parts_before_deadline(
        &mut stream,
        &mut [
            io::IoSlice::new(b""),
            io::IoSlice::new(b"ab"),
            io::IoSlice::new(b""),
            io::IoSlice::new(&body),
            io::IoSlice::new(b"end"),
            io::IoSlice::new(b""),
        ],
        Instant::now() + Duration::from_secs(3),
        Duration::from_secs(3),
    )
    .unwrap();
    server.join().unwrap();
    assert_eq!(body.as_ptr(), pointer);
    assert_eq!(body.len(), length);
    assert!(body.iter().all(|byte| *byte == b'x'));
}

#[test]
fn expired_vectored_write_sends_nothing_and_empty_parts_do_not_bypass_deadline() {
    let (listener, url) = listener();
    let server = std::thread::spawn(move || {
        let mut stream = accept(&listener);
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut byte = [0];
        assert_eq!(stream.read(&mut byte).unwrap(), 0);
    });
    let mut stream = TcpStream::connect(url.strip_prefix("http://").unwrap()).unwrap();
    for body in [b"".as_slice(), b"not-sent".as_slice()] {
        let error = write_parts_before_deadline(
            &mut stream,
            &mut [io::IoSlice::new(body)],
            Instant::now() - Duration::from_millis(1),
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(error.message.contains("deadline exhausted"));
    }
    drop(stream);
    server.join().unwrap();
}
