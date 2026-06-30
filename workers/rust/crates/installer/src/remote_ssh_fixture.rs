const REMOTE_SSH_FIXTURE_SCHEMA_VERSION: &str = "kyuubiki.remote-ssh-fixture/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteSshFixtureReport {
    pub schema_version: String,
    pub fixture_target: String,
    pub commands: Vec<RemoteSshFixtureCommand>,
    pub checks: Vec<RemoteSshFixtureCheck>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteSshFixturePlan {
    pub schema_version: String,
    pub compose_file: String,
    pub dockerfile: String,
    pub run_script: String,
    pub runtime_dir: String,
    pub bind_address: String,
    pub ssh_user: String,
    pub remote_workspace: String,
    pub manual_only: bool,
    pub ignored_runtime_paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteSshFixtureCommand {
    pub id: String,
    pub program: String,
    pub args: Vec<String>,
    pub mutates_remote: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteSshFixtureCheck {
    pub label: String,
    pub ok: bool,
    pub detail: String,
}

impl RemoteSshFixtureReport {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki remote ssh fixture".to_string(),
            format!("schema: {}", self.schema_version),
            format!("fixture_target: {}", self.fixture_target),
            "commands:".to_string(),
        ];
        for command in &self.commands {
            lines.push(format!(
                "  - {} {} {}",
                command.id,
                command.program,
                command.args.join(" ")
            ));
            lines.push(format!("    mutates_remote: {}", command.mutates_remote));
        }
        lines.push("checks:".to_string());
        for check in &self.checks {
            lines.push(format!(
                "  - [{}] {} - {}",
                if check.ok { "ok" } else { "fail" },
                check.label,
                check.detail
            ));
        }
        lines.push(
            "note: fixture only validates command shape; it does not open network sockets"
                .to_string(),
        );
        lines.join("\n")
    }
}

impl RemoteSshFixturePlan {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki remote ssh fixture plan".to_string(),
            format!("schema: {}", self.schema_version),
            format!("compose_file: {}", self.compose_file),
            format!("dockerfile: {}", self.dockerfile),
            format!("run_script: {}", self.run_script),
            format!("runtime_dir: {}", self.runtime_dir),
            format!("bind_address: {}", self.bind_address),
            format!("ssh_user: {}", self.ssh_user),
            format!("remote_workspace: {}", self.remote_workspace),
            format!("manual_only: {}", self.manual_only),
            "ignored_runtime_paths:".to_string(),
        ];
        for path in &self.ignored_runtime_paths {
            lines.push(format!("  - {path}"));
        }
        lines.push(
            "note: plan describes the Docker fixture scaffold; it does not start containers"
                .to_string(),
        );
        lines.join("\n")
    }
}

pub fn default_remote_ssh_fixture_plan() -> RemoteSshFixturePlan {
    RemoteSshFixturePlan {
        schema_version: "kyuubiki.remote-ssh-fixture-plan/v1".to_string(),
        compose_file: "tests/integration/remote-ssh-fixture/compose.yaml".to_string(),
        dockerfile: "tests/integration/remote-ssh-fixture/Dockerfile".to_string(),
        run_script: "scripts/run-remote-ssh-fixture.sh".to_string(),
        runtime_dir: "tests/integration/remote-ssh-fixture/runtime".to_string(),
        bind_address: "127.0.0.1:2222".to_string(),
        ssh_user: "kyuubiki-fixture".to_string(),
        remote_workspace: "/tmp/kyuubiki-fixture".to_string(),
        manual_only: true,
        ignored_runtime_paths: vec![
            "tests/integration/remote-ssh-fixture/runtime/client_key".to_string(),
            "tests/integration/remote-ssh-fixture/runtime/client_key.pub".to_string(),
            "tests/integration/remote-ssh-fixture/runtime/known_hosts".to_string(),
            "tests/integration/remote-ssh-fixture/runtime/workspace/".to_string(),
        ],
    }
}

pub fn default_remote_ssh_fixture_report() -> RemoteSshFixtureReport {
    remote_ssh_fixture_report(RemoteSshFixtureInput {
        ssh_user: "kyuubiki-fixture",
        target_host: "fixture.local",
        ssh_port: 2222,
        remote_workspace: "/tmp/kyuubiki-fixture",
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteSshFixtureInput<'a> {
    pub ssh_user: &'a str,
    pub target_host: &'a str,
    pub ssh_port: u16,
    pub remote_workspace: &'a str,
}

pub fn remote_ssh_fixture_report(input: RemoteSshFixtureInput<'_>) -> RemoteSshFixtureReport {
    let target = format!("{}@{}", input.ssh_user, input.target_host);
    let probe_command = format!(
        "cd '{}' && printf '%s' 'kyuubiki-remote-ok'",
        input.remote_workspace
    );
    let commands = vec![
        RemoteSshFixtureCommand {
            id: "probe".to_string(),
            program: "ssh".to_string(),
            args: ssh_args(input.ssh_port, &target, &probe_command),
            mutates_remote: false,
        },
        RemoteSshFixtureCommand {
            id: "artifact-sync".to_string(),
            program: "scp".to_string(),
            args: scp_args(
                input.ssh_port,
                &target,
                input.remote_workspace,
                &["fixture-artifact.tar.zst"],
            ),
            mutates_remote: true,
        },
    ];
    let checks = fixture_checks(input, &commands);
    RemoteSshFixtureReport {
        schema_version: REMOTE_SSH_FIXTURE_SCHEMA_VERSION.to_string(),
        fixture_target: target,
        commands,
        checks,
    }
}

fn ssh_args(port: u16, target: &str, remote_command: &str) -> Vec<String> {
    let mut args = common_ssh_options();
    args.extend([
        "-p".to_string(),
        port.to_string(),
        target.to_string(),
        remote_command.to_string(),
    ]);
    args
}

fn scp_args(port: u16, target: &str, remote_dir: &str, sources: &[&str]) -> Vec<String> {
    let mut args = common_ssh_options();
    args.extend(["-P".to_string(), port.to_string()]);
    args.extend(sources.iter().map(|source| (*source).to_string()));
    args.push(format!("{}:{}/", target, remote_dir.trim_end_matches('/')));
    args
}

fn common_ssh_options() -> Vec<String> {
    vec![
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        "-o".to_string(),
        "ServerAliveInterval=15".to_string(),
        "-o".to_string(),
        "ServerAliveCountMax=3".to_string(),
    ]
}

fn fixture_checks(
    input: RemoteSshFixtureInput<'_>,
    commands: &[RemoteSshFixtureCommand],
) -> Vec<RemoteSshFixtureCheck> {
    let flattened = commands
        .iter()
        .flat_map(|command| {
            std::iter::once(command.program.as_str()).chain(command.args.iter().map(String::as_str))
        })
        .collect::<Vec<_>>()
        .join(" ");
    vec![
        check(
            "fixture-target-only",
            input.target_host.ends_with(".local") && input.ssh_port == 2222,
            "uses a non-production fixture host and non-default fixture port",
        ),
        check(
            "no-password-material",
            !contains_secret_token(&flattened),
            "command plan must not contain passwords, tokens, or private-key material",
        ),
        check(
            "bounded-timeouts",
            flattened.contains("ConnectTimeout=10")
                && flattened.contains("ServerAliveInterval=15")
                && flattened.contains("ServerAliveCountMax=3"),
            "SSH commands include bounded connection and keepalive options",
        ),
        check(
            "host-trust-visible",
            flattened.contains("StrictHostKeyChecking=accept-new"),
            "fixture documents current dev trust mode before host-key pinning lands",
        ),
        check(
            "no-network-opened",
            true,
            "report generation is pure data construction and does not execute commands",
        ),
    ]
}

fn contains_secret_token(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    [
        "password",
        "passwd",
        "token",
        "private_key",
        "identityfile=",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn check(label: &str, ok: bool, detail: &str) -> RemoteSshFixtureCheck {
    RemoteSshFixtureCheck {
        label: label.to_string(),
        ok,
        detail: detail.to_string(),
    }
}
