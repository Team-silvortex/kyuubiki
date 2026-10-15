use super::*;
use std::io::Write;
use std::net::TcpListener;
use std::sync::{Barrier, mpsc};
use std::thread;

fn ingress(timeout_ms: usize) -> Ingress {
    Ingress::new(Limits {
        frame_bytes: 32,
        in_flight_bytes: 48,
        timeout_ms,
        connections: 2,
    })
}

fn pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (server, _) = listener.accept().unwrap();
    server.set_nonblocking(false).unwrap();
    (client, server)
}

fn send(client: &mut TcpStream, payload: &[u8]) {
    client
        .write_all(&(payload.len() as u32).to_be_bytes())
        .unwrap();
    client.write_all(payload).unwrap();
}

#[test]
fn ingress_configuration_is_bounded_and_never_silently_disabled() {
    let defaults = Limits::parse(|_| Ok(None)).unwrap();
    assert_eq!(defaults.frame_bytes, 256 * MIB);
    assert_eq!(defaults.in_flight_bytes, 512 * MIB);
    assert_eq!(defaults.connections, 64);
    for name in ENV_NAMES {
        for invalid in ["", "0", "-1", "1.5", "unlimited", "18446744073709551616"] {
            assert!(Limits::parse(|key| Ok((key == name).then(|| invalid.into()))).is_err());
        }
    }
    for (name, max) in ENV_NAMES
        .into_iter()
        .zip([512 * MIB, 1024 * MIB, 300_000, 256])
    {
        assert!(Limits::parse(|key| Ok((key == name).then(|| (max + 1).to_string()))).is_err());
    }
    assert!(Limits::parse(|_| Err("invalid environment encoding".into())).is_err());
    assert!(Limits::parse(|key| Ok((key == ENV_NAMES[1]).then(|| "1".into()))).is_err());
}

#[test]
fn empty_or_oversized_headers_reject_before_payload_reservation() {
    let ingress = ingress(1000);
    for length in [0_u32, 33, u32::MAX] {
        let (mut client, mut server) = pair();
        client.write_all(&length.to_be_bytes()).unwrap();
        let error = ingress.read_request(&mut server).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(ingress.request_bytes.load(Ordering::Acquire), 0);
    }
}

#[test]
fn frame_capacity_remains_reserved_until_dispatch_owner_drops_it() {
    let ingress = ingress(1000);
    let (mut client, mut server) = pair();
    send(&mut client, &[1; 32]);
    let first = ingress.read_request(&mut server).unwrap();
    assert_eq!(first.payload, vec![1; 32]);
    assert_eq!(ingress.snapshot()["reserved_request_bytes"], 32);
    send(&mut client, &[2; 17]);
    let recovery = ingress.read_request(&mut server).unwrap();
    assert!(recovery.recovery_only);
    assert_eq!(ingress.snapshot()["general_reserved_request_bytes"], 32);
    assert_eq!(ingress.snapshot()["reserved_request_bytes"], 49);
    assert_eq!(ingress.snapshot()["recovery_reserve"]["active_requests"], 1);
    drop(recovery);
    drop(first);
    assert_eq!(ingress.snapshot()["reserved_request_bytes"], 0);
    let (mut client, mut server) = pair();
    send(&mut client, &[2; 32]);
    assert_eq!(
        ingress.read_request(&mut server).unwrap().payload,
        vec![2; 32]
    );
    assert_eq!(ingress.snapshot()["reserved_request_bytes"], 0);
}

#[test]
fn truncated_payload_and_unwind_release_the_request_budget() {
    let ingress = ingress(1000);
    let (mut client, mut server) = pair();
    client.write_all(&32_u32.to_be_bytes()).unwrap();
    client.write_all(&[1; 3]).unwrap();
    drop(client);
    assert_eq!(
        ingress.read_request(&mut server).err().unwrap().kind(),
        io::ErrorKind::UnexpectedEof
    );
    assert_eq!(ingress.snapshot()["reserved_request_bytes"], 0);
    let (mut client, mut server) = pair();
    send(&mut client, &[1; 32]);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _frame = ingress.read_request(&mut server).unwrap();
        panic!("injected request-owner unwind");
    }));
    assert!(result.is_err());
    assert_eq!(ingress.snapshot()["reserved_request_bytes"], 0);
}

#[test]
fn idle_partial_header_and_trickled_body_share_a_total_deadline() {
    for prefix in [vec![], vec![0], vec![0, 0, 0, 32, 1]] {
        let ingress = ingress(120);
        let (mut client, mut server) = pair();
        let (stop, stopped) = mpsc::channel();
        let writer = thread::spawn(move || {
            client.write_all(&prefix).unwrap();
            // No fragment gap reaches the request timeout, but the whole exchange does.
            for _ in 0..10 {
                if stopped.recv_timeout(Duration::from_millis(50)).is_ok() {
                    break;
                }
                if !prefix.is_empty() && client.write_all(&[0]).is_err() {
                    break;
                }
            }
        });
        let started = Instant::now();
        let error = ingress.read_request(&mut server).err().unwrap();
        assert!(
            matches!(
                error.kind(),
                io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
            ),
            "{error}"
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(ingress.snapshot()["reserved_request_bytes"], 0);
        let _ = stop.send(());
        writer.join().unwrap();
    }
}

#[test]
fn persistent_connection_gets_a_fresh_receive_budget_only_after_each_complete_request() {
    let ingress = ingress(80);
    let (mut client, mut server) = pair();
    send(&mut client, b"first");
    let first = ingress.read_request(&mut server).unwrap();
    // Execution/reply time is deliberately not an ingress deadline.
    thread::sleep(Duration::from_millis(100));
    drop(first);
    send(&mut client, b"second");
    assert_eq!(
        ingress.read_request(&mut server).unwrap().payload,
        b"second"
    );
}

#[test]
fn connection_and_byte_permits_release_on_scope_exit_and_failed_spawn_closure_drop() {
    let ingress = ingress(1000);
    let first = ingress.admit_connection().unwrap();
    let second = ingress.admit_connection().unwrap();
    assert!(!first.recovery_only && !second.recovery_only);
    let reserves = (0..recovery::CONNECTIONS)
        .map(|_| ingress.admit_connection().unwrap())
        .collect::<Vec<_>>();
    assert!(reserves.iter().all(|permit| permit.recovery_only));
    assert!(ingress.admit_connection().is_none());
    let never_started = move || drop(second);
    drop(never_started);
    assert!(ingress.admit_connection().is_some());
    drop(first);
    drop(reserves);
    assert_eq!(ingress.snapshot()["active_connections"], 0);
    let counter = Arc::new(AtomicUsize::new(usize::MAX - 1));
    assert!(Permit::acquire(&counter, 2, usize::MAX).is_none());
    assert_eq!(counter.load(Ordering::Acquire), usize::MAX - 1);
}

#[test]
fn concurrent_admission_cannot_exceed_the_connection_limit() {
    let ingress = ingress(1000);
    let acquired = AtomicUsize::new(0);
    let barrier = Barrier::new(12);
    thread::scope(|scope| {
        for _ in 0..12 {
            scope.spawn(|| {
                let permit = ingress.admit_connection();
                if permit.is_some() {
                    acquired.fetch_add(1, Ordering::SeqCst);
                }
                barrier.wait();
                assert_eq!(ingress.connections.load(Ordering::Acquire), 2);
                assert_eq!(ingress.recovery.active_connections(), recovery::CONNECTIONS);
                barrier.wait();
                drop(permit);
            });
        }
    });
    assert_eq!(acquired.load(Ordering::Acquire), 2 + recovery::CONNECTIONS);
    assert_eq!(ingress.snapshot()["active_connections"], 0);
}

#[test]
fn recovery_requests_have_independent_count_and_byte_limits_and_release_on_unwind() {
    let ingress = ingress(1000);
    let (mut client, mut server) = pair();
    send(&mut client, &[0; 32]);
    let first = ingress.read_request(&mut server).unwrap();
    send(&mut client, &[0; 16]);
    let second = ingress.read_request(&mut server).unwrap();
    let mut reserved = Vec::new();
    for _ in 0..recovery::REQUESTS {
        send(&mut client, &[0; 32]);
        let frame = ingress.read_request(&mut server).unwrap();
        assert!(frame.recovery_only);
        reserved.push(frame);
    }
    client.write_all(&1_u32.to_be_bytes()).unwrap();
    assert_eq!(
        ingress.read_request(&mut server).err().unwrap().to_string(),
        "agent_request_byte_capacity"
    );
    assert_eq!(ingress.snapshot()["general_reserved_request_bytes"], 48);
    assert_eq!(
        ingress.snapshot()["recovery_reserve"]["reserved_request_bytes"],
        128
    );
    assert_eq!(ingress.snapshot()["reserved_request_bytes"], 176);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _frames = reserved;
        panic!("recovery dispatch-owner unwind");
    }));
    assert!(result.is_err());
    assert_eq!(ingress.snapshot()["recovery_reserve"]["active_requests"], 0);
    assert_eq!(
        ingress.snapshot()["recovery_reserve"]["reserved_request_bytes"],
        0
    );
    drop((first, second));
    send(&mut client, b"normal");
    assert!(!ingress.read_request(&mut server).unwrap().recovery_only);
}

#[test]
fn reserved_connections_never_promote_and_recovery_frames_keep_their_size_bound() {
    let ingress = Ingress::new(Limits {
        frame_bytes: 64 * 1024,
        in_flight_bytes: 128 * 1024,
        ..Limits::default()
    });
    let general = (0..ingress.limits.connections)
        .map(|_| ingress.admit_connection().unwrap())
        .collect::<Vec<_>>();
    let connection = ingress.admit_connection().unwrap();
    assert!(connection.recovery_only);
    drop(general);
    assert!(connection.recovery_only);
    let (mut client, mut server) = pair();
    client
        .write_all(&((recovery::FRAME_BYTES + 1) as u32).to_be_bytes())
        .unwrap();
    assert_eq!(
        ingress
            .read_request_in(&mut server, connection.recovery_only)
            .err()
            .unwrap()
            .to_string(),
        "agent_request_byte_capacity"
    );
    assert_eq!(ingress.snapshot()["reserved_request_bytes"], 0);
    send(&mut client, b"control");
    assert!(
        ingress
            .read_request_in(&mut server, connection.recovery_only)
            .unwrap()
            .recovery_only
    );
}

#[test]
fn recovery_receive_deadline_is_shorter_and_returns_partial_reservations() {
    let ingress = ingress(5000);
    let (mut client, mut server) = pair();
    client.write_all(&32_u32.to_be_bytes()).unwrap();
    client.write_all(b"partial").unwrap();
    let started = Instant::now();
    let error = ingress.read_request_in(&mut server, true).err().unwrap();
    assert!(matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
    ));
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(ingress.snapshot()["recovery_reserve"]["active_requests"], 0);
    assert_eq!(ingress.snapshot()["reserved_request_bytes"], 0);
}

#[test]
fn recovery_method_allowlist_excludes_every_advertised_execution_entrypoint() {
    use kyuubiki_protocol::{AgentDescriptor, RpcMethod};
    let expected = [
        RpcMethod::Ping,
        RpcMethod::DescribeAgent,
        RpcMethod::DescribeAgentLifecycle,
        RpcMethod::BeginAgentDrain,
        RpcMethod::ResumeAgentAdmission,
        RpcMethod::FetchDurableAgentExecution,
        RpcMethod::ListDurableAgentExecutions,
        RpcMethod::CancelDurableAgentExecution,
        RpcMethod::FetchOperatorTaskResult,
        RpcMethod::CancelExecution,
        RpcMethod::CancelJob,
    ];
    let descriptor = AgentDescriptor::solver_agent_default();
    for method in descriptor.protocol.methods {
        assert_eq!(
            allows_recovery_method(&method),
            expected.contains(&method),
            "{method:?}"
        );
    }
    let ingress = ingress(5000);
    assert_eq!(
        ingress.snapshot()["recovery_reserve"]["allowed_methods"],
        serde_json::json!(expected)
    );
    assert_eq!(ingress.snapshot()["max_total_connections"], 6);
    assert_eq!(ingress.snapshot()["max_total_in_flight_request_bytes"], 176);
}
