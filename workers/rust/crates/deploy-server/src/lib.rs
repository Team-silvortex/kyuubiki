use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

const SERVER_SCHEMA_VERSION: &str = "kyuubiki.deploy-server/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeployServerConfig {
    pub host: String,
    pub port: u16,
    pub auth_token: Option<String>,
    pub workspace_root: PathBuf,
    pub deploy_root: PathBuf,
    pub artifact_root: PathBuf,
    pub update_catalog_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Request {
    method: String,
    path: String,
    headers: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Response {
    status_code: u16,
    content_type: String,
    body: Vec<u8>,
}

impl DeployServerConfig {
    fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    fn render(&self) -> String {
        json!({
            "schema_version": SERVER_SCHEMA_VERSION,
            "service": "kyuubiki-deploy-server",
            "version": env!("CARGO_PKG_VERSION"),
            "host": self.host,
            "port": self.port,
            "paths": {
                "update_channels": "/api/v1/update/channels",
                "workload_catalog": "/api/v1/deploy/workloads",
                "agents_local": "/api/v1/deploy/agents/local",
                "agents_distributed": "/api/v1/deploy/agents/distributed",
                "integrity_contract": "/api/v1/deploy/integrity-contract",
                "artifacts": "/artifacts/*"
            }
        })
        .to_string()
    }
}

pub fn run_cli(args: Vec<String>) -> Result<(), String> {
    let config = parse_cli_args(args)?;
    serve(config)
}

pub fn print_help() {
    println!(concat!(
        "kyuubiki-deploy-server\n\n",
        "Usage:\n",
        "  cargo run -p kyuubiki-deploy-server -- serve [options]\n",
        "  cargo run -p kyuubiki-deploy-server -- help\n\n",
        "Options:\n",
        "  --host <host>                Bind host (default 127.0.0.1)\n",
        "  --port <port>                Bind port (default 4070)\n",
        "  --token <token>              Require auth on non-health routes\n",
        "  --workspace-root <path>      Repo workspace root\n",
        "  --deploy-root <path>         Deploy descriptor root\n",
        "  --artifact-root <path>       Artifact file root\n",
        "  --catalog-path <path>        Update catalog JSON path\n\n",
        "Environment:\n",
        "  KYUUBIKI_DEPLOY_SERVER_HOST\n",
        "  KYUUBIKI_DEPLOY_SERVER_PORT\n",
        "  KYUUBIKI_DEPLOY_SERVER_TOKEN\n",
        "  KYUUBIKI_DEPLOY_SERVER_WORKSPACE_ROOT\n",
        "  KYUUBIKI_DEPLOY_SERVER_DEPLOY_ROOT\n",
        "  KYUUBIKI_DEPLOY_SERVER_ARTIFACT_ROOT\n",
        "  KYUUBIKI_DEPLOY_SERVER_CATALOG_PATH\n\n",
        "Notes:\n",
        "  The server is read-only and defaults to loopback-only binding.\n",
        "  Set a token to protect non-health routes.\n",
    ));
}

fn parse_cli_args(args: Vec<String>) -> Result<DeployServerConfig, String> {
    let mut host =
        env_string("KYUUBIKI_DEPLOY_SERVER_HOST").unwrap_or_else(|| "127.0.0.1".to_string());
    let mut port = env_u16("KYUUBIKI_DEPLOY_SERVER_PORT").unwrap_or(4070);
    let mut auth_token = env_string("KYUUBIKI_DEPLOY_SERVER_TOKEN");
    let mut workspace_root =
        env_path("KYUUBIKI_DEPLOY_SERVER_WORKSPACE_ROOT").unwrap_or_else(default_workspace_root);
    let mut deploy_root = env_path("KYUUBIKI_DEPLOY_SERVER_DEPLOY_ROOT");
    let mut artifact_root = env_path("KYUUBIKI_DEPLOY_SERVER_ARTIFACT_ROOT");
    let mut catalog_path = env_path("KYUUBIKI_DEPLOY_SERVER_CATALOG_PATH");

    let mut iter = args.into_iter();
    if let Some(first) = iter.next() {
        match first.as_str() {
            "help" | "--help" | "-h" => {
                print_help();
                return Err(String::new());
            }
            "serve" => {}
            flag => {
                iter = std::iter::once(flag.to_string())
                    .chain(iter)
                    .collect::<Vec<_>>()
                    .into_iter()
            }
        }
    }

    let rest: Vec<String> = iter.collect();
    let mut index = 0;
    while index < rest.len() {
        let flag = &rest[index];
        let value = rest
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--host" => host = value.trim().to_string(),
            "--port" => {
                port = value
                    .parse::<u16>()
                    .map_err(|_| format!("invalid port: {value}"))?;
            }
            "--token" => auth_token = Some(value.trim().to_string()),
            "--workspace-root" => workspace_root = PathBuf::from(value),
            "--deploy-root" => deploy_root = Some(PathBuf::from(value)),
            "--artifact-root" => artifact_root = Some(PathBuf::from(value)),
            "--catalog-path" => catalog_path = Some(PathBuf::from(value)),
            other => return Err(format!("unknown flag: {other}")),
        }
        index += 2;
    }

    if host.trim().is_empty() {
        return Err("host must not be empty".to_string());
    }

    let deploy_root = deploy_root.unwrap_or_else(|| workspace_root.join("deploy"));
    let artifact_root = artifact_root.unwrap_or_else(|| workspace_root.join("dist"));
    let update_catalog_path =
        catalog_path.unwrap_or_else(|| deploy_root.join("update-channels.json"));

    Ok(DeployServerConfig {
        host,
        port,
        auth_token: auth_token.filter(|value| !value.is_empty()),
        workspace_root,
        deploy_root,
        artifact_root,
        update_catalog_path,
    })
}

pub fn serve(config: DeployServerConfig) -> Result<(), String> {
    let listener = TcpListener::bind(config.bind_addr().as_str())
        .map_err(|error| format!("failed to bind {}: {error}", config.bind_addr()))?;
    println!(
        "kyuubiki deploy server listening on http://{}",
        config.bind_addr()
    );

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_connection(stream, &config) {
                    eprintln!("{error}");
                }
            }
            Err(error) => eprintln!("failed to accept connection: {error}"),
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream, config: &DeployServerConfig) -> Result<(), String> {
    let mut buffer = [0_u8; 16 * 1024];
    let size = stream
        .read(&mut buffer)
        .map_err(|error| format!("failed to read request: {error}"))?;
    if size == 0 {
        return Ok(());
    }

    let request = parse_request(&buffer[..size])?;
    let head_only = request.method == "HEAD";
    let response = route_request(config, &request);
    write_response(&mut stream, response, head_only)
        .map_err(|error| format!("failed to write response: {error}"))
}

fn parse_request(buffer: &[u8]) -> Result<Request, String> {
    let request = String::from_utf8_lossy(buffer);
    let mut lines = request.lines();
    let first_line = lines.next().ok_or_else(|| "empty request".to_string())?;
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    if method.is_empty() || path.is_empty() {
        return Err(format!("invalid request line: {first_line}"));
    }
    let mut headers = HashMap::new();
    for line in lines.take_while(|line| !line.trim().is_empty()) {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    Ok(Request {
        method,
        path,
        headers,
    })
}

fn route_request(config: &DeployServerConfig, request: &Request) -> Response {
    if !matches!(request.method.as_str(), "GET" | "HEAD") {
        return json_response(405, json!({ "error": "method_not_allowed" }));
    }
    if route_requires_auth(request.path.as_str()) && !request_is_authorized(config, request) {
        return json_response(401, json!({ "error": "unauthorized" }));
    }

    match request.path.as_str() {
        "/" => json_response(200, root_payload(config)),
        "/health" | "/api/health" => json_response(200, health_payload(config)),
        "/api/v1/server/config" => raw_json_response(200, config.render().into_bytes()),
        "/api/v1/update/channels" => serve_json_file(&config.update_catalog_path),
        "/api/v1/deploy/workloads" => {
            serve_json_file(&config.deploy_root.join("workload-catalog.example.json"))
        }
        "/api/v1/deploy/agents/local" => {
            serve_json_file(&config.deploy_root.join("agents.local.example.json"))
        }
        "/api/v1/deploy/agents/distributed" => {
            serve_json_file(&config.deploy_root.join("agents.distributed.example.json"))
        }
        "/api/v1/deploy/integrity-contract" => serve_json_file(
            &config
                .deploy_root
                .join("installation-integrity-contract.json"),
        ),
        path if path.starts_with("/api/v1/update/channels/") => serve_channel_details(config, path),
        path if path.starts_with("/api/v1/releases/") => serve_release_manifest(config, path),
        path if path.starts_with("/artifacts/") => serve_artifact(config, path),
        _ => json_response(
            404,
            json!({
                "error": "not_found",
                "path": request.path,
            }),
        ),
    }
}

fn route_requires_auth(path: &str) -> bool {
    !matches!(path, "/health" | "/api/health")
}

fn request_is_authorized(config: &DeployServerConfig, request: &Request) -> bool {
    let Some(token) = config.auth_token.as_deref() else {
        return true;
    };
    match request.headers.get("authorization") {
        Some(value) if value.strip_prefix("Bearer ").map(str::trim) == Some(token) => true,
        _ => request.headers.get("x-kyuubiki-token").map(String::as_str) == Some(token),
    }
}

fn root_payload(config: &DeployServerConfig) -> Value {
    json!({
        "schema_version": SERVER_SCHEMA_VERSION,
        "service": "kyuubiki-deploy-server",
        "version": env!("CARGO_PKG_VERSION"),
        "listening": format!("http://{}", config.bind_addr()),
        "routes": [
            "/health",
            "/api/health",
            "/api/v1/server/config",
            "/api/v1/update/channels",
            "/api/v1/update/channels/<channel>",
            "/api/v1/deploy/workloads",
            "/api/v1/deploy/agents/local",
            "/api/v1/deploy/agents/distributed",
            "/api/v1/deploy/integrity-contract",
            "/api/v1/releases/<platform>/manifest",
            "/api/v1/releases/<platform>/launch",
            "/artifacts/*",
        ],
    })
}

fn health_payload(_config: &DeployServerConfig) -> Value {
    json!({
        "status": "ok",
        "service": "kyuubiki-deploy-server",
        "version": env!("CARGO_PKG_VERSION"),
        "schema_version": SERVER_SCHEMA_VERSION,
        "timestamp": unix_timestamp(),
    })
}

fn serve_channel_details(config: &DeployServerConfig, path: &str) -> Response {
    let channel_id = path.trim_start_matches("/api/v1/update/channels/");
    if channel_id.is_empty() {
        return json_response(404, json!({ "error": "channel_not_found" }));
    }

    let catalog = match read_json_file(&config.update_catalog_path) {
        Ok(value) => value,
        Err(error) => return json_response(500, json!({ "error": error })),
    };
    let channel = catalog
        .get("channels")
        .and_then(Value::as_array)
        .and_then(|channels| {
            channels
                .iter()
                .find(|entry| entry.get("id").and_then(Value::as_str) == Some(channel_id))
        });

    match channel {
        Some(value) => json_response(200, value.clone()),
        None => json_response(
            404,
            json!({
                "error": "channel_not_found",
                "channel": channel_id,
            }),
        ),
    }
}

fn serve_release_manifest(config: &DeployServerConfig, path: &str) -> Response {
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if segments.len() != 5 {
        return json_response(404, json!({ "error": "release_route_not_found" }));
    }
    let platform = segments[3];
    let kind = segments[4];
    let relative = match kind {
        "manifest" => PathBuf::from("dist")
            .join(platform)
            .join("manifests")
            .join("release-manifest.json"),
        "launch" => PathBuf::from("dist")
            .join(platform)
            .join("manifests")
            .join("launch.json"),
        _ => return json_response(404, json!({ "error": "release_route_not_found" })),
    };
    serve_json_file(&config.workspace_root.join(relative))
}

fn serve_artifact(config: &DeployServerConfig, path: &str) -> Response {
    let relative = path.trim_start_matches("/artifacts/");
    let target = match safe_join(&config.artifact_root, relative) {
        Ok(path) => path,
        Err(error) => return json_response(400, json!({ "error": error })),
    };
    let metadata = match fs::metadata(&target) {
        Ok(metadata) => metadata,
        Err(_) => {
            return json_response(
                404,
                json!({ "error": "artifact_not_found", "path": relative }),
            );
        }
    };
    if !metadata.is_file() {
        return json_response(
            404,
            json!({ "error": "artifact_not_found", "path": relative }),
        );
    }

    match fs::read(&target) {
        Ok(body) => Response {
            status_code: 200,
            content_type: content_type_for(&target).to_string(),
            body,
        },
        Err(error) => json_response(
            500,
            json!({ "error": format!("failed to read {}: {error}", target.display()) }),
        ),
    }
}

fn serve_json_file(path: &Path) -> Response {
    match fs::read(path) {
        Ok(body) => raw_json_response(200, body),
        Err(error) => json_response(
            404,
            json!({ "error": format!("failed to read {}: {error}", path.display()) }),
        ),
    }
}

fn read_json_file(path: &Path) -> Result<Value, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let mut target = root.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => target.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("artifact path escapes configured root".to_string());
            }
        }
    }
    Ok(target)
}

fn content_type_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
    {
        "json" => "application/json; charset=utf-8",
        "txt" | "md" => "text/plain; charset=utf-8",
        "html" => "text/html; charset=utf-8",
        "sh" => "text/x-shellscript; charset=utf-8",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        _ => "application/octet-stream",
    }
}

fn json_response(status_code: u16, value: Value) -> Response {
    raw_json_response(
        status_code,
        serde_json::to_vec_pretty(&value)
            .unwrap_or_else(|_| b"{\"error\":\"serialization_failed\"}".to_vec()),
    )
}

fn raw_json_response(status_code: u16, body: Vec<u8>) -> Response {
    Response {
        status_code,
        content_type: "application/json; charset=utf-8".to_string(),
        body,
    }
}

fn write_response(
    stream: &mut TcpStream,
    response: Response,
    head_only: bool,
) -> std::io::Result<()> {
    let body = if head_only { Vec::new() } else { response.body };
    let headers = format!(
        concat!(
            "HTTP/1.1 {} {}\r\n",
            "Content-Type: {}\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
        response.status_code,
        status_text(response.status_code),
        response.content_type,
        body.len()
    );
    stream.write_all(headers.as_bytes())?;
    stream.write_all(&body)
}

fn status_text(status_code: u16) -> &'static str {
    match status_code {
        200 => "OK",
        401 => "Unauthorized",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

fn env_string(key: &str) -> Option<String> {
    env::var(key).ok().map(|value| value.trim().to_string())
}

fn env_u16(key: &str) -> Option<u16> {
    env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var(key).ok().map(PathBuf::from)
}

fn default_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
