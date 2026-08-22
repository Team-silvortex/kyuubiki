use std::env;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::json;

use crate::direct_mesh_gateway::DirectMeshGateway;
use crate::frontend_http::{
    json_response, read_request, static_response, text_response, write_response,
};
use crate::frontend_proxy::{LoopbackUpstream, proxy_request};

const MAX_CONCURRENT_CONNECTIONS: usize = 128;

#[derive(Clone)]
struct FrontendServerConfig {
    host: String,
    port: u16,
    static_root: PathBuf,
    orchestrator: LoopbackUpstream,
    direct_mesh: DirectMeshGateway,
}

pub fn serve_frontend(args: Vec<String>) -> Result<(), String> {
    let config = Arc::new(parse_options(args)?);
    let listener = TcpListener::bind((config.host.as_str(), config.port)).map_err(|error| {
        format!(
            "failed to bind native frontend at {}: {error}",
            display_address(&config.host, config.port)
        )
    })?;
    let address = display_address(&config.host, config.port);
    println!(
        "Kyuubiki native frontend listening on http://{address} (root {})",
        config.static_root.display()
    );
    let active = Arc::new(AtomicUsize::new(0));
    for incoming in listener.incoming() {
        let mut stream = match incoming {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!("failed to accept frontend connection: {error}");
                continue;
            }
        };
        if active.fetch_add(1, Ordering::AcqRel) >= MAX_CONCURRENT_CONNECTIONS {
            active.fetch_sub(1, Ordering::AcqRel);
            let _ = write_response(
                &mut stream,
                json_response(503, json!({"error": "frontend_connection_limit"})),
                false,
            );
            continue;
        }
        let config = Arc::clone(&config);
        let active = Arc::clone(&active);
        std::thread::spawn(move || {
            let _guard = ConnectionGuard(active);
            if let Err(error) = handle_connection(&mut stream, &config) {
                eprintln!("native frontend request failed: {error}");
            }
        });
    }
    Ok(())
}

struct ConnectionGuard(Arc<AtomicUsize>);

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

fn parse_options(args: Vec<String>) -> Result<FrontendServerConfig, String> {
    let mut host = env::var("KYUUBIKI_FRONTEND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let mut port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|port| *port > 0)
        .unwrap_or(3000);
    let mut root = env::var_os("KYUUBIKI_FRONTEND_STATIC_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("services/frontend"));
    let mut orchestrator = env::var("KYUUBIKI_ORCHESTRATOR_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:4000".to_string());

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--host" => host = value,
            "--port" => {
                port = value
                    .parse::<u16>()
                    .ok()
                    .filter(|port| *port > 0)
                    .ok_or_else(|| format!("invalid frontend port: {value}"))?;
            }
            "--root" => root = PathBuf::from(value),
            "--orchestrator-url" => orchestrator = value,
            _ => return Err(format!("unknown serve-frontend option: {flag}")),
        }
    }
    if !valid_loopback_host(&host) {
        return Err("native frontend must bind to an explicit loopback host".to_string());
    }
    let static_root = root.canonicalize().map_err(|error| {
        format!(
            "failed to resolve frontend root {}: {error}",
            root.display()
        )
    })?;
    if !static_root.join("index.html").is_file() {
        return Err(format!(
            "frontend root {} does not contain index.html",
            static_root.display()
        ));
    }
    Ok(FrontendServerConfig {
        host,
        port,
        static_root,
        orchestrator: LoopbackUpstream::parse(&orchestrator)?,
        direct_mesh: DirectMeshGateway::from_env()?,
    })
}

fn handle_connection(stream: &mut TcpStream, config: &FrontendServerConfig) -> Result<(), String> {
    let request = match read_request(stream) {
        Ok(request) => request,
        Err(error) => {
            return write_response(
                stream,
                text_response(
                    if error.contains("exceeds 32 MiB") {
                        413
                    } else {
                        400
                    },
                    error,
                ),
                false,
            );
        }
    };
    if is_proxy_route(&request.path) {
        return match proxy_request(stream, &request, &config.orchestrator) {
            Ok(()) => Ok(()),
            Err(error) if !error.response_started => write_response(
                stream,
                json_response(
                    502,
                    json!({
                        "error": "orchestra_unavailable",
                        "message": error.message,
                    }),
                ),
                request.method == "HEAD",
            ),
            Err(error) => Err(error.message),
        };
    }
    let head_only = request.method == "HEAD";
    let response = if request.path.starts_with("/api/direct-mesh") {
        config.direct_mesh.handle(&request)
    } else {
        static_response(&config.static_root, &request)
    };
    write_response(stream, response, head_only)
}

fn is_proxy_route(path: &str) -> bool {
    path == "/api/health"
        || path == "/api/v1"
        || path.starts_with("/api/v1/")
        || path == "/api/playground"
        || path.starts_with("/api/playground/")
}

fn valid_loopback_host(host: &str) -> bool {
    host == "localhost"
        || host
            .parse::<IpAddr>()
            .map(|address| address.is_loopback())
            .unwrap_or(false)
        || host == Ipv4Addr::LOCALHOST.to_string()
        || host == Ipv6Addr::LOCALHOST.to_string()
}

fn display_address(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::{display_address, is_proxy_route, valid_loopback_host};

    #[test]
    fn api_ownership_is_explicit() {
        assert!(is_proxy_route("/api/v1/jobs"));
        assert!(is_proxy_route("/api/health"));
        assert!(!is_proxy_route("/api/direct-mesh/agents"));
        assert!(!is_proxy_route("/api/v10/not-v1"));
    }

    #[test]
    fn frontend_binding_stays_local() {
        assert!(valid_loopback_host("127.0.0.1"));
        assert!(valid_loopback_host("localhost"));
        assert!(valid_loopback_host("::1"));
        assert!(!valid_loopback_host("0.0.0.0"));
    }

    #[test]
    fn ipv6_loopback_urls_are_bracketed() {
        assert_eq!(display_address("::1", 3000), "[::1]:3000");
        assert_eq!(display_address("127.0.0.1", 3000), "127.0.0.1:3000");
    }
}
