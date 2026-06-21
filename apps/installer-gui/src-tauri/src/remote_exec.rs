use std::process::Command;

pub(crate) fn run_remote_ssh_command(
    ssh_port: Option<u16>,
    target: &str,
    remote_command: &str,
) -> Result<String, String> {
    let mut command = Command::new("ssh");
    command
        .arg("-o")
        .arg("StrictHostKeyChecking=accept-new")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg("-o")
        .arg("ServerAliveInterval=15")
        .arg("-o")
        .arg("ServerAliveCountMax=3");
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
            format!("remote command completed on {target}")
        } else {
            stdout
        })
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

pub(crate) fn run_remote_scp_files(
    ssh_port: Option<u16>,
    target: &str,
    remote_dir: &str,
    sources: &[&str],
) -> Result<(), String> {
    if sources.is_empty() {
        return Ok(());
    }
    let mut command = Command::new("scp");
    command
        .arg("-o")
        .arg("StrictHostKeyChecking=accept-new")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg("-o")
        .arg("ServerAliveInterval=15")
        .arg("-o")
        .arg("ServerAliveCountMax=3");
    if let Some(port) = ssh_port {
        command.arg("-P").arg(port.to_string());
    }
    for source in sources {
        command.arg(source);
    }
    let remote_target = format!("{target}:{}", remote_dir.trim_end_matches('/')) + "/";
    let output = command
        .arg(remote_target)
        .output()
        .map_err(|error| format!("failed to run scp command: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

pub(crate) fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
