use super::*;
use std::io::Write;
use std::net::TcpListener;
use std::time::Instant;

#[test]
fn peer_descriptor_requires_the_exact_rpc_reply_and_valid_envelope() {
    for case in ["wrong_id", "wrong_version", "contradictory"] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request: RpcRequest = serde_json::from_slice(
                &read_frame(&mut stream)
                    .map_err(frame_error_message)
                    .unwrap(),
            )
            .unwrap();
            let mut response = RpcResponse::success(
                request.id,
                serde_json::to_value(AgentDescriptor::solver_agent_default()).unwrap(),
            );
            match case {
                "wrong_id" => response.id = "unrelated-request".into(),
                "wrong_version" => response.rpc_version = 0,
                _ => response.error = RpcResponse::error("unused", "failure", "not success").error,
            }
            write_frame(&mut stream, &serde_json::to_vec(&response).unwrap()).unwrap();
        });
        let response = request_agent_descriptor(&address);
        server.join().unwrap();
        assert!(response.is_err(), "accepted {case}");
    }
}

fn config(peers: &[&str]) -> AgentConfig {
    let mut args = vec!["--cluster-id".into(), "research-mesh".into()];
    for peer in peers {
        args.extend(["--peer".into(), (*peer).into()]);
    }
    AgentConfig::from_args(&args)
}

#[test]
fn mesh_rejects_mixed_authorities_and_unbounded_or_invalid_seeds() {
    let mut cfg = config(&["127.0.0.1:5002"]);
    assert!(validate_mesh_config(&cfg).is_ok());
    cfg.orchestrator_url = Some("http://127.0.0.1:4000".into());
    assert!(
        validate_mesh_config(&cfg)
            .unwrap_err()
            .contains("mutually exclusive")
    );
    cfg.orchestrator_url = None;
    cfg.peers = vec!["127.0.0.1:5002".into(); MAX_PEERS + 1];
    assert!(validate_mesh_config(&cfg).is_err());
    for peer in [
        "bad",
        "127.0.0.1:0",
        "0.0.0.0:5002",
        "[::]:5002",
        "[::ffff:0.0.0.0]:5002",
        "[::ffff:224.0.0.1]:5002",
        "[::ffff:255.255.255.255]:5002",
        "224.0.0.1:5002",
        "255.255.255.255:5002",
        "http://localhost:5002",
        "x/y:5002",
    ] {
        cfg.peers = vec![peer.into()];
        assert!(validate_mesh_config(&cfg).is_err(), "accepted {peer}");
    }
    cfg.peers = vec!["127.0.0.1:5002".into()];
    for cluster in [" ", "research\nmesh"] {
        cfg.cluster_id = Some(cluster.into());
        assert!(validate_mesh_config(&cfg).is_err());
    }
}

#[test]
fn only_same_cluster_offline_mesh_descriptors_are_compatible() {
    let descriptor = crate::agent_state::build_agent_descriptor(&config(&["127.0.0.1:5002"]));
    assert!(compatible_peer(&descriptor, Some("research-mesh")));
    assert!(!compatible_peer(&descriptor, Some("other-mesh")));
    assert!(!compatible_peer(&descriptor, None));
    for case in [
        "mode",
        "authority",
        "control",
        "binding",
        "session",
        "multi",
        "peers",
    ] {
        let mut invalid = descriptor.clone();
        match case {
            "mode" => invalid.runtime.runtime_mode = "orchestrated".into(),
            "authority" => invalid.authority.authority_mode = "orchestrated".into(),
            "control" => invalid.authority.control_mode = "standalone".into(),
            "binding" => invalid.authority.orchestrator_id = Some("orch".into()),
            "session" => invalid.authority.orchestrator_session_id = Some("old-session".into()),
            "multi" => invalid.authority.accepts_multi_orchestrator_binding = true,
            _ => invalid.runtime.peers = vec![hint("127.0.0.1:5002"); MAX_PEERS + 1],
        }
        assert!(
            !compatible_peer(&invalid, Some("research-mesh")),
            "accepted {case}"
        );
    }
    let mut unnamed = descriptor;
    unnamed.runtime.cluster_id = None;
    assert!(compatible_peer(&unnamed, None));
    assert!(!compatible_peer(&unnamed, Some("research-mesh")));
}

fn hint(address: &str) -> ClusterPeerDescriptor {
    ClusterPeerDescriptor {
        address: address.into(),
        status: "healthy".into(),
        failure_count: 0,
        last_seen_unix_s: Some(u64::MAX),
    }
}

#[test]
fn peer_hints_are_bounded_canonical_candidates_not_health_evidence() {
    let own = vec!["127.0.0.1:5001".into()];
    let mut peers = CandidatePool::new(vec!["127.0.0.1:5002".into()], own);
    let now = Instant::now();
    peers.merge_hints(
        vec![
            hint("127.0.0.1:5001"),
            hint("127.0.0.1:5002"),
            hint(" Lab.LOCAL:05003 "),
            hint("lab.local:5003"),
            hint("bad-url"),
        ],
        now,
    );
    assert_eq!(
        peers
            .finish_round(now)
            .iter()
            .map(|p| p.address.as_str())
            .collect::<Vec<_>>(),
        ["127.0.0.1:5002", "lab.local:5003"]
    );
    peers.merge_hints(
        (6000..6200)
            .map(|port| hint(&format!("127.0.0.1:{port}")))
            .collect(),
        now,
    );
    let descriptors = peers.finish_round(now);
    assert_eq!(descriptors.len(), MAX_PEERS);
    assert!(
        descriptors
            .iter()
            .all(|peer| peer.status == "unreachable" && peer.last_seen_unix_s.is_none())
    );
    assert_eq!(compute_cluster_health_score(&descriptors), 0);
}

#[test]
fn bounded_probe_rounds_eventually_cover_all_candidates() {
    let peers: Vec<_> = (0..MAX_PEERS)
        .map(|index| format!("127.0.0.1:{}", 5000 + index))
        .collect();
    let mut candidates = CandidatePool::new(peers.clone(), vec![]);
    let mut visited = Vec::new();
    for _ in 0..MAX_PEERS / PROBES_PER_ROUND {
        let round = candidates.next_round();
        assert_eq!(round.len(), PROBES_PER_ROUND);
        visited.extend(round);
    }
    assert_eq!(visited, peers);
    assert_eq!(
        CandidatePool::new(peers[..2].to_vec(), vec![])
            .next_round()
            .len(),
        2
    );
    assert!(CandidatePool::new(vec![], vec![]).next_round().is_empty());
}

#[test]
fn self_filter_preserves_mesh_mode_with_only_ipv6_self_seed() {
    let mut cfg = config(&["[::1]:5001", "[0:0:0:0:0:0:0:1]:5001"]);
    cfg.host = "::1".into();
    cfg.port = 5001;
    cfg.advertise_host = None;
    assert!(configured_peers(&cfg).is_empty());
    let descriptor = crate::agent_state::build_agent_descriptor(&cfg);
    assert_eq!(descriptor.runtime.runtime_mode, "peer_mesh");
    assert_eq!(descriptor.authority.authority_mode, "offline_mesh");
    assert_eq!(descriptor.runtime.cluster_size, 1);
}

#[test]
fn peer_frames_are_rejected_before_body_allocation() {
    for size in [0, (peer_rpc::MAX_FRAME_BYTES + 1) as u32, u32::MAX] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            read_frame(&mut stream)
                .map_err(frame_error_message)
                .unwrap();
            stream.write_all(&size.to_be_bytes()).unwrap();
        });
        let error = request_agent_descriptor(&address).unwrap_err();
        server.join().unwrap();
        assert!(error.contains("frame exceeds limit"), "{error}");
    }
}

#[test]
fn partial_reply_does_not_renew_the_total_peer_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        read_frame(&mut stream)
            .map_err(frame_error_message)
            .unwrap();
        stream.write_all(&1024_u32.to_be_bytes()).unwrap();
        for _ in 0..30 {
            if stream.write_all(b" ").is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(40));
        }
    });
    let started = Instant::now();
    let response = peer_rpc::request_with_timeout(&address, Duration::from_millis(180));
    let elapsed = started.elapsed();
    server.join().unwrap();
    assert!(response.is_err());
    assert!(
        elapsed < Duration::from_millis(800),
        "deadline renewed: {elapsed:?}"
    );
}
