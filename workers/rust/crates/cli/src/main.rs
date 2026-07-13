use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use kyuubiki_protocol::{RpcRequest, RpcResponse};

mod agent_deployment;
mod agent_http;
mod agent_mesh;
mod agent_state;
mod agent_watchdog;
mod config;
mod operator_task_builtin;
mod operator_task_runtime;
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
use config::{AgentConfig, Command};
use operator_task_runtime::{
    operator_package_runtime_binding_from_config, store_operator_package_runtime_binding,
};
use rpc::handle_request;
use transport::{AgentReply, FrameReadError, read_frame, write_agent_reply};
use worker::run_worker;

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
    store_runtime_descriptor(build_agent_descriptor(config));
    store_deployment_readiness(build_agent_deployment_readiness_for_config(config));
    store_operator_package_runtime_binding(operator_package_runtime_binding_from_config(config));
    let registration = AgentRegistrationHandle::maybe_spawn(config);
    let peer_mesh = PeerMeshHandle::maybe_spawn(config);
    let listener = TcpListener::bind((config.host.as_str(), config.port))
        .map_err(|error| format!("failed to bind {}:{}: {error}", config.host, config.port))?;

    for stream in listener.incoming() {
        let stream = stream.map_err(|error| format!("failed to accept connection: {error}"))?;
        handle_connection(stream)?;
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

#[cfg(test)]
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

#[cfg(test)]
mod tests;
