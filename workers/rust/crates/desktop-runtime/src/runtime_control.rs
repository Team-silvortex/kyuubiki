use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde_json::Value;

use crate::runtime_layout::{
    RuntimePaths, resolve_development_command, runtime_bin_dirs, runtime_paths,
};
use crate::{HotServiceMode, ServiceMode};

const ORCHESTRATOR_PORT: u16 = 4000;
const FRONTEND_PORT: u16 = 3000;
const DEFAULT_AGENT_ENDPOINTS: &str = "127.0.0.1:5001,127.0.0.1:5002";

struct ManagedProcess {
    label: String,
    command: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
    pid: PathBuf,
    log: PathBuf,
    port: Option<u16>,
    env: HashMap<String, String>,
}

pub(super) fn service_status() -> Result<String, String> {
    let paths = runtime_paths()?;
    let env = runtime_env(&paths.root);
    let mode = read_runtime_mode(&paths, &env);
    let mut lines = vec![
        format!("deployment-mode: {mode}"),
        format!(
            "control-mode: {}",
            if mode == "local" {
                "standalone"
            } else {
                "orch_managed"
            }
        ),
        format!(
            "authority-mode: {}",
            if mode == "local" {
                "self_directed"
            } else {
                "single_orchestrator"
            }
        ),
        format!("runtime-policy: {}", paths.origin_label()),
    ];
    if paths.is_development() {
        for command in ["npm", "mix", "cargo"] {
            lines.push(match resolve_development_command(&paths.root, command) {
                Ok(path) => format!(
                    "runtime-command[{command}]: development -> {}",
                    path.display()
                ),
                Err(error) => format!("runtime-command[{command}]: missing ({error})"),
            });
        }
    } else {
        lines.push(format!("runtime-root: {}", paths.root.display()));
        lines.push(format!("runtime-state: {}", paths.state.display()));
        for service in ["agent", "orchestrator", "frontend"] {
            lines.push(
                match paths.service(service, &[("port", "5001".to_string())]) {
                    Ok(spec) => format!(
                        "runtime-service[{service}]: installed -> {}",
                        spec.command.display()
                    ),
                    Err(error) => format!("runtime-service[{service}]: blocked ({error})"),
                },
            );
        }
    }
    lines.push(service_line(
        "orchestrator",
        &paths.run.join("orchestrator.pid"),
        ORCHESTRATOR_PORT,
        "http",
    ));
    lines.push(service_line(
        "frontend",
        &paths.run.join("frontend.pid"),
        FRONTEND_PORT,
        "http",
    ));
    for port in agent_ports(&paths.root, &env) {
        lines.push(service_line(
            &format!("agent[{port}]"),
            &paths.run.join(format!("agent-{port}.pid")),
            port,
            "tcp",
        ));
    }
    Ok(lines.join("\n"))
}

pub(super) fn service_start(mode: ServiceMode) -> Result<String, String> {
    let requested = service_mode_name(mode);
    start_services(requested)
}

pub(super) fn service_restart(mode: ServiceMode) -> Result<String, String> {
    let mut lines = vec![service_stop()?];
    lines.push(start_services(service_mode_name(mode))?);
    lines.push("restart complete".to_string());
    Ok(lines.join("\n"))
}

pub(super) fn service_stop() -> Result<String, String> {
    let paths = runtime_paths()?;
    let env = runtime_env(&paths.root);
    let mut lines = Vec::new();
    let mut ports = agent_ports(&paths.root, &env);
    ports.reverse();
    for port in ports {
        lines.push(stop_managed(
            &paths.run.join(format!("agent-{port}.pid")),
            &format!("agent[{port}]"),
            Some(port),
        )?);
    }
    lines.push(stop_managed(
        &paths.run.join("frontend.pid"),
        "frontend",
        Some(FRONTEND_PORT),
    )?);
    lines.push(stop_managed(
        &paths.run.join("orchestrator.pid"),
        "orchestrator",
        Some(ORCHESTRATOR_PORT),
    )?);
    remove_file_if_present(&paths.run.join("runtime-mode.txt"))?;
    remove_file_if_present(&paths.hot.join("native-mode.txt"))?;
    Ok(lines.join("\n"))
}

pub(super) fn hot_service_status() -> Result<String, String> {
    let paths = runtime_paths()?;
    let active = paths.hot.join("native-mode.txt").is_file();
    let mut lines = vec![format!(
        "hot-loop: {}",
        if active {
            "running (native runtime control)"
        } else {
            "stopped"
        }
    )];
    lines.push(listening_line(
        "hot-web",
        "http://127.0.0.1:4000",
        ORCHESTRATOR_PORT,
    ));
    lines.push(listening_line(
        "hot-frontend",
        "http://127.0.0.1:3000",
        FRONTEND_PORT,
    ));
    let env = runtime_env(&paths.root);
    for port in agent_ports(&paths.root, &env) {
        lines.push(listening_line(
            &format!("hot-agent[{port}]"),
            &format!("tcp://127.0.0.1:{port}"),
            port,
        ));
    }
    lines.push(format!("hot-logs: {}", paths.hot.display()));
    Ok(lines.join("\n"))
}

pub(super) fn hot_service_start(mode: HotServiceMode) -> Result<String, String> {
    let paths = runtime_paths()?;
    if !paths.is_development() {
        return Err(
            "hot runtime controls are available only in explicit desktop source mode".to_string(),
        );
    }
    ensure_runtime_dirs(&paths)?;
    let mode = match mode {
        HotServiceMode::Local => "local",
        HotServiceMode::Cloud => "cloud",
        HotServiceMode::Distributed => "distributed",
    };
    let rendered = start_services(mode)?;
    fs::write(paths.hot.join("native-mode.txt"), format!("{mode}\n"))
        .map_err(|error| format!("failed to write native hot runtime state: {error}"))?;
    Ok(format!(
        "{rendered}\nstarted native development runtime ({mode})"
    ))
}

pub(super) fn hot_service_stop() -> Result<String, String> {
    service_stop()
}

pub(super) fn export_database(url: Option<&str>) -> Result<String, String> {
    let url = url.unwrap_or("http://127.0.0.1:4000/api/v1/export/database");
    let target = url
        .strip_prefix("http://")
        .ok_or_else(|| "native database export currently requires an HTTP URL".to_string())?;
    let (authority, path) = target
        .split_once('/')
        .map(|(authority, path)| (authority, format!("/{path}")))
        .unwrap_or((target, "/".to_string()));
    let (host, port) = authority
        .rsplit_once(':')
        .map(|(host, port)| {
            port.parse::<u16>()
                .map(|port| (host, port))
                .map_err(|error| format!("invalid export URL port: {error}"))
        })
        .transpose()?
        .unwrap_or((authority, 80));
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return Err("native database export only permits loopback endpoints".to_string());
    }

    let mut stream = TcpStream::connect((host, port))
        .map_err(|error| format!("failed to connect to database export endpoint: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|error| format!("failed to configure export timeout: {error}"))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\nAccept: application/json\r\n\r\n"
    )
    .map_err(|error| format!("failed to request database export: {error}"))?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| format!("failed to read database export: {error}"))?;
    let rendered = String::from_utf8(response)
        .map_err(|error| format!("database export was not UTF-8: {error}"))?;
    let (headers, body) = rendered
        .split_once("\r\n\r\n")
        .ok_or_else(|| "database export returned an invalid HTTP response".to_string())?;
    if !headers
        .lines()
        .next()
        .is_some_and(|line| line.contains(" 200 "))
    {
        return Err(format!(
            "database export failed: {}",
            headers.lines().next().unwrap_or("unknown HTTP status")
        ));
    }
    Ok(body.to_string())
}

fn start_services(requested_mode: &str) -> Result<String, String> {
    let paths = runtime_paths()?;
    ensure_runtime_dirs(&paths)?;
    let mut env = runtime_env(&paths.root);
    let mode = resolve_mode(requested_mode, &env)?;
    if mode == "local" {
        env.entry("SQLITE_DATABASE_PATH".to_string())
            .or_insert_with(|| {
                paths
                    .data
                    .join(if paths.is_development() {
                        "kyuubiki_dev.sqlite3"
                    } else {
                        "kyuubiki.sqlite3"
                    })
                    .display()
                    .to_string()
            });
    }
    apply_mode_env(&mut env, &mode)?;
    let endpoints = agent_endpoints(&env);
    env.insert("KYUUBIKI_AGENT_ENDPOINTS".to_string(), endpoints.clone());
    env.entry("KYUUBIKI_AGENT_DISCOVERY".to_string())
        .or_insert_with(|| "static".to_string());
    augment_path(&paths, &mut env);

    let mut lines = Vec::new();
    if mode != "distributed" {
        for port in agent_ports(&paths.root, &env) {
            lines.push(start_agent(&paths, port, &env)?);
        }
    }
    lines.push(start_orchestrator(&paths, &env, &mode)?);
    lines.push(start_frontend(&paths, &env)?);
    fs::write(paths.run.join("runtime-mode.txt"), format!("{mode}\n"))
        .map_err(|error| format!("failed to persist runtime mode: {error}"))?;
    Ok(lines.join("\n"))
}

fn start_agent(
    paths: &RuntimePaths,
    port: u16,
    env: &HashMap<String, String>,
) -> Result<String, String> {
    if is_port_listening(port) {
        return Ok(format!(
            "Rust FEM agent already running at tcp://127.0.0.1:{port}"
        ));
    }
    let (command, args, cwd) = if paths.is_development() {
        (
            resolve_development_command(&paths.root, "cargo")?,
            vec![
                "run".into(),
                "-p".into(),
                "kyuubiki-cli".into(),
                "--bin".into(),
                "kyuubiki-cli".into(),
                "--".into(),
                "agent".into(),
                "--port".into(),
                port.to_string(),
            ],
            paths.root.join("workers/rust"),
        )
    } else {
        let spec = paths.service("agent", &[("port", port.to_string())])?;
        (spec.command, spec.args, spec.cwd)
    };
    let process = ManagedProcess {
        label: format!("agent[{port}]"),
        command,
        args,
        cwd,
        pid: paths.run.join(format!("agent-{port}.pid")),
        log: paths.run.join(format!("agent-{port}.log")),
        port: Some(port),
        env: env.clone(),
    };
    spawn_managed(process, Duration::from_secs(60))?;
    Ok(format!("started Rust FEM agent at tcp://127.0.0.1:{port}"))
}

fn start_orchestrator(
    paths: &RuntimePaths,
    env: &HashMap<String, String>,
    mode: &str,
) -> Result<String, String> {
    if is_port_listening(ORCHESTRATOR_PORT) {
        return Ok("orchestrator already running at http://127.0.0.1:4000".to_string());
    }
    let mut process_env = env.clone();
    process_env.insert("PORT".to_string(), ORCHESTRATOR_PORT.to_string());
    process_env.insert("RELEASE_DISTRIBUTION".to_string(), "none".to_string());
    let (command, args, cwd) = if paths.is_development() {
        (
            resolve_development_command(&paths.root, "mix")?,
            vec!["run".into(), "--no-halt".into()],
            paths.root.join("apps/web"),
        )
    } else {
        let spec = paths.service("orchestrator", &[])?;
        (spec.command, spec.args, spec.cwd)
    };
    let process = ManagedProcess {
        label: "orchestrator".to_string(),
        command,
        args,
        cwd,
        pid: paths.run.join("orchestrator.pid"),
        log: paths.run.join("orchestrator.log"),
        port: Some(ORCHESTRATOR_PORT),
        env: process_env,
    };
    // A self-hosted Elixir runtime may need to compile OTP dependencies on its
    // first launch, especially after installer or cache maintenance.
    spawn_managed(process, Duration::from_secs(120))?;
    Ok(format!(
        "started orchestrator API at http://127.0.0.1:4000 ({mode})"
    ))
}

fn start_frontend(paths: &RuntimePaths, env: &HashMap<String, String>) -> Result<String, String> {
    if is_port_listening(FRONTEND_PORT) {
        return Ok("frontend already running at http://127.0.0.1:3000".to_string());
    }
    let mut process_env = env.clone();
    process_env.insert("HOSTNAME".to_string(), "127.0.0.1".to_string());
    process_env.insert("PORT".to_string(), FRONTEND_PORT.to_string());
    let (command, args, cwd, label) = if paths.is_development() {
        (
            resolve_development_command(&paths.root, "npm")?,
            vec!["run".into(), "dev".into()],
            paths.root.join("apps/frontend"),
            "development Next.js workbench",
        )
    } else {
        let spec = paths.service("frontend", &[])?;
        (
            spec.command,
            spec.args,
            spec.cwd,
            "installer-managed workbench frontend",
        )
    };
    let process = ManagedProcess {
        label: "frontend".to_string(),
        command,
        args,
        cwd,
        pid: paths.run.join("frontend.pid"),
        log: paths.run.join("frontend.log"),
        port: Some(FRONTEND_PORT),
        env: process_env,
    };
    spawn_managed(process, Duration::from_secs(60))?;
    Ok(format!("started {label} at http://127.0.0.1:3000"))
}

fn spawn_managed(process: ManagedProcess, timeout: Duration) -> Result<u32, String> {
    if let Some(parent) = process.pid.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&process.log)
        .map_err(|error| format!("failed to open {}: {error}", process.log.display()))?;
    let stderr = stdout
        .try_clone()
        .map_err(|error| format!("failed to clone {}: {error}", process.log.display()))?;
    let mut command = Command::new(&process.command);
    command
        .args(&process.args)
        .current_dir(&process.cwd)
        .envs(&process.env)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    configure_detached(&mut command);
    let mut child = command.spawn().map_err(|error| {
        format!(
            "failed to start {} with {}: {error}",
            process.label,
            process.command.display()
        )
    })?;
    let pid = child.id();
    fs::write(&process.pid, format!("{pid}\n"))
        .map_err(|error| format!("failed to write {}: {error}", process.pid.display()))?;
    thread::spawn(move || {
        let _ = child.wait();
    });

    if let Some(port) = process.port {
        wait_for_port(port, true, timeout).map_err(|error| {
            let detail = fs::read_to_string(&process.log)
                .ok()
                .map(|text| {
                    let mut lines = text.lines().rev().take(8).collect::<Vec<_>>();
                    lines.reverse();
                    lines.join(" | ")
                })
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| "no runtime log output".to_string());
            format!("{error}; {} log: {detail}", process.label)
        })?;
    }
    Ok(pid)
}

#[cfg(unix)]
fn configure_detached(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
}

#[cfg(windows)]
fn configure_detached(command: &mut Command) {
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
}

fn stop_managed(pid_path: &Path, label: &str, port: Option<u16>) -> Result<String, String> {
    let pid = read_pid(pid_path);
    if let Some(pid) = pid.filter(|pid| is_pid_alive(*pid)) {
        terminate_process(pid)?;
        if let Some(port) = port {
            wait_for_port(port, false, Duration::from_secs(10))?;
        }
        remove_file_if_present(pid_path)?;
        return Ok(format!("stopped {label} (pid {pid})"));
    }
    remove_file_if_present(pid_path)?;
    Ok(match port {
        Some(port) if is_port_listening(port) => {
            format!("{label}: port {port} is still busy (unmanaged process)")
        }
        _ => format!("{label}: stopped"),
    })
}

#[cfg(unix)]
fn terminate_process(pid: u32) -> Result<(), String> {
    unsafe {
        libc::kill(-(pid as i32), libc::SIGTERM);
    }
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(5) && is_pid_alive(pid) {
        thread::sleep(Duration::from_millis(100));
    }
    if is_pid_alive(pid) {
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn terminate_process(pid: u32) -> Result<(), String> {
    let status = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status()
        .map_err(|error| format!("failed to stop process {pid}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("taskkill failed for process {pid}"))
    }
}

fn wait_for_port(port: u16, expected_listening: bool, timeout: Duration) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if is_port_listening(port) == expected_listening {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(200));
    }
    Err(format!(
        "timed out waiting for port {port} to become {}",
        if expected_listening {
            "ready"
        } else {
            "closed"
        }
    ))
}

fn is_port_listening(port: u16) -> bool {
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    TcpStream::connect_timeout(&address, Duration::from_millis(180)).is_ok()
}

fn read_pid(path: &Path) -> Option<u32> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

#[cfg(unix)]
fn is_pid_alive(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as i32, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(windows)]
fn is_pid_alive(pid: u32) -> bool {
    Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
        .unwrap_or(false)
}

fn service_line(label: &str, pid_path: &Path, port: u16, scheme: &str) -> String {
    let pid = read_pid(pid_path);
    let address = format!("{scheme}://127.0.0.1:{port}");
    if is_port_listening(port) {
        if let Some(pid) = pid.filter(|pid| is_pid_alive(*pid)) {
            format!("{label}: running on {address} (pid {pid})")
        } else {
            format!("{label}: running on {address} (unmanaged pid)")
        }
    } else {
        format!("{label}: stopped")
    }
}

fn listening_line(label: &str, address: &str, port: u16) -> String {
    if is_port_listening(port) {
        format!("{label}: listening on {address}")
    } else {
        format!("{label}: stopped")
    }
}

fn ensure_runtime_dirs(paths: &RuntimePaths) -> Result<(), String> {
    fs::create_dir_all(&paths.hot)
        .map_err(|error| format!("failed to create {}: {error}", paths.hot.display()))?;
    fs::create_dir_all(&paths.data)
        .map_err(|error| format!("failed to create {}: {error}", paths.data.display()))
}

fn runtime_env(root: &Path) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for path in [
        root.join("config/.env.example"),
        root.join("config/.env.local"),
        root.join(".env.example"),
        root.join(".env.local"),
    ] {
        load_env_file(&path, &mut values);
    }
    values.extend(env::vars());
    values
}

fn load_env_file(path: &Path, values: &mut HashMap<String, String>) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    for line in contents.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
}

fn resolve_mode(requested: &str, env: &HashMap<String, String>) -> Result<String, String> {
    let mode = match requested {
        "local" | "cloud" | "distributed" => requested,
        _ => env
            .get("KYUUBIKI_DEPLOYMENT_MODE")
            .map(String::as_str)
            .unwrap_or("local"),
    };
    if matches!(mode, "local" | "cloud" | "distributed") {
        Ok(mode.to_string())
    } else {
        Err(format!("unsupported deployment mode: {mode}"))
    }
}

fn apply_mode_env(env: &mut HashMap<String, String>, mode: &str) -> Result<(), String> {
    if mode == "local" {
        env.insert("KYUUBIKI_STORAGE_BACKEND".into(), "sqlite".into());
        env.entry("SQLITE_DATABASE_PATH".into())
            .or_insert_with(|| "./tmp/data/kyuubiki_dev.sqlite3".into());
    } else {
        if env.get("DATABASE_URL").is_none_or(String::is_empty) {
            return Err(format!("DATABASE_URL is required for {mode} mode"));
        }
        env.insert("KYUUBIKI_STORAGE_BACKEND".into(), "postgres".into());
    }
    env.insert("KYUUBIKI_DEPLOYMENT_MODE".into(), mode.into());
    Ok(())
}

fn read_runtime_mode(paths: &RuntimePaths, env: &HashMap<String, String>) -> String {
    fs::read_to_string(paths.run.join("runtime-mode.txt"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| matches!(value.as_str(), "local" | "cloud" | "distributed"))
        .or_else(|| env.get("KYUUBIKI_DEPLOYMENT_MODE").cloned())
        .unwrap_or_else(|| "local".to_string())
}

fn agent_endpoints(env: &HashMap<String, String>) -> String {
    env.get("KYUUBIKI_AGENT_ENDPOINTS")
        .cloned()
        .unwrap_or_else(|| DEFAULT_AGENT_ENDPOINTS.to_string())
}

fn agent_ports(root: &Path, env: &HashMap<String, String>) -> Vec<u16> {
    if env.get("KYUUBIKI_AGENT_DISCOVERY").map(String::as_str) == Some("manifest") {
        return manifest_agent_ports(root, env);
    }
    agent_endpoints(env)
        .split(',')
        .filter_map(|entry| entry.trim().rsplit(':').next()?.parse().ok())
        .collect()
}

fn manifest_agent_ports(root: &Path, env: &HashMap<String, String>) -> Vec<u16> {
    let path = env
        .get("KYUUBIKI_AGENT_MANIFEST_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./deploy/agents.local.example.json"));
    let path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value.get("agents").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|agent| agent.get("port")?.as_u64())
        .filter_map(|port| u16::try_from(port).ok())
        .collect()
}

fn augment_path(paths: &RuntimePaths, env: &mut HashMap<String, String>) {
    let mut entries = runtime_bin_dirs(&paths.root);
    if cfg!(windows) {
        if let Some(system_root) = env.get("SystemRoot").or_else(|| env.get("SYSTEMROOT")) {
            entries.push(PathBuf::from(system_root).join("System32"));
        }
    } else {
        // Self-contained releases still use the OS command baseline from their
        // generated launchers. Package-manager and language-tool paths stay out.
        entries.extend([PathBuf::from("/usr/bin"), PathBuf::from("/bin")]);
    }
    if paths.is_development() {
        if let Some(home) = env.get("HOME") {
            entries.push(PathBuf::from(home).join(".cargo/bin"));
        }
        #[cfg(unix)]
        entries.extend([
            unix_rooted_path(&["opt", "homebrew", "bin"]),
            unix_rooted_path(&["usr", "local", "bin"]),
            unix_rooted_path(&["usr", "bin"]),
            unix_rooted_path(&["bin"]),
        ]);
        if let Some(current) = env.get("PATH") {
            entries.extend(env::split_paths(current));
        }
    }
    let joined = env::join_paths(entries.iter().filter(|path| path.is_dir()))
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    env.insert("PATH".to_string(), joined);
}

#[cfg(unix)]
fn unix_rooted_path(parts: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(std::path::MAIN_SEPARATOR.to_string());
    path.extend(parts);
    path
}

fn service_mode_name(mode: ServiceMode) -> &'static str {
    match mode {
        ServiceMode::Default => "default",
        ServiceMode::Local => "local",
        ServiceMode::Cloud => "cloud",
        ServiceMode::Distributed => "distributed",
    }
}

fn remove_file_if_present(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("failed to remove {}: {error}", path.display())),
    }
}

#[cfg(test)]
#[path = "runtime_control_tests.rs"]
mod tests;
