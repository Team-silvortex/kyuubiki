use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use kyuubiki_protocol::{
    AgentDescriptor, CancelJobRequest, Job, JobStatus, ProgressEvent, RPC_VERSION, RpcMethod,
    RpcProgress, RpcRequest, RpcResponse, SolveBarRequest, SolvePlaneTriangle2dRequest,
    SolveTruss2dRequest, SolveTruss3dRequest,
};
use kyuubiki_solver::{
    MockSolver, solve_bar_1d, solve_plane_triangle_2d, solve_truss_2d, solve_truss_3d,
};

fn main() {
    match Command::from_env() {
        Command::Worker(config) => {
            let job = Job::new(config.job_id, config.project_id, config.case_id);
            let solver = MockSolver::new(config.steps);

            for event in solver.solve(&job) {
                println!("{}", format_event(&event));
            }
        }
        Command::Agent(config) => {
            if let Err(error) = run_agent(&config) {
                eprintln!("agent error: {error}");
                std::process::exit(1);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkerConfig {
    job_id: String,
    project_id: String,
    case_id: String,
    steps: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentConfig {
    host: String,
    port: u16,
    agent_id: Option<String>,
    advertise_host: Option<String>,
    orchestrator_url: Option<String>,
    register_interval_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Worker(WorkerConfig),
    Agent(AgentConfig),
}

impl Command {
    fn from_env() -> Self {
        let args = std::env::args().skip(1).collect::<Vec<_>>();

        match args.first().map(String::as_str) {
            Some("agent") => Self::Agent(AgentConfig::from_args(&args[1..])),
            _ => Self::Worker(WorkerConfig::from_args(&args)),
        }
    }
}

impl WorkerConfig {
    fn from_args(args: &[String]) -> Self {
        let mut config = Self {
            job_id: "job-local-1".to_string(),
            project_id: "project-local-1".to_string(),
            case_id: "case-local-1".to_string(),
            steps: 5,
        };

        let mut args = args.iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--job-id" => {
                    if let Some(value) = args.next() {
                        config.job_id = value.clone();
                    }
                }
                "--project-id" => {
                    if let Some(value) = args.next() {
                        config.project_id = value.clone();
                    }
                }
                "--case-id" => {
                    if let Some(value) = args.next() {
                        config.case_id = value.clone();
                    }
                }
                "--steps" => {
                    if let Some(value) = args.next() {
                        config.steps = value.parse().unwrap_or(config.steps);
                    }
                }
                _ => {}
            }
        }

        config
    }
}

impl AgentConfig {
    fn from_args(args: &[String]) -> Self {
        let mut config = Self {
            host: "127.0.0.1".to_string(),
            port: 5001,
            agent_id: None,
            advertise_host: None,
            orchestrator_url: None,
            register_interval_ms: 5_000,
        };

        let mut args = args.iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--host" => {
                    if let Some(value) = args.next() {
                        config.host = value.clone();
                    }
                }
                "--port" => {
                    if let Some(value) = args.next() {
                        config.port = value.parse().unwrap_or(config.port);
                    }
                }
                "--agent-id" => {
                    if let Some(value) = args.next() {
                        config.agent_id = Some(value.clone());
                    }
                }
                "--advertise-host" => {
                    if let Some(value) = args.next() {
                        config.advertise_host = Some(value.clone());
                    }
                }
                "--orchestrator-url" => {
                    if let Some(value) = args.next() {
                        config.orchestrator_url = Some(value.clone());
                    }
                }
                "--register-interval-ms" => {
                    if let Some(value) = args.next() {
                        config.register_interval_ms =
                            value.parse().unwrap_or(config.register_interval_ms);
                    }
                }
                _ => {}
            }
        }

        if config.agent_id.is_none() {
            config.agent_id = std::env::var("KYUUBIKI_AGENT_ID").ok();
        }

        if config.advertise_host.is_none() {
            config.advertise_host = std::env::var("KYUUBIKI_AGENT_ADVERTISE_HOST").ok();
        }

        if config.orchestrator_url.is_none() {
            config.orchestrator_url = std::env::var("KYUUBIKI_ORCHESTRATOR_URL").ok();
        }

        config
    }
}

fn run_agent(config: &AgentConfig) -> Result<(), String> {
    let registration = AgentRegistrationHandle::maybe_spawn(config);
    let listener = TcpListener::bind((config.host.as_str(), config.port))
        .map_err(|error| format!("failed to bind {}:{}: {error}", config.host, config.port))?;

    for stream in listener.incoming() {
        let stream = stream.map_err(|error| format!("failed to accept connection: {error}"))?;
        handle_connection(stream)?;
    }

    if let Some(registration) = registration {
        registration.stop();
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<(), String> {
    loop {
        let payload = match read_frame(&mut stream) {
            Ok(payload) => payload,
            Err(FrameReadError::ConnectionClosed) => break,
            Err(FrameReadError::Io(error)) => {
                return Err(format!("failed to read request frame: {error}"));
            }
        };

        let writer = Arc::new(Mutex::new(
            stream
                .try_clone()
                .map_err(|error| format!("failed to clone stream: {error}"))?,
        ));

        let response = match serde_json::from_slice::<RpcRequest>(&payload) {
            Ok(request) => handle_request(request, Some(writer.clone())),
            Err(error) => AgentReply::Stream(
                Vec::new(),
                RpcResponse::error("unknown", "invalid_json", error.to_string()),
            ),
        };

        write_agent_reply(&writer, response)?;
    }

    Ok(())
}

fn handle_request_bytes(payload: &[u8]) -> AgentReply {
    let request = match serde_json::from_slice::<RpcRequest>(payload) {
        Ok(request) => request,
        Err(error) => {
            return AgentReply::Stream(
                Vec::new(),
                RpcResponse::error("unknown", "invalid_json", error.to_string()),
            );
        }
    };

    handle_request(request, None)
}

fn handle_request(request: RpcRequest, writer: Option<Arc<Mutex<TcpStream>>>) -> AgentReply {
    if request.rpc_version != RPC_VERSION {
        return AgentReply::Stream(
            Vec::new(),
            RpcResponse::error(
                request.id,
                "invalid_version",
                format!("unsupported rpc version: {}", request.rpc_version),
            ),
        );
    }

    let maybe_job_id = extract_job_id(&request.params);

    match request.method {
        RpcMethod::Ping => AgentReply::Stream(
            Vec::new(),
            RpcResponse::success(request.id, serde_json::json!({ "pong": true })),
        ),
        RpcMethod::DescribeAgent => AgentReply::Stream(
            Vec::new(),
            RpcResponse::success(
                request.id,
                serde_json::to_value(agent_descriptor()).expect("agent descriptor should serialize"),
            ),
        ),
        RpcMethod::SolveBar1d => {
            let params = match serde_json::from_value::<SolveBarRequest>(request.params.clone()) {
                Ok(params) => params,
                Err(error) => {
                    return AgentReply::Stream(
                        Vec::new(),
                        RpcResponse::error(request.id, "invalid_params", error.to_string()),
                    );
                }
            };

            let heartbeat = maybe_job_id.as_ref().and_then(|job_id| {
                writer.clone().map(|shared_writer| {
                    HeartbeatHandle::spawn(shared_writer, request.id.clone(), job_id.clone())
                })
            });

            match solve_bar_1d(&params) {
                Ok(result) => {
                    if let Some(job_id) = maybe_job_id.as_deref() {
                        if take_cancelled(job_id) {
                            if let Some(heartbeat) = heartbeat {
                                heartbeat.stop();
                            }

                            return AgentReply::Stream(
                                Vec::new(),
                                RpcResponse::error(request.id, "cancelled", "job was cancelled"),
                            );
                        }
                    }

                    let progress_frames =
                        build_progress_frames("axial bar", &request.id, params.elements + 1);
                    if let Some(heartbeat) = heartbeat {
                        heartbeat.stop();
                    }
                    AgentReply::Stream(
                        progress_frames,
                        RpcResponse::success(
                            request.id,
                            serde_json::to_value(result).expect("bar result should serialize"),
                        ),
                    )
                }
                Err(error) => {
                    if let Some(heartbeat) = heartbeat {
                        heartbeat.stop();
                    }

                    AgentReply::Stream(
                        Vec::new(),
                        RpcResponse::error(request.id, "solve_failed", error),
                    )
                }
            }
        }
        RpcMethod::SolveTruss2d => {
            let params = match serde_json::from_value::<SolveTruss2dRequest>(request.params.clone())
            {
                Ok(params) => params,
                Err(error) => {
                    return AgentReply::Stream(
                        Vec::new(),
                        RpcResponse::error(request.id, "invalid_params", error.to_string()),
                    );
                }
            };

            let heartbeat = maybe_job_id.as_ref().and_then(|job_id| {
                writer.clone().map(|shared_writer| {
                    HeartbeatHandle::spawn(shared_writer, request.id.clone(), job_id.clone())
                })
            });

            match solve_truss_2d(&params) {
                Ok(result) => {
                    if let Some(job_id) = maybe_job_id.as_deref() {
                        if take_cancelled(job_id) {
                            if let Some(heartbeat) = heartbeat {
                                heartbeat.stop();
                            }

                            return AgentReply::Stream(
                                Vec::new(),
                                RpcResponse::error(request.id, "cancelled", "job was cancelled"),
                            );
                        }
                    }

                    let progress_frames =
                        build_progress_frames("2d truss", &request.id, params.nodes.len());
                    if let Some(heartbeat) = heartbeat {
                        heartbeat.stop();
                    }
                    AgentReply::Stream(
                        progress_frames,
                        RpcResponse::success(
                            request.id,
                            serde_json::to_value(result).expect("truss result should serialize"),
                        ),
                    )
                }
                Err(error) => {
                    if let Some(heartbeat) = heartbeat {
                        heartbeat.stop();
                    }

                    AgentReply::Stream(
                        Vec::new(),
                        RpcResponse::error(request.id, "solve_failed", error),
                    )
                }
            }
        }
        RpcMethod::SolveTruss3d => {
            let params = match serde_json::from_value::<SolveTruss3dRequest>(request.params.clone())
            {
                Ok(params) => params,
                Err(error) => {
                    return AgentReply::Stream(
                        Vec::new(),
                        RpcResponse::error(request.id, "invalid_params", error.to_string()),
                    );
                }
            };

            let heartbeat = maybe_job_id.as_ref().and_then(|job_id| {
                writer.clone().map(|shared_writer| {
                    HeartbeatHandle::spawn(shared_writer, request.id.clone(), job_id.clone())
                })
            });

            match solve_truss_3d(&params) {
                Ok(result) => {
                    if let Some(job_id) = maybe_job_id.as_deref() {
                        if take_cancelled(job_id) {
                            if let Some(heartbeat) = heartbeat {
                                heartbeat.stop();
                            }

                            return AgentReply::Stream(
                                Vec::new(),
                                RpcResponse::error(request.id, "cancelled", "job was cancelled"),
                            );
                        }
                    }

                    let progress_frames =
                        build_progress_frames("3d truss", &request.id, params.nodes.len());
                    if let Some(heartbeat) = heartbeat {
                        heartbeat.stop();
                    }
                    AgentReply::Stream(
                        progress_frames,
                        RpcResponse::success(
                            request.id,
                            serde_json::to_value(result).expect("3d truss result should serialize"),
                        ),
                    )
                }
                Err(error) => {
                    if let Some(heartbeat) = heartbeat {
                        heartbeat.stop();
                    }

                    AgentReply::Stream(
                        Vec::new(),
                        RpcResponse::error(request.id, "solve_failed", error),
                    )
                }
            }
        }
        RpcMethod::SolvePlaneTriangle2d => {
            let params =
                match serde_json::from_value::<SolvePlaneTriangle2dRequest>(request.params.clone())
                {
                    Ok(params) => params,
                    Err(error) => {
                        return AgentReply::Stream(
                            Vec::new(),
                            RpcResponse::error(request.id, "invalid_params", error.to_string()),
                        );
                    }
                };

            let heartbeat = maybe_job_id.as_ref().and_then(|job_id| {
                writer.clone().map(|shared_writer| {
                    HeartbeatHandle::spawn(shared_writer, request.id.clone(), job_id.clone())
                })
            });

            match solve_plane_triangle_2d(&params) {
                Ok(result) => {
                    if let Some(job_id) = maybe_job_id.as_deref() {
                        if take_cancelled(job_id) {
                            if let Some(heartbeat) = heartbeat {
                                heartbeat.stop();
                            }

                            return AgentReply::Stream(
                                Vec::new(),
                                RpcResponse::error(request.id, "cancelled", "job was cancelled"),
                            );
                        }
                    }

                    let progress_frames =
                        build_progress_frames("2d plane triangle", &request.id, params.nodes.len());
                    if let Some(heartbeat) = heartbeat {
                        heartbeat.stop();
                    }
                    AgentReply::Stream(
                        progress_frames,
                        RpcResponse::success(
                            request.id,
                            serde_json::to_value(result).expect("plane result should serialize"),
                        ),
                    )
                }
                Err(error) => {
                    if let Some(heartbeat) = heartbeat {
                        heartbeat.stop();
                    }

                    AgentReply::Stream(
                        Vec::new(),
                        RpcResponse::error(request.id, "solve_failed", error),
                    )
                }
            }
        }
        RpcMethod::CancelJob => {
            let params = match serde_json::from_value::<CancelJobRequest>(request.params.clone()) {
                Ok(params) => params,
                Err(error) => {
                    return AgentReply::Stream(
                        Vec::new(),
                        RpcResponse::error(request.id, "invalid_params", error.to_string()),
                    );
                }
            };

            register_cancel(params.job_id);
            AgentReply::Stream(
                Vec::new(),
                RpcResponse::success(request.id, serde_json::json!({ "cancelled": true })),
            )
        }
    }
}

fn agent_descriptor() -> AgentDescriptor {
    AgentDescriptor::solver_agent_default()
}

fn build_progress_frames(
    model_name: &str,
    request_id: &str,
    node_count: usize,
) -> Vec<RpcProgress> {
    let steps = [
        (
            JobStatus::Preprocessing,
            0.1_f32,
            Some("normalizing study inputs".to_string()),
        ),
        (
            JobStatus::Partitioning,
            0.25_f32,
            Some(format!("partitioning {model_name} topology")),
        ),
        (
            JobStatus::Solving,
            0.7_f32,
            Some(format!("solving structural system with {node_count} nodes")),
        ),
        (
            JobStatus::Postprocessing,
            0.92_f32,
            Some("collecting nodal and elemental responses".to_string()),
        ),
    ];

    steps
        .into_iter()
        .enumerate()
        .map(|(index, (stage, progress, message))| {
            let mut event = ProgressEvent::new("solver-session", stage, progress);
            event.iteration = Some((index + 1) as u64);
            event.residual = Some(1.0 / ((index + 2) as f64));
            event.peak_memory = Some(512 + (index as u64) * 128);
            event.message = message;

            RpcProgress::new(request_id.to_string(), event)
        })
        .collect()
}

fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, FrameReadError> {
    let mut header = [0_u8; 4];

    match stream.read_exact(&mut header) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(FrameReadError::ConnectionClosed);
        }
        Err(error) => return Err(FrameReadError::Io(error)),
    }

    let frame_length = u32::from_be_bytes(header) as usize;
    let mut payload = vec![0_u8; frame_length];
    stream
        .read_exact(&mut payload)
        .map_err(FrameReadError::Io)?;

    Ok(payload)
}

fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> std::io::Result<()> {
    let frame_length = u32::try_from(payload.len()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "payload too large for 4-byte frame length",
        )
    })?;

    stream.write_all(&frame_length.to_be_bytes())?;
    stream.write_all(payload)
}

enum FrameReadError {
    ConnectionClosed,
    Io(std::io::Error),
}

enum AgentReply {
    Stream(Vec<RpcProgress>, RpcResponse),
}

fn cancellation_registry() -> &'static Mutex<HashSet<String>> {
    static REGISTRY: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

fn register_cancel(job_id: String) {
    if let Ok(mut registry) = cancellation_registry().lock() {
        registry.insert(job_id);
    }
}

fn take_cancelled(job_id: &str) -> bool {
    if let Ok(mut registry) = cancellation_registry().lock() {
        return registry.remove(job_id);
    }

    false
}

fn extract_job_id(params: &serde_json::Value) -> Option<String> {
    params
        .as_object()
        .and_then(|value| value.get("job_id"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn write_agent_reply(
    writer: &Arc<Mutex<TcpStream>>,
    reply: AgentReply,
) -> Result<(), String> {
    match reply {
        AgentReply::Stream(progress_frames, final_response) => {
            for progress_frame in progress_frames {
                write_json_frame(writer, &progress_frame)?;
            }

            write_json_frame(writer, &final_response)?;
            Ok(())
        }
    }
}

fn write_json_frame<T: serde::Serialize>(
    writer: &Arc<Mutex<TcpStream>>,
    payload: &T,
) -> Result<(), String> {
    let encoded = serde_json::to_vec(payload)
        .map_err(|error| format!("failed to serialize response frame: {error}"))?;

    let mut guard = writer
        .lock()
        .map_err(|_| "failed to lock tcp writer".to_string())?;

    write_frame(&mut guard, &encoded)
        .map_err(|error| format!("failed to write response frame: {error}"))
}

struct HeartbeatHandle {
    running: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

struct AgentRegistrationHandle {
    running: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl AgentRegistrationHandle {
    fn maybe_spawn(config: &AgentConfig) -> Option<Self> {
        let agent_id = config.agent_id.clone()?;
        let orchestrator_url = config.orchestrator_url.clone()?;
        let advertise_host = config
            .advertise_host
            .clone()
            .unwrap_or_else(|| config.host.clone());
        let port = config.port;
        let interval_ms = config.register_interval_ms;
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let initial_payload = serde_json::json!({
            "id": agent_id,
            "host": advertise_host,
            "port": port,
            "role": "solver"
        });
        let orchestrator_url_clone = orchestrator_url.clone();
        let agent_id_clone = agent_id.clone();

        let join_handle = thread::spawn(move || {
            let _ = post_json(
                &format!("{}/api/v1/agents/register", normalize_base_url(&orchestrator_url_clone)),
                &initial_payload,
            );

            while running_clone.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(interval_ms));

                if !running_clone.load(Ordering::SeqCst) {
                    break;
                }

                let _ = post_json(
                    &format!(
                        "{}/api/v1/agents/{}/heartbeat",
                        normalize_base_url(&orchestrator_url_clone),
                        agent_id_clone
                    ),
                    &initial_payload,
                );
            }

            let _ = delete_request(&format!(
                "{}/api/v1/agents/{}",
                normalize_base_url(&orchestrator_url_clone),
                agent_id_clone
            ));
        });

        Some(Self {
            running,
            join_handle: Some(join_handle),
        })
    }

    fn stop(mut self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

fn normalize_base_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

fn post_json(url: &str, payload: &serde_json::Value) -> Result<(), String> {
    let body = serde_json::to_string(payload)
        .map_err(|error| format!("failed to serialize registration payload: {error}"))?;
    send_http_request("POST", url, Some(("application/json", body.as_bytes())))
}

fn delete_request(url: &str) -> Result<(), String> {
    send_http_request("DELETE", url, None)
}

fn send_http_request(
    method: &str,
    url: &str,
    body: Option<(&str, &[u8])>,
) -> Result<(), String> {
    let parsed = parse_http_url(url)?;
    let address = format!("{}:{}", parsed.host, parsed.port);
    let socket_addr = address
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve {address}: {error}"))?
        .next()
        .ok_or_else(|| format!("failed to resolve {address}"))?;

    let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_millis(1_500))
        .map_err(|error| format!("failed to connect to {}:{}: {error}", parsed.host, parsed.port))?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(2_000)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(2_000)));

    let (content_type, bytes) = body.unwrap_or(("application/json", &[]));
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Type: {content_type}\r\nContent-Length: {length}\r\n\r\n",
        method = method,
        path = parsed.path,
        host = parsed.host,
        content_type = content_type,
        length = bytes.len()
    );

    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to write HTTP request: {error}"))?;
    if !bytes.is_empty() {
        stream
            .write_all(bytes)
            .map_err(|error| format!("failed to write HTTP request body: {error}"))?;
    }
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("failed to read HTTP response: {error}"))?;

    if response.starts_with("HTTP/1.1 2") || response.starts_with("HTTP/1.0 2") {
        Ok(())
    } else {
        Err(format!("unexpected HTTP response from {url}: {response}"))
    }
}

struct ParsedHttpUrl {
    host: String,
    port: u16,
    path: String,
}

fn parse_http_url(url: &str) -> Result<ParsedHttpUrl, String> {
    let raw = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("unsupported orchestrator URL: {url} (expected http://...)"))?;
    let (authority, path) = match raw.split_once('/') {
        Some((authority, path)) => (authority, format!("/{}", path)),
        None => (raw, "/".to_string()),
    };
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => {
            let port = port
                .parse::<u16>()
                .map_err(|_| format!("invalid orchestrator port in URL: {url}"))?;
            (host.to_string(), port)
        }
        None => (authority.to_string(), 80),
    };

    if host.trim().is_empty() {
        return Err(format!("invalid orchestrator host in URL: {url}"));
    }

    Ok(ParsedHttpUrl { host, port, path })
}

impl HeartbeatHandle {
    fn spawn(writer: Arc<Mutex<TcpStream>>, request_id: String, job_id: String) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let join_handle = thread::spawn(move || {
            while running_clone.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(1_000));

                if !running_clone.load(Ordering::SeqCst) {
                    break;
                }

                let heartbeat = RpcProgress::heartbeat(
                    request_id.clone(),
                    ProgressEvent {
                        job_id: job_id.clone(),
                        stage: JobStatus::Solving,
                        progress: 0.7,
                        residual: None,
                        iteration: None,
                        peak_memory: None,
                        message: Some("agent heartbeat: solver still active".to_string()),
                    },
                );

                if write_json_frame(&writer, &heartbeat).is_err() {
                    break;
                }
            }
        });

        Self {
            running,
            join_handle: Some(join_handle),
        }
    }

    fn stop(mut self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

fn format_event(event: &ProgressEvent) -> String {
    format!(
        "event|{}|{}|{:.2}|{}|{}|{}|{}",
        event.job_id,
        event.stage.as_str(),
        event.progress,
        optional_u64(event.iteration),
        optional_f64(event.residual),
        optional_u64(event.peak_memory),
        optional_string(event.message.as_deref())
    )
}

fn optional_u64(value: Option<u64>) -> String {
    value.map(|number| number.to_string()).unwrap_or_default()
}

fn optional_f64(value: Option<f64>) -> String {
    value.map(|number| number.to_string()).unwrap_or_default()
}

fn optional_string(value: Option<&str>) -> String {
    value.unwrap_or_default().replace('|', "/")
}

#[cfg(test)]
mod tests {
    use super::{
        AgentConfig, AgentReply, Command, WorkerConfig, format_event, handle_request_bytes,
        parse_http_url,
    };
    use kyuubiki_protocol::{
        AgentDescriptor, JobStatus, PlaneNodeInput, PlaneTriangleElementInput, ProgressEvent,
        RPC_VERSION, RpcMethod, RpcRequest, SolveBarRequest, SolvePlaneTriangle2dRequest,
        SolveTruss3dRequest, Truss3dElementInput, Truss3dNodeInput,
    };

    #[test]
    fn formats_events_for_machine_consumption() {
        let mut event = ProgressEvent::new("job-1", JobStatus::Solving, 0.5);
        event.iteration = Some(2);
        event.residual = Some(0.125);
        event.peak_memory = Some(576);
        event.message = Some("mock solve step 2/4".to_string());

        assert_eq!(
            format_event(&event),
            "event|job-1|solving|0.50|2|0.125|576|mock solve step 2/4"
        );
    }

    #[test]
    fn preserves_worker_defaults_when_no_args_are_given() {
        let config = WorkerConfig {
            job_id: "job-local-1".to_string(),
            project_id: "project-local-1".to_string(),
            case_id: "case-local-1".to_string(),
            steps: 5,
        };

        assert_eq!(config.steps, 5);
        assert_eq!(config.job_id, "job-local-1");
    }

    #[test]
    fn parses_agent_command_defaults() {
        let command = Command::Agent(AgentConfig::from_args(&[]));

        assert_eq!(
            command,
            Command::Agent(AgentConfig {
                host: "127.0.0.1".to_string(),
                port: 5001,
                agent_id: None,
                advertise_host: None,
                orchestrator_url: None,
                register_interval_ms: 5_000,
            })
        );
    }

    #[test]
    fn parses_http_url_for_remote_registration() {
        let parsed = parse_http_url("http://orchestrator.example.com:4000/api/v1/agents/register")
            .expect("parsed URL");
        assert_eq!(parsed.host, "orchestrator.example.com");
        assert_eq!(parsed.port, 4000);
        assert_eq!(parsed.path, "/api/v1/agents/register");
    }

    #[test]
    fn handles_solver_rpc_requests() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-1".to_string(),
            method: RpcMethod::SolveBar1d,
            params: serde_json::to_value(SolveBarRequest {
                length: 1.0,
                area: 0.01,
                youngs_modulus: 210.0e9,
                elements: 1,
                tip_force: 1000.0,
            })
            .expect("params"),
        };

        let response =
            handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

        let AgentReply::Stream(progress_frames, final_response) = response;

        assert_eq!(progress_frames.len(), 4);
        assert_eq!(progress_frames[0].event, "progress");
        assert_eq!(progress_frames[2].progress.stage, JobStatus::Solving);
        assert!(final_response.ok);
        assert!(final_response.error.is_none());
        assert_eq!(final_response.id, "rpc-1");
        let result: kyuubiki_protocol::SolveBarResult =
            serde_json::from_value(final_response.result.expect("solver result"))
                .expect("bar result");
        assert!((result.tip_displacement - 4.761904761904762e-7).abs() < 1.0e-12);
    }

    #[test]
    fn handles_plane_triangle_rpc_requests() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-plane".to_string(),
            method: RpcMethod::SolvePlaneTriangle2d,
            params: serde_json::to_value(SolvePlaneTriangle2dRequest {
                nodes: vec![
                    PlaneNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                    },
                    PlaneNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_x: false,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                    },
                    PlaneNodeInput {
                        id: "n2".to_string(),
                        x: 1.0,
                        y: 1.0,
                        fix_x: false,
                        fix_y: false,
                        load_x: 0.0,
                        load_y: -1000.0,
                    },
                ],
                elements: vec![PlaneTriangleElementInput {
                    id: "p0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    thickness: 0.02,
                    youngs_modulus: 70.0e9,
                    poisson_ratio: 0.33,
                }],
            })
            .expect("params"),
        };

        let response =
            handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

        let AgentReply::Stream(progress_frames, final_response) = response;

        assert_eq!(progress_frames.len(), 4);
        assert!(final_response.ok);
        let result: kyuubiki_protocol::SolvePlaneTriangle2dResult =
            serde_json::from_value(final_response.result.expect("solver result"))
                .expect("plane result");
        assert_eq!(result.nodes.len(), 3);
        assert_eq!(result.elements.len(), 1);
    }

    #[test]
    fn handles_truss_3d_rpc_requests() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-truss-3d".to_string(),
            method: RpcMethod::SolveTruss3d,
            params: serde_json::to_value(SolveTruss3dRequest {
                nodes: vec![
                    Truss3dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        fix_x: true,
                        fix_y: true,
                        fix_z: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        load_z: 0.0,
                    },
                    Truss3dNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                        fix_x: true,
                        fix_y: true,
                        fix_z: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        load_z: 0.0,
                    },
                    Truss3dNodeInput {
                        id: "n2".to_string(),
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                        fix_x: true,
                        fix_y: true,
                        fix_z: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        load_z: 0.0,
                    },
                    Truss3dNodeInput {
                        id: "n3".to_string(),
                        x: 0.2,
                        y: 0.2,
                        z: 1.0,
                        fix_x: false,
                        fix_y: false,
                        fix_z: false,
                        load_x: 0.0,
                        load_y: 0.0,
                        load_z: -1000.0,
                    },
                ],
                elements: vec![
                    Truss3dElementInput {
                        id: "e0".to_string(),
                        node_i: 0,
                        node_j: 3,
                        area: 0.01,
                        youngs_modulus: 70.0e9,
                    },
                    Truss3dElementInput {
                        id: "e1".to_string(),
                        node_i: 1,
                        node_j: 3,
                        area: 0.01,
                        youngs_modulus: 70.0e9,
                    },
                    Truss3dElementInput {
                        id: "e2".to_string(),
                        node_i: 2,
                        node_j: 3,
                        area: 0.01,
                        youngs_modulus: 70.0e9,
                    },
                    Truss3dElementInput {
                        id: "e3".to_string(),
                        node_i: 0,
                        node_j: 1,
                        area: 0.01,
                        youngs_modulus: 70.0e9,
                    },
                    Truss3dElementInput {
                        id: "e4".to_string(),
                        node_i: 1,
                        node_j: 2,
                        area: 0.01,
                        youngs_modulus: 70.0e9,
                    },
                    Truss3dElementInput {
                        id: "e5".to_string(),
                        node_i: 2,
                        node_j: 0,
                        area: 0.01,
                        youngs_modulus: 70.0e9,
                    },
                ],
            })
            .expect("params"),
        };

        let response =
            handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

        let AgentReply::Stream(progress_frames, final_response) = response;

        assert_eq!(progress_frames.len(), 4);
        assert!(final_response.ok);
        let result: kyuubiki_protocol::SolveTruss3dResult =
            serde_json::from_value(final_response.result.expect("solver result"))
                .expect("3d truss result");
        assert_eq!(result.nodes.len(), 4);
        assert_eq!(result.elements.len(), 6);
    }

    #[test]
    fn handles_ping_rpc_requests() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-ping".to_string(),
            method: RpcMethod::Ping,
            params: serde_json::json!({}),
        };

        let AgentReply::Stream(progress_frames, final_response) =
            handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

        assert!(progress_frames.is_empty());
        assert!(final_response.ok);
        assert_eq!(
            final_response.result.expect("ping result"),
            serde_json::json!({ "pong": true })
        );
    }

    #[test]
    fn handles_describe_agent_rpc_requests() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-describe".to_string(),
            method: RpcMethod::DescribeAgent,
            params: serde_json::json!({}),
        };

        let AgentReply::Stream(progress_frames, final_response) =
            handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

        assert!(progress_frames.is_empty());
        assert!(final_response.ok);

        let descriptor: AgentDescriptor =
            serde_json::from_value(final_response.result.expect("descriptor result"))
                .expect("agent descriptor");

        assert_eq!(descriptor.program, "kyuubiki-rust-agent");
        assert_eq!(descriptor.protocol.rpc_version, RPC_VERSION);
        assert!(descriptor
            .protocol
            .methods
            .contains(&RpcMethod::SolveTruss3d));
    }
}
