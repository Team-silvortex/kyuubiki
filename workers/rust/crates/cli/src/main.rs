use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use kyuubiki_protocol::{RpcRequest, RpcResponse};
use serde::Deserialize;

mod agent_artifact;
mod agent_control_link;
mod agent_deployment;
mod agent_fault_injection;
mod agent_headless_bridge;
mod agent_http;
mod agent_mesh;
mod agent_result_artifact;
mod agent_state;
mod agent_watchdog;
mod agent_watchdog_runtime;
mod config;
mod operator_package_fetch_runtime;
mod operator_package_generation;
mod operator_package_runtime;
mod operator_task_builtin;
mod operator_task_receipts;
mod operator_task_runtime;
mod operator_task_runtime_error;
mod rpc;
mod transport;
mod worker;

#[cfg(test)]
use agent_http::parse_http_url;
use agent_mesh::{AgentRegistrationHandle, PeerMeshHandle};
#[cfg(test)]
use agent_mesh::{
    build_peer_descriptors, compute_cluster_health_score, filter_self_peers,
    normalize_peer_addresses,
};
use agent_state::{
    build_agent_deployment_readiness_for_config, build_agent_descriptor,
    store_deployment_readiness, store_runtime_descriptor,
};
use agent_watchdog_runtime::AgentWatchdogRuntimeHandle;
use config::{AgentConfig, Command};
use operator_package_fetch_runtime::configure_operator_package_fetch_runtime;
use operator_package_runtime::initialize_operator_package_runtime;
use rpc::handle_request;
use transport::{AgentReply, FrameReadError, read_frame, write_agent_reply};
use worker::run_worker;

#[derive(Deserialize)]
struct RpcRequestEnvelope {
    #[serde(flatten)]
    request: RpcRequest,
    #[serde(default)]
    job_id: Option<String>,
}

fn main() {
    match Command::from_env() {
        Command::Worker(config) => run_worker(config),
        Command::Agent(config) => {
            if let Err(error) = run_agent(&config) {
                eprintln!("agent error: {error}");
                std::process::exit(1);
            }
        }
    }
}

fn run_agent(config: &AgentConfig) -> Result<(), String> {
    let package_binding = initialize_operator_package_runtime(config)?;
    let mut config = config.clone();
    config.operator_activated_package_count = package_binding.activated_package_count();
    configure_operator_package_fetch_runtime(&config);
    agent_artifact::configure(&config);
    agent_fault_injection::configure_from_env()?;
    store_runtime_descriptor(build_agent_descriptor(&config));
    store_deployment_readiness(build_agent_deployment_readiness_for_config(&config));
    let listener = TcpListener::bind((config.host.as_str(), config.port))
        .map_err(|error| format!("failed to bind {}:{}: {error}", config.host, config.port))?;
    let watchdog = AgentWatchdogRuntimeHandle::maybe_spawn(&config)?;
    let registration = AgentRegistrationHandle::maybe_spawn(&config);
    let peer_mesh = PeerMeshHandle::maybe_spawn(&config);

    for stream in listener.incoming() {
        let stream = stream.map_err(|error| format!("failed to accept connection: {error}"))?;
        thread::Builder::new()
            .name("kyuubiki-agent-rpc".to_string())
            .spawn(move || {
                if let Err(error) = handle_connection(stream) {
                    eprintln!("agent connection error: {error}");
                }
            })
            .map_err(|error| format!("failed to spawn agent connection handler: {error}"))?;
    }

    if let Some(watchdog) = watchdog {
        watchdog.stop();
    }

    if let Some(registration) = registration {
        registration.stop();
    }

    if let Some(peer_mesh) = peer_mesh {
        peer_mesh.stop();
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

        let response = match decode_rpc_request(&payload) {
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

#[cfg(test)]
fn handle_request_bytes(payload: &[u8]) -> AgentReply {
    let request = match decode_rpc_request(payload) {
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

fn decode_rpc_request(payload: &[u8]) -> Result<RpcRequest, serde_json::Error> {
    let mut envelope = serde_json::from_slice::<RpcRequestEnvelope>(payload)?;

    if let (Some(job_id), Some(params)) = (envelope.job_id, envelope.request.params.as_object_mut())
    {
        params
            .entry("job_id".to_string())
            .or_insert(serde_json::Value::String(job_id));
    }

    Ok(envelope.request)
}

#[cfg(test)]
mod tests;
