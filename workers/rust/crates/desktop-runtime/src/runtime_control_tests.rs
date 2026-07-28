use super::{
    agent_ports, apply_mode_env, runtime_env, service_start, service_status, service_stop,
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
