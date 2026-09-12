use super::{
    agent_ports, apply_mode_env, development_agent_args, hot_runtime_state, runtime_env,
    service_start, service_status, service_stop,
};
use crate::runtime_layout::resolve_development_command;
use crate::{ServiceMode, workspace_root};

#[test]
fn local_mode_has_native_storage_defaults() {
    let mut env = std::collections::HashMap::new();
    apply_mode_env(&mut env, "local").expect("local mode");
    assert_eq!(env.get("KYUUBIKI_STORAGE_BACKEND").unwrap(), "sqlite");
}

#[test]
fn resolves_local_runtime_commands_without_node_launcher() {
    let root = workspace_root();
    for command in ["cargo", "mix", "npm"] {
        assert!(
            resolve_development_command(&root, command).is_ok(),
            "{command}"
        );
    }
}

#[test]
fn parses_default_agent_endpoints() {
    let root = workspace_root();
    let env = runtime_env(&root);
    assert!(!agent_ports(&root, &env).is_empty());
}

#[test]
fn release_agent_profile_is_explicit() {
    let env = [(
        "KYUUBIKI_AGENT_BUILD_PROFILE".to_string(),
        "release".to_string(),
    )]
    .into();
    let args = development_agent_args(5002, &env);
    assert!(args.iter().any(|arg| arg == "--release"));
    assert_eq!(args.last().map(String::as_str), Some("5002"));
}

#[test]
fn hot_runtime_marker_is_not_a_health_claim() {
    let healthy = "orchestrator: running\nfrontend: running\nagent[5001]: running";
    assert_eq!(hot_runtime_state(false, healthy), "stopped");
    assert_eq!(hot_runtime_state(true, healthy), "running");
    assert_eq!(
        hot_runtime_state(
            true,
            &healthy.replace("agent[5001]: running", "agent[5001]: blocked")
        ),
        "blocked"
    );
    assert_eq!(
        hot_runtime_state(
            true,
            &healthy.replace("frontend: running", "frontend: starting")
        ),
        "starting"
    );
    assert_eq!(
        hot_runtime_state(
            true,
            &healthy.replace("frontend: running", "frontend: stopped")
        ),
        "degraded"
    );
    assert_eq!(
        hot_runtime_state(
            true,
            "deployment-mode: distributed\norchestrator: running\nfrontend: disabled\nagent[5001]: stopped"
        ),
        "running"
    );
}

#[test]
#[ignore = "starts the real local control plane and agents"]
fn native_local_stack_round_trip() {
    let start = service_start(ServiceMode::Local);
    let status = service_status();
    let stop = service_stop();
    assert!(start.is_ok(), "start failed: {start:?}");
    assert!(
        status
            .as_deref()
            .is_ok_and(|text| text.contains("runtime-policy: development-source")),
        "status failed: {status:?}"
    );
    assert!(stop.is_ok(), "stop failed: {stop:?}");
}
