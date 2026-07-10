use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::remote_host;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_lab(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let command = args
        .first()
        .and_then(|value| value.to_str())
        .unwrap_or("help");
    let host = std::env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".to_string());
    let http_host = std::env::var("KYUUBIKI_LAB_HTTP_HOST")
        .unwrap_or_else(|_| "kyuubiki-lab.local".to_string());

    match command {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(0)
        }
        "status" => remote_host::ssh_status(root, &host, status_command()),
        "health" => run_health(root, &host, &http_host),
        "restart-deploy-server" => remote_sudo(root, &host, deploy_server_restart_command()),
        "deploy-server-log" => {
            remote_sudo(root, &host, journal_command("kyuubiki-deploy-server", 40))
        }
        "orchestrator-status" => {
            remote_host::ssh_status(root, &host, orchestrator_status_command())
        }
        "orchestrator-start" => remote_sudo(
            root,
            &host,
            service_command("start", "kyuubiki-orchestrator", 14),
        ),
        "orchestrator-stop" => remote_sudo(
            root,
            &host,
            service_command("stop", "kyuubiki-orchestrator", 14),
        ),
        "orchestrator-restart" => remote_sudo(
            root,
            &host,
            service_command("restart", "kyuubiki-orchestrator", 14),
        ),
        "orchestrator-log" => {
            remote_sudo(root, &host, journal_command("kyuubiki-orchestrator", 80))
        }
        "agent-status" => remote_host::ssh_status(root, &host, agent_status_command()),
        "agent-start" => remote_sudo(root, &host, service_command("start", "kyuubiki-agent", 14)),
        "agent-stop" => remote_sudo(root, &host, service_command("stop", "kyuubiki-agent", 14)),
        "agent-restart" => remote_sudo(
            root,
            &host,
            service_command("restart", "kyuubiki-agent", 14),
        ),
        "agent-log" => remote_sudo(root, &host, journal_command("kyuubiki-agent", 60)),
        "agent-registry-status" => remote_host::ssh_status(
            root,
            &host,
            service_status_command("kyuubiki-agent-registry", 14),
        ),
        "agent-registry-start" => remote_sudo(
            root,
            &host,
            service_command("start", "kyuubiki-agent-registry", 14),
        ),
        "agent-registry-stop" => remote_sudo(
            root,
            &host,
            service_command("stop", "kyuubiki-agent-registry", 14),
        ),
        "agent-registry-restart" => remote_sudo(
            root,
            &host,
            service_command("restart", "kyuubiki-agent-registry", 14),
        ),
        "agent-registry-log" => {
            remote_sudo(root, &host, journal_command("kyuubiki-agent-registry", 60))
        }
        "cleanup-residue" => remote_host::ssh_status(root, &host, cleanup_command()),
        _ => {
            eprintln!("unknown lab command: {command}");
            print_help();
            Ok(2)
        }
    }
}

fn remote_sudo(root: &Path, host: &str, command: String) -> RunnerResult<u8> {
    if !remote_host::ssh_success_quiet(root, host, "sudo -n true".to_string())? {
        return Err(format!(
            "remote sudo is not configured for passwordless access on {host}"
        ));
    }
    remote_host::ssh_status(root, host, format!("sudo {command}"))
}

fn run_health(root: &Path, host: &str, http_host: &str) -> RunnerResult<u8> {
    let https_status = Command::new("curl")
        .args(["-ks", &format!("https://{http_host}/health")])
        .current_dir(root)
        .status()
        .map_err(|error| format!("failed to run curl: {error}"))?;
    if https_status.success() {
        return Ok(0);
    }
    remote_host::ssh_status(
        root,
        host,
        "curl -s http://127.0.0.1:4070/health".to_string(),
    )
}

fn status_command() -> String {
    "systemctl --no-pager --full status kyuubiki-deploy-server | sed -n '1,12p'; \
     echo ---; systemctl --no-pager --full status caddy | sed -n '1,12p'; \
     echo ---; ss -ltnp | grep -E ':(80|443|4070)\\s' || true"
        .to_string()
}

fn deploy_server_restart_command() -> String {
    service_command("restart", "kyuubiki-deploy-server", 12)
}

fn orchestrator_status_command() -> String {
    format!(
        "{}; echo ---; ss -ltnp | grep ':4000 ' || true",
        service_status_command("kyuubiki-orchestrator", 14)
    )
}

fn agent_status_command() -> String {
    format!(
        "{}; echo ---; ss -ltnp | grep ':5001 ' || true",
        service_status_command("kyuubiki-agent", 14)
    )
}

fn service_command(action: &str, service: &str, lines: u16) -> String {
    format!(
        "systemctl {action} {service} && {}",
        service_status_command(service, lines)
    )
}

fn service_status_command(service: &str, lines: u16) -> String {
    format!("systemctl --no-pager --full status {service} | sed -n '1,{lines}p'")
}

fn journal_command(service: &str, lines: u16) -> String {
    format!("journalctl -u {service} -n {lines} --no-pager")
}

fn cleanup_command() -> String {
    "rm -f ~/deploy-server.pid ~/kyuubiki-agent-*.log ~/kyuubiki-agent-*.pid \
     ~/kyuubiki/.DS_Store ~/kyuubiki/deploy-server.log && echo cleaned"
        .to_string()
}

fn print_help() {
    println!(
        "kyuubiki-lab\n\n\
Usage:\n  \
./scripts/kyuubiki lab status\n  \
./scripts/kyuubiki lab health\n  \
./scripts/kyuubiki lab restart-deploy-server\n  \
./scripts/kyuubiki lab deploy-server-log\n  \
./scripts/kyuubiki lab orchestrator-status|orchestrator-start|orchestrator-stop\n  \
./scripts/kyuubiki lab agent-status|agent-start|agent-stop|agent-restart|agent-log\n  \
./scripts/kyuubiki lab agent-registry-status|agent-registry-start|agent-registry-stop\n  \
./scripts/kyuubiki lab cleanup-residue\n\n\
Environment:\n  \
KYUUBIKI_LAB_HOST        SSH host alias for the shared lab machine\n  \
KYUUBIKI_LAB_HTTP_HOST   HTTPS host for the shared lab machine"
    );
}
