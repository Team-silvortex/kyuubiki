#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use kyuubiki_agent_transport::ClientSecurity;
use serde_json::json;
use std::error::Error;
use support::*;

const TOKEN: &str = "test-only-agent-token-0123456789abcdef";

#[test]
fn incomplete_api_configuration_stops_instead_of_starting_plaintext() -> Result<(), Box<dyn Error>>
{
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_kyuubiki-cli"))
        .args(["agent", "--host", "127.0.0.1", "--port", "0"])
        .env("KYUUBIKI_AGENT_API_TOKEN", TOKEN)
        .env_remove("KYUUBIKI_AGENT_TLS_CERT_FILE")
        .env_remove("KYUUBIKI_AGENT_TLS_KEY_FILE")
        .env_remove("KYUUBIKI_AGENT_ALLOWED_PEER_IPS")
        .env_remove("KYUUBIKI_AGENT_TRUSTED_LAN_CONTROL")
        .env_remove("KYUUBIKI_AGENT_PEERS")
        .output()?;
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr)?;
    assert!(error.contains("requires token, TLS certificate and private key together"));
    assert!(!error.contains(TOKEN));
    Ok(())
}

#[test]
fn tls_agent_enforces_auth_and_preserves_real_execution_and_recovery() -> Result<(), Box<dyn Error>>
{
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let pem = cert.cert.pem();
    let key = cert.signing_key.serialize_pem();
    let mut agent = LiveAgent::start_with_durable_store()?;
    agent.restart_with_api_security(&pem, &key, TOKEN)?;
    let descriptor = agent.request("tls-describe", "describe_agent", json!({}))?;
    let access = &successful_result(&descriptor, "tls-describe")["connection_access"];
    assert_eq!(access["authenticated"], true);
    assert_eq!(access["encrypted"], true);
    assert_eq!(access["control_mode"], "tls_api_token");

    let wrong = ClientSecurity::from_pem(
        &pem,
        "localhost",
        "wrong-token-012345678901234567890123".into(),
    )?;
    assert!(
        rpc_request_secured(
            agent.port(),
            "bad-token",
            "submit_durable_agent_task",
            task('a', "never"),
            Some(&wrong)
        )
        .is_err()
    );
    assert!(
        rpc_request(
            agent.port(),
            "plaintext",
            "solve_bar_1d",
            solver_params("never")
        )
        .is_err()
    );
    assert_eq!(
        lifecycle(&agent, "no-dispatch")?["active_execution_count"],
        0
    );
    assert_eq!(fetch(&agent, 'a')?["status"], "not_found");

    let reply = agent.request(
        "secured-run",
        "submit_durable_agent_task",
        task('b', "secured-bar"),
    )?;
    assert_eq!(
        successful_result(&reply, "secured-run")["status"],
        "running"
    );
    let receipt = wait_status(&agent, 'b', "completed")?;
    assert_eq!(receipt["response"]["result"]["max_stress"], 10.0);
    let rotated = "rotated-token-0123456789abcdef01234567";
    agent.restart_with_api_security(&pem, &key, rotated)?;
    let previous = ClientSecurity::from_pem(&pem, "localhost", TOKEN.into())?;
    assert!(
        rpc_request_secured(
            agent.port(),
            "old-token",
            "fetch_durable_agent_execution",
            json!({"execution_id":"b".repeat(32)}),
            Some(&previous)
        )
        .is_err()
    );
    assert_eq!(fetch(&agent, 'b')?, receipt);
    let logs = std::fs::read_to_string(&agent.log_path)?;
    assert!(!logs.contains(TOKEN) && !logs.contains(rotated) && !logs.contains("PRIVATE KEY"));
    Ok(())
}
