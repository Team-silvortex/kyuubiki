use super::{LiveServer, Scratch, repo_root, spawn_log_reader};
use std::{
    error::Error,
    io::BufReader,
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

pub(super) fn start_live_server() -> Result<LiveServer, Box<dyn Error>> {
    start_live_server_with_agent(None)
}

pub(super) fn start_live_server_with_agent(
    agent_port: Option<u16>,
) -> Result<LiveServer, Box<dyn Error>> {
    start_live_server_with_agents(agent_port, None)
}

pub(super) fn start_live_server_with_agents(
    agent_port: Option<u16>,
    peer_port: Option<u16>,
) -> Result<LiveServer, Box<dyn Error>> {
    start_live_server_with_state(agent_port, peer_port, None)
}

pub(super) fn start_live_server_with_state(
    agent_port: Option<u16>,
    peer_port: Option<u16>,
    data_dir: Option<&Path>,
) -> Result<LiveServer, Box<dyn Error>> {
    start_live_server_with_result_fixtures(agent_port, peer_port, data_dir, false, None)
}

pub(super) fn start_live_server_with_result_fixtures(
    agent_port: Option<u16>,
    peer_port: Option<u16>,
    data_dir: Option<&Path>,
    result_fixtures: bool,
    database: Option<&Path>,
) -> Result<LiveServer, Box<dyn Error>> {
    start_server(
        agent_port,
        peer_port,
        data_dir,
        result_fixtures,
        database,
        false,
    )
}

pub(super) fn start_live_server_with_native_gate(
    agent_port: u16,
    peer_port: u16,
    data_dir: &Path,
    database: &Path,
) -> Result<LiveServer, Box<dyn Error>> {
    start_server(
        Some(agent_port),
        Some(peer_port),
        Some(data_dir),
        false,
        Some(database),
        true,
    )
}

fn start_server(
    agent_port: Option<u16>,
    peer_port: Option<u16>,
    data_dir: Option<&Path>,
    result_fixtures: bool,
    database: Option<&Path>,
    native_gate: bool,
) -> Result<LiveServer, Box<dyn Error>> {
    let root = repo_root();
    let web_root = root.join("apps/web");
    let server_script = web_root.join("test/support/headless_live_server.exs");
    let scratch = Scratch::new()?;
    let sqlite_path = database
        .map(Path::to_path_buf)
        .unwrap_or_else(|| scratch.0.join("kyuubiki.sqlite3"));
    let data_root = data_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| scratch.0.join("data"));

    let mut command = Command::new("mix");
    command
        .arg("run")
        .arg("--no-start")
        .arg(server_script)
        .current_dir(&web_root)
        .env("MIX_ENV", "test")
        // Concurrent owned servers must not each reserve every host scheduler.
        .env(
            "ERL_FLAGS",
            std::env::var_os("ERL_FLAGS").unwrap_or_else(|| "+S 2:2".into()),
        )
        .env("KYUUBIKI_STORAGE_BACKEND", "sqlite")
        .env("SQLITE_DATABASE_PATH", sqlite_path)
        .env("KYUUBIKI_DATA_DIR", &data_root)
        .env("KYUUBIKI_DEPLOYMENT_MODE", "local")
        .env(
            "KYUUBIKI_HEADLESS_LIVE_RESULT_FIXTURES",
            if result_fixtures { "1" } else { "0" },
        )
        .env(
            "KYUUBIKI_HEADLESS_LIVE_JOURNAL_ROOT",
            data_root.join("operator-task-dispatches"),
        )
        .env(
            "KYUUBIKI_HEADLESS_LIVE_SCENARIO",
            if agent_port.is_some() {
                "real_agent"
            } else {
                "electrostatic_quad_summary"
            },
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if native_gate {
        command.env("KYUUBIKI_HEADLESS_LIVE_NATIVE_GATE_ROOT", &data_root);
    } else {
        command.env_remove("KYUUBIKI_HEADLESS_LIVE_NATIVE_GATE_ROOT");
    }
    if database.is_some() {
        // Only the explicit restart fixture shares a lease across real BEAM process incarnations.
        command.env(
            "KYUUBIKI_ORCHESTRA_LEASE_NAME",
            "owned-live-restart-recovery",
        );
    }
    if let Some(port) = agent_port {
        command.env("KYUUBIKI_HEADLESS_LIVE_AGENT_PORT", port.to_string());
    } else {
        command.env_remove("KYUUBIKI_HEADLESS_LIVE_AGENT_PORT");
    }
    if let Some(port) = peer_port {
        command.env("KYUUBIKI_HEADLESS_LIVE_AGENT_PEER_PORT", port.to_string());
    } else {
        command.env_remove("KYUUBIKI_HEADLESS_LIVE_AGENT_PEER_PORT");
    }
    let child = command.spawn()?;

    let stdout_log = Arc::new(Mutex::new(String::new()));
    let stderr_log = Arc::new(Mutex::new(String::new()));
    // Own the child before readiness waiting, so every error path reaps it.
    let mut server = LiveServer {
        child,
        _scratch: scratch,
        stdout_log: Arc::clone(&stdout_log),
        stderr_log: Arc::clone(&stderr_log),
        port: 0,
    };
    let stdout = server.child.stdout.take().expect("stdout pipe");
    let stderr = server.child.stderr.take().expect("stderr pipe");
    let (ready_tx, ready_rx) = mpsc::channel();
    spawn_log_reader(
        BufReader::new(stdout),
        Arc::clone(&stdout_log),
        Some(ready_tx),
    );
    spawn_log_reader(BufReader::new(stderr), Arc::clone(&stderr_log), None);
    server.port = ready_rx.recv_timeout(Duration::from_secs(60))?;
    Ok(server)
}
