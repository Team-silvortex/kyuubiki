use std::process::Command;

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBootstrapPayload {
    pub target_host: String,
    pub ssh_user: String,
    pub remote_workspace: String,
    pub ssh_port: Option<u16>,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAgentPayload {
    pub target_host: String,
    pub ssh_user: String,
    pub remote_workspace: String,
    pub orchestrator_url: String,
    pub agent_id: String,
    pub advertise_host: String,
    pub agent_port: u16,
    pub ssh_port: Option<u16>,
}

#[tauri::command]
pub fn remote_bootstrap(payload: RemoteBootstrapPayload) -> Result<String, String> {
    let ssh_user = validate_ssh_identity(&payload.ssh_user, "ssh user")?;
    let target_host = validate_ssh_identity(&payload.target_host, "target host")?;
    let target = format!("{}@{}", ssh_user, target_host);
    let remote_command = format!(
        "cd {workspace} && cargo run -p kyuubiki-installer --manifest-path workers/rust/Cargo.toml -- bootstrap",
        workspace = shell_escape(&payload.remote_workspace)
    );

    run_remote_ssh(payload.ssh_port, &target, &remote_command)
}

#[tauri::command]
pub fn remote_start_agent(payload: RemoteAgentPayload) -> Result<String, String> {
    let ssh_user = validate_ssh_identity(&payload.ssh_user, "ssh user")?;
    let target_host = validate_ssh_identity(&payload.target_host, "target host")?;
    let target = format!("{}@{}", ssh_user, target_host);
    let screen_name = format!("kyuubiki_remote_agent_{}", payload.agent_port);
    let remote_command = format!(
        "cd {workspace} && screen -S {screen} -X quit >/dev/null 2>&1 || true && screen -dmS {screen} sh -lc 'cd workers/rust && KYUUBIKI_ORCHESTRATOR_URL={orchestrator} KYUUBIKI_AGENT_ID={agent_id} KYUUBIKI_AGENT_ADVERTISE_HOST={advertise_host} cargo run -p kyuubiki-cli -- agent --host 0.0.0.0 --port {port} --agent-id {agent_id} --advertise-host {advertise_host} --orchestrator-url {orchestrator}'",
        workspace = shell_escape(&payload.remote_workspace),
        screen = shell_escape(&screen_name),
        orchestrator = shell_escape(&payload.orchestrator_url),
        agent_id = shell_escape(&payload.agent_id),
        advertise_host = shell_escape(&payload.advertise_host),
        port = payload.agent_port
    );

    run_remote_ssh(payload.ssh_port, &target, &remote_command)
}

fn validate_ssh_identity(value: &str, label: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is required"));
    }

    if trimmed.starts_with('-') {
        return Err(format!("{label} must not start with '-'"));
    }

    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ':' | '@'))
    {
        return Err(format!("{label} contains unsupported characters"));
    }

    Ok(trimmed.to_string())
}

fn run_remote_ssh(ssh_port: Option<u16>, target: &str, remote_command: &str) -> Result<String, String> {
    let mut command = Command::new("ssh");
    if let Some(port) = ssh_port {
        command.arg("-p").arg(port.to_string());
    }

    let output = command
        .arg(target)
        .arg(remote_command)
        .output()
        .map_err(|error| format!("failed to run ssh command: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() {
            format!("remote command completed on {}", target)
        } else {
            stdout
        })
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
