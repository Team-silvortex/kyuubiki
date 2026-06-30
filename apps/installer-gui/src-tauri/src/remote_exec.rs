use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use kyuubiki_installer::{credential_sandbox_root, workspace_root};

const REMOTE_TRUST_MODE_ENV: &str = "KYUUBIKI_INSTALLER_REMOTE_TRUST_MODE";

#[derive(Clone, Debug, PartialEq, Eq)]
struct RemoteTrustOptions {
    strict_host_key_checking: &'static str,
    known_hosts_path: PathBuf,
}

pub(crate) fn run_remote_ssh_command(
    ssh_port: Option<u16>,
    target: &str,
    remote_command: &str,
) -> Result<String, String> {
    let trust = remote_trust_options()?;
    let mut command = Command::new("ssh");
    apply_common_ssh_options(&mut command, &trust)?;
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
    let trust = remote_trust_options()?;
    let mut command = Command::new("scp");
    apply_common_ssh_options(&mut command, &trust)?;
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

fn apply_common_ssh_options(
    command: &mut Command,
    trust: &RemoteTrustOptions,
) -> Result<(), String> {
    if let Some(parent) = trust.known_hosts_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    command
        .arg("-o")
        .arg(format!(
            "StrictHostKeyChecking={}",
            trust.strict_host_key_checking
        ))
        .arg("-o")
        .arg(format!(
            "UserKnownHostsFile={}",
            trust.known_hosts_path.display()
        ))
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg("-o")
        .arg("ServerAliveInterval=15")
        .arg("-o")
        .arg("ServerAliveCountMax=3");
    Ok(())
}

fn remote_trust_options() -> Result<RemoteTrustOptions, String> {
    match env::var(REMOTE_TRUST_MODE_ENV)
        .unwrap_or_else(|_| "pinned-known-host".to_string())
        .trim()
    {
        "dev-accept-new" => Ok(RemoteTrustOptions {
            strict_host_key_checking: "accept-new",
            known_hosts_path: credential_sandbox_root(&workspace_root())
                .join("installer")
                .join("remote-trust")
                .join("dev-known_hosts"),
        }),
        "pinned-known-host" | "" => Ok(RemoteTrustOptions {
            strict_host_key_checking: "yes",
            known_hosts_path: credential_sandbox_root(&workspace_root())
                .join("installer")
                .join("remote-trust")
                .join("known_hosts"),
        }),
        other => Err(format!(
            "invalid {REMOTE_TRUST_MODE_ENV}: {other} (expected pinned-known-host or dev-accept-new)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::TEST_ENV_LOCK;

    #[test]
    fn defaults_to_pinned_known_host_mode() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        unsafe {
            env::remove_var(REMOTE_TRUST_MODE_ENV);
        }
        let trust = remote_trust_options().unwrap();
        assert_eq!(trust.strict_host_key_checking, "yes");
        assert!(trust.known_hosts_path.ends_with("remote-trust/known_hosts"));
    }

    #[test]
    fn dev_accept_new_requires_explicit_mode() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        unsafe {
            env::set_var(REMOTE_TRUST_MODE_ENV, "dev-accept-new");
        }
        let trust = remote_trust_options().unwrap();
        assert_eq!(trust.strict_host_key_checking, "accept-new");
        assert!(
            trust
                .known_hosts_path
                .ends_with("remote-trust/dev-known_hosts")
        );
        unsafe {
            env::remove_var(REMOTE_TRUST_MODE_ENV);
        }
    }
}
