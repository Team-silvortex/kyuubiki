#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;

use serde_json::{Value, json};
use std::error::Error;
use support::*;

fn policy(agent: &LiveAgent) -> Result<Value, Box<dyn Error>> {
    let reply = agent.request("network-policy", "describe_agent", json!({}))?;
    Ok(successful_result(&reply, "network-policy")["request_ingress"]["network_access"].clone())
}

#[test]
fn native_default_is_local_only_and_preserves_real_durable_execution() -> Result<(), Box<dyn Error>>
{
    let agent = LiveAgent::start_with_durable_store()?;
    let access = policy(&agent)?;
    assert_eq!(access["mode"], "loopback_only");
    assert_eq!(access["allowed_peer_ips"], json!([]));
    assert_eq!(access["authenticated"], false);
    assert_eq!(access["encrypted"], false);
    assert_eq!(access["lan_control"]["mode"], "discovery_only");
    let reply = agent.request(
        "local-task",
        "submit_durable_agent_task",
        task('a', "local-bar"),
    )?;
    assert_eq!(successful_result(&reply, "local-task")["status"], "running");
    assert_eq!(
        wait_status(&agent, 'a', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    Ok(())
}

#[test]
fn explicit_network_policy_is_visible_and_restart_does_not_replay_receipts()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    let reply = agent.request(
        "before-policy",
        "submit_durable_agent_task",
        task('b', "saved-bar"),
    )?;
    assert_eq!(
        successful_result(&reply, "before-policy")["status"],
        "running"
    );
    let retained = wait_status(&agent, 'b', "completed")?;
    agent.restart_with_allowed_peer_ips(Some("192.0.2.1, ::ffff:192.0.2.1, 2001:db8::1"))?;
    let access = policy(&agent)?;
    assert_eq!(access["mode"], "loopback_and_explicit_peer_ips");
    assert_eq!(
        access["allowed_peer_ips"],
        json!(["192.0.2.1", "2001:db8::1"])
    );
    assert_eq!(access["authority_binding_verified"], false);
    assert_eq!(fetch(&agent, 'b')?, retained);
    assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
    agent.restart_with_allowed_peer_ips(None)?;
    assert_eq!(policy(&agent)?["mode"], "loopback_only");
    assert_eq!(fetch(&agent, 'b')?, retained);
    Ok(())
}

#[test]
fn invalid_network_configuration_stops_startup_but_preserves_original_store()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    let reply = agent.request("saved", "submit_durable_agent_task", task('c', "saved-bar"))?;
    assert_eq!(successful_result(&reply, "saved")["status"], "running");
    let retained = wait_status(&agent, 'c', "completed")?;
    for invalid in ["", "*", "0.0.0.0", "192.0.2.0/24", "secret.example.invalid"] {
        let error = agent
            .restart_with_allowed_peer_ips(Some(invalid))
            .unwrap_err()
            .to_string();
        assert!(error.contains("KYUUBIKI_AGENT_ALLOWED_PEER_IPS"));
        assert!(!error.contains("secret.example.invalid"));
    }
    agent.restart_with_allowed_peer_ips(None)?;
    assert_eq!(fetch(&agent, 'c')?, retained);
    let response = agent.request("after-invalid", "solve_bar_1d", solver_params("healthy"))?;
    assert_eq!(
        successful_result(&response, "after-invalid")["max_stress"],
        10.0
    );
    Ok(())
}

#[test]
fn lan_control_configuration_is_strict_and_keeps_original_local_receipts()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    let response = agent.request(
        "before-trust",
        "submit_durable_agent_task",
        task('d', "saved"),
    )?;
    assert_eq!(
        successful_result(&response, "before-trust")["status"],
        "running"
    );
    let retained = wait_status(&agent, 'd', "completed")?;
    for invalid in ["", "TRUE", "1", "secret-config-value"] {
        let error = agent
            .restart_with_trusted_lan_control(Some(invalid))
            .unwrap_err()
            .to_string();
        assert!(error.contains("KYUUBIKI_AGENT_TRUSTED_LAN_CONTROL"));
        assert!(!error.contains("secret-config-value"));
    }
    for (value, mode) in [
        (Some("true"), "trusted_lan_control"),
        (Some("false"), "discovery_only"),
        (None, "discovery_only"),
    ] {
        agent.restart_with_trusted_lan_control(value)?;
        let access = policy(&agent)?;
        assert_eq!(access["lan_control"]["mode"], mode);
        assert_eq!(access["mode"], "loopback_only");
        assert_eq!(access["authenticated"], false);
        assert_eq!(fetch(&agent, 'd')?, retained);
        assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
    }
    Ok(())
}

#[test]
#[ignore = "requires an explicit local IPv4 interface in KYUUBIKI_TEST_AGENT_LAN_IP"]
fn real_non_loopback_socket_is_denied_then_explicitly_admitted_and_revoked()
-> Result<(), Box<dyn Error>> {
    use std::io::{Read, Write};
    use std::net::{Ipv4Addr, TcpListener, TcpStream};
    use std::time::Duration;

    let ip: Ipv4Addr = std::env::var("KYUUBIKI_TEST_AGENT_LAN_IP")?.parse()?;
    assert!(!ip.is_loopback() && !ip.is_unspecified() && !ip.is_multicast() && !ip.is_broadcast());
    // Refuse non-local destinations. Never probe another machine to run this test.
    drop(TcpListener::bind((ip, 0))?);
    let mut agent = LiveAgent::start_with_durable_store()?;
    agent.restart_with_wildcard_bind()?;
    let connect = |agent: &LiveAgent| -> Result<TcpStream, Box<dyn Error>> {
        let stream = TcpStream::connect((ip, agent.port()))?;
        assert_eq!(stream.local_addr()?.ip(), std::net::IpAddr::V4(ip));
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        Ok(stream)
    };
    let denied = |agent: &LiveAgent| -> Result<(), Box<dyn Error>> {
        let mut stream = connect(agent)?;
        // No header is sent: rejection must happen before frame parsing and its timeout.
        match stream.read(&mut [0; 1]) {
            Ok(0) => Ok(()),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                ) =>
            {
                Ok(())
            }
            other => Err(format!(
                "non-loopback peer was not denied before frame: {other:?}; policy={}",
                policy(agent)?
            )
            .into()),
        }
    };
    for _ in 0..8 {
        denied(&agent)?;
    }
    assert!(policy(&agent)?["denied_connections"].as_u64().unwrap() >= 8);

    agent.restart_with_allowed_peer_ips(Some(&ip.to_string()))?;
    let mut stream = connect(&agent)?;
    let request = |stream: &mut TcpStream,
                   id: &str,
                   method: &str,
                   params: Value|
     -> Result<Value, Box<dyn Error>> {
        let payload = serde_json::to_vec(
            &json!({"rpc_version":1, "id":id, "method":method, "params":params}),
        )?;
        stream.write_all(&u32::try_from(payload.len())?.to_be_bytes())?;
        stream.write_all(&payload)?;
        for _ in 0..16 {
            let reply = read_json_frame(stream)?;
            assert_eq!(reply["id"], id);
            if reply.get("ok").is_some() {
                return Ok(reply);
            }
        }
        Err("too many progress frames".into())
    };
    let response = request(&mut stream, "discovery", "describe_agent", json!({}))?;
    let access = &successful_result(&response, "discovery")["connection_access"];
    assert_eq!(access["peer_scope"], "non_loopback");
    assert_eq!(access["control_allowed"], false);
    let local = agent.request("local-access", "describe_agent", json!({}))?;
    assert_eq!(
        successful_result(&local, "local-access")["connection_access"]["control_allowed"],
        true
    );
    assert_eq!(
        successful_result(&response, "discovery")["request_ingress"]["network_access"]["lan_control"]
            ["mode"],
        "discovery_only"
    );
    let local = agent.request(
        "keep-receipt",
        "submit_durable_agent_task",
        task('e', "local-owned"),
    )?;
    assert_eq!(
        successful_result(&local, "keep-receipt")["status"],
        "running"
    );
    let retained = wait_status(&agent, 'e', "completed")?;
    let methods = kyuubiki_protocol::AgentDescriptor::solver_agent_default()
        .protocol
        .methods;
    for method in methods {
        use kyuubiki_protocol::RpcMethod;
        if matches!(
            method,
            RpcMethod::Ping
                | RpcMethod::DescribeAgent
                | RpcMethod::DescribeAgentLifecycle
                | RpcMethod::BeginAgentDrain
                | RpcMethod::ResumeAgentAdmission
        ) {
            continue;
        }
        let name = serde_json::to_value(method)?;
        let mut params = task('f', "must-not-run");
        params["store_id"] = retained["store_id"].clone();
        params["expected_store_id"] = retained["store_id"].clone();
        params["controller_id"] = json!("claimed-controller");
        params["authenticated"] = json!(true);
        let response = request(&mut stream, "blocked", name.as_str().unwrap(), params)?;
        assert_eq!(
            response["error"]["code"],
            "agent_lan_control_requires_explicit_trust"
        );
        assert_eq!(response["error"]["details"]["dispatched"], false);
        assert_eq!(
            response["error"]["details"]["automatic_replay_authorized"],
            false
        );
    }
    assert_eq!(fetch(&agent, 'e')?, retained);
    assert_eq!(fetch(&agent, 'f')?["status"], "not_found");
    assert_eq!(
        lifecycle(&agent, "no-admission")?["active_execution_count"],
        0
    );
    drop(stream);

    // Saturate the general pool: a recovery slot must not bypass LAN control policy.
    agent.restart_with_ingress_limits(65_536, 131_072, 5000, 1)?;
    let mut general = connect(&agent)?;
    assert_eq!(
        request(&mut general, "hold-general", "ping", json!({}))?["ok"],
        true
    );
    let mut reserve = connect(&agent)?;
    let response = request(
        &mut reserve,
        "reserved-recovery",
        "fetch_durable_agent_execution",
        json!({"execution_id":"e".repeat(32)}),
    )?;
    assert_eq!(
        response["error"]["code"],
        "agent_lan_control_requires_explicit_trust"
    );
    assert_eq!(
        request(&mut reserve, "reserved-ping", "ping", json!({}))?["ok"],
        true
    );
    drop((reserve, general));
    agent.restart_with_ingress_limits(65_536, 131_072, 5000, 4)?;
    agent.restart_with_trusted_lan_control(Some("true"))?;
    let mut stream = connect(&agent)?;
    let response = request(
        &mut stream,
        "trusted-discovery",
        "describe_agent",
        json!({}),
    )?;
    let access = &successful_result(&response, "trusted-discovery")["connection_access"];
    assert_eq!(access["control_allowed"], true);
    assert_eq!(access["control_mode"], "trusted_lan_control");
    assert_eq!(access["authenticated"], false);
    let response = request(
        &mut stream,
        "explicit-source",
        "solve_bar_1d",
        solver_params("network-bar"),
    )?;
    assert_eq!(
        successful_result(&response, "explicit-source")["max_stress"],
        10.0
    );
    let response = request(
        &mut stream,
        "not-local-admin",
        "begin_agent_drain",
        json!({"controller_id":"test-controller", "reason":"must not mutate"}),
    )?;
    assert_eq!(
        response["error"]["code"],
        "agent_lifecycle_control_requires_loopback"
    );
    assert_eq!(lifecycle(&agent, "still-accepting")?["state"], "accepting");

    let response = request(
        &mut stream,
        "trusted-submit",
        "submit_durable_agent_task",
        task('a', "lan-owned"),
    )?;
    assert_eq!(
        successful_result(&response, "trusted-submit")["status"],
        "running"
    );
    let lan_retained = wait_status(&agent, 'a', "completed")?;
    let response = request(
        &mut stream,
        "trusted-fetch",
        "fetch_durable_agent_execution",
        json!({"execution_id":"a".repeat(32)}),
    )?;
    assert_eq!(successful_result(&response, "trusted-fetch"), &lan_retained);

    agent.restart_with_trusted_lan_control(None)?;
    assert!(request(&mut stream, "old-connection", "ping", json!({})).is_err());
    drop(stream);
    let mut stream = connect(&agent)?;
    let response = request(
        &mut stream,
        "revoked-discovery",
        "describe_agent",
        json!({}),
    )?;
    assert_eq!(
        successful_result(&response, "revoked-discovery")["connection_access"]["control_allowed"],
        false
    );
    assert_eq!(
        request(&mut stream, "after-revoke", "ping", json!({}))?["ok"],
        true
    );
    let response = request(
        &mut stream,
        "revoked-fetch",
        "fetch_durable_agent_execution",
        json!({"execution_id":"a".repeat(32), "store_id":lan_retained["store_id"]}),
    )?;
    assert_eq!(
        response["error"]["code"],
        "agent_lan_control_requires_explicit_trust"
    );
    assert_eq!(fetch(&agent, 'a')?, lan_retained);
    assert_eq!(fetch(&agent, 'e')?, retained);
    drop(stream);

    agent.restart_with_allowed_peer_ips(None)?;
    denied(&agent)?;
    let response = agent.request(
        "local-after-revoke",
        "solve_bar_1d",
        solver_params("local-bar"),
    )?;
    assert_eq!(
        successful_result(&response, "local-after-revoke")["max_stress"],
        10.0
    );
    Ok(())
}
