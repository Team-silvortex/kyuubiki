use super::{LiveServer, Scratch, repo_root, spawn_log_reader};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    io::BufReader,
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

enum LiveDatabase<'a> {
    Sqlite(Option<&'a Path>),
    Postgres(&'a str),
}

pub(super) struct OwnedPostgresScope {
    url: String,
    schema: String,
    cleaned: bool,
}

impl OwnedPostgresScope {
    pub(super) fn new(url: &str, root: &Path) -> Self {
        Self {
            url: url.into(),
            schema: postgres_schema(root),
            cleaned: false,
        }
    }

    pub(super) fn cleanup(&mut self) -> Result<(), Box<dyn Error>> {
        let script = "test/support/headless_live_postgres.exs";
        let output = Command::new("mix")
            .args([
                "run",
                "--no-start",
                "-r",
                script,
                "-e",
                "KyuubikiWeb.TestSupport.HeadlessLivePostgres.cleanup()",
            ])
            .current_dir(repo_root().join("apps/web"))
            .env("MIX_ENV", "test")
            .env("ERL_FLAGS", "+S 2:2")
            .env("KYUUBIKI_STORAGE_BACKEND", "postgres")
            .env("DATABASE_URL", &self.url)
            .env("KYUUBIKI_HEADLESS_LIVE_POSTGRES_SCHEMA", &self.schema)
            .output()?;
        if !output.status.success() {
            return Err("owned PostgreSQL test schema cleanup failed".into());
        }
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for OwnedPostgresScope {
    fn drop(&mut self) {
        if !self.cleaned && self.cleanup().is_err() {
            eprintln!("owned PostgreSQL test schema cleanup failed");
        }
    }
}

fn postgres_schema(root: &Path) -> String {
    let digest = format!("{:x}", Sha256::digest(root.to_string_lossy().as_bytes()));
    format!("owned_live_{}", &digest[..48])
}

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
        LiveDatabase::Sqlite(database),
        false,
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
        LiveDatabase::Sqlite(Some(database)),
        true,
        false,
    )
}

pub(super) fn start_live_server_with_described_agents(
    agent_port: u16,
    data_dir: &Path,
) -> Result<LiveServer, Box<dyn Error>> {
    start_server(
        Some(agent_port),
        None,
        Some(data_dir),
        false,
        LiveDatabase::Sqlite(None),
        false,
        true,
    )
}

pub(super) fn start_live_server_with_typed_native_gate(
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
        LiveDatabase::Sqlite(Some(database)),
        true,
        true,
    )
}

pub(super) fn start_live_server_with_typed_postgres_gate(
    agent_port: u16,
    peer_port: u16,
    data_dir: &Path,
    database_url: &str,
) -> Result<LiveServer, Box<dyn Error>> {
    start_server(
        Some(agent_port),
        Some(peer_port),
        Some(data_dir),
        false,
        LiveDatabase::Postgres(database_url),
        true,
        true,
    )
}

fn start_server(
    agent_port: Option<u16>,
    peer_port: Option<u16>,
    data_dir: Option<&Path>,
    result_fixtures: bool,
    database: LiveDatabase<'_>,
    native_gate: bool,
    describe_agents: bool,
) -> Result<LiveServer, Box<dyn Error>> {
    let root = repo_root();
    let web_root = root.join("apps/web");
    let server_script = web_root.join("test/support/headless_live_server.exs");
    let scratch = Scratch::new()?;
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
        .env("KYUUBIKI_DATA_DIR", &data_root)
        .env("KYUUBIKI_DEPLOYMENT_MODE", "local")
        .env(
            "KYUUBIKI_HEADLESS_LIVE_DESCRIBE_AGENTS",
            if describe_agents { "1" } else { "0" },
        )
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
    let shared_lease = match database {
        LiveDatabase::Sqlite(path) => {
            let sqlite_path = path
                .map(Path::to_path_buf)
                .unwrap_or_else(|| scratch.0.join("kyuubiki.sqlite3"));
            command
                .env("KYUUBIKI_STORAGE_BACKEND", "sqlite")
                .env("SQLITE_DATABASE_PATH", sqlite_path);
            path.is_some()
        }
        LiveDatabase::Postgres(url) => {
            command
                .env("KYUUBIKI_STORAGE_BACKEND", "postgres")
                .env("DATABASE_URL", url)
                .env(
                    "KYUUBIKI_HEADLESS_LIVE_POSTGRES_SCHEMA",
                    postgres_schema(&data_root),
                )
                .env_remove("SQLITE_DATABASE_PATH");
            true
        }
    };
    if matches!(database, LiveDatabase::Sqlite(_)) {
        command.env_remove("KYUUBIKI_HEADLESS_LIVE_POSTGRES_SCHEMA");
    }
    if native_gate {
        command.env("KYUUBIKI_HEADLESS_LIVE_NATIVE_GATE_ROOT", &data_root);
    } else {
        command.env_remove("KYUUBIKI_HEADLESS_LIVE_NATIVE_GATE_ROOT");
    }
    if shared_lease {
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
    server.port = ready_rx
        .recv_timeout(Duration::from_secs(60))
        .map_err(|error| {
            format!(
                "live Orchestra readiness failed: {error}\n{}",
                server.logs()
            )
        })?;
    Ok(server)
}
