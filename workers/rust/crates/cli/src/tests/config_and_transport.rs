use super::*;

#[test]
fn formats_events_for_machine_consumption() {
    let mut event = ProgressEvent::new("job-1", JobStatus::Solving, 0.5);
    event.iteration = Some(2);
    event.residual = Some(0.125);
    event.peak_memory = Some(576);
    event.message = Some("mock solve step 2/4".to_string());

    assert_eq!(
        format_event(&event),
        "event|job-1|solving|0.50|2|0.125|576|mock solve step 2/4"
    );
}

#[test]
fn preserves_worker_defaults_when_no_args_are_given() {
    let config = WorkerConfig {
        job_id: "job-local-1".to_string(),
        project_id: "project-local-1".to_string(),
        case_id: "case-local-1".to_string(),
        steps: 5,
    };

    assert_eq!(config.steps, 5);
    assert_eq!(config.job_id, "job-local-1");
}

#[test]
fn parses_agent_command_defaults() {
    let command = Command::Agent(AgentConfig::from_args(&[]));

    assert_eq!(
        command,
        Command::Agent(AgentConfig {
            host: "127.0.0.1".to_string(),
            port: 5001,
            agent_id: None,
            advertise_host: None,
            orchestrator_url: None,
            cluster_api_token: None,
            agent_fingerprint: None,
            certificate_id: None,
            cert_path: None,
            key_path: None,
            ca_cert_path: None,
            register_interval_ms: 5_000,
            cluster_id: None,
            peers: vec![],
        })
    );
}

#[test]
fn parses_peer_mesh_agent_args() {
    let config = AgentConfig::from_args(&[
        "--cluster-id".to_string(),
        "lan-lab-a".to_string(),
        "--peer".to_string(),
        "10.0.0.10:5001".to_string(),
        "--peer".to_string(),
        "10.0.0.11:5001".to_string(),
    ]);

    assert_eq!(config.cluster_id.as_deref(), Some("lan-lab-a"));
    assert_eq!(config.peers.len(), 2);
}

#[test]
fn normalizes_and_filters_peer_addresses() {
    let peers = normalize_peer_addresses(vec![
        " 10.0.0.11:5001 ".to_string(),
        "10.0.0.10:5001".to_string(),
        "10.0.0.11:5001".to_string(),
    ]);

    assert_eq!(
        peers,
        vec!["10.0.0.10:5001".to_string(), "10.0.0.11:5001".to_string()]
    );

    let filtered = filter_self_peers(
        peers,
        &["10.0.0.10:5001".to_string(), "127.0.0.1:5001".to_string()],
    );

    assert_eq!(filtered, vec!["10.0.0.11:5001".to_string()]);
}

#[test]
fn parses_http_url_for_remote_registration() {
    let parsed = parse_http_url("http://orchestrator.example.com:4000/api/v1/agents/register")
        .expect("parsed URL");
    assert_eq!(parsed.host, "orchestrator.example.com");
    assert_eq!(parsed.port, 4000);
    assert_eq!(parsed.path, "/api/v1/agents/register");
}
