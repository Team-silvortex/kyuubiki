mod support;

use kyuubiki_operator_sdk::{current_platform_library_file_name, current_platform_target_id};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use support::operator_package::{
    LiveAgent, external_operator_task, refresh_task_digest, template_library_path,
    template_packages_root,
};

const PACKAGE_ID: &str = "extract.template_summary";
const PACKAGE_VERSION_V1: &str = "0.1.0";
const PACKAGE_VERSION_V2: &str = "0.2.0";
const TOKEN: &str = "operator-fetch-live-token";

#[test]
#[ignore = "requires prebuilt operator template cdylib"]
fn agent_fetches_executes_and_safely_rotates_bound_orchestra_package() {
    let work_root = temp_dir("agent-orchestra-fetch-live");
    let _cleanup = Cleanup(work_root.clone());
    let packages_root = work_root.join("store/packages");
    fs::create_dir_all(&packages_root).expect("create empty managed package cache");

    let template_root = template_packages_root();
    let entrypoint = fs::read(template_library_path(&template_root))
        .expect("read prebuilt operator template cdylib");
    let entrypoint_sha256 = sha256(&entrypoint);
    let fixture = CentralFixture::new(entrypoint);
    let (central_url, server) = fixture.serve();
    let agent = LiveAgent::start_orchestrated(&packages_root, &central_url, TOKEN)
        .expect("start orchestrated Agent with empty package cache");

    let task_v1 = central_operator_task(PACKAGE_VERSION_V1, &entrypoint_sha256);
    let first = agent
        .request(
            "fetch-and-execute",
            "run_operator_task_ir",
            json!({ "mode": "execute", "task_ir": task_v1.clone() }),
        )
        .expect("fetch and execute central operator package");
    assert_eq!(first["ok"], true, "response: {first}");
    assert_eq!(
        first["result"]["operator_package_execution"]["origin"],
        "bound_orchestra_fetch"
    );
    assert_eq!(
        first["result"]["operator_package_execution"]["cache_status"],
        "fetched_and_activated"
    );
    assert_eq!(
        first["result"]["operator_package_runtime"]["activated_package_count"],
        1
    );
    assert_eq!(first["result"]["result"]["summary"]["sum"], 14.0);

    let cached_v1 = agent
        .request(
            "execute-v1-from-cache",
            "run_operator_task_ir",
            json!({ "mode": "execute", "task_ir": task_v1 }),
        )
        .expect("execute v1 from verified Agent cache");
    assert_eq!(cached_v1["ok"], true, "response: {cached_v1}");
    assert_eq!(
        cached_v1["result"]["operator_package_execution"]["cache_status"],
        "verified_cache_hit"
    );
    assert_eq!(cached_v1["result"]["result"]["summary"]["max"], 8.0);

    let task_v2 = central_operator_task(PACKAGE_VERSION_V2, &entrypoint_sha256);
    let rotated = agent
        .request(
            "fetch-rotate-and-execute-v2",
            "run_operator_task_ir",
            json!({ "mode": "execute", "task_ir": task_v2.clone() }),
        )
        .expect("rotate and execute central operator package v2");
    assert_eq!(rotated["ok"], true, "response: {rotated}");
    assert_eq!(
        rotated["result"]["operator_package_execution"]["package_version"],
        PACKAGE_VERSION_V2
    );
    assert_eq!(
        rotated["result"]["operator_package_execution"]["cache_status"],
        "fetched_and_activated"
    );

    let requests = server.join().expect("central fixture server");
    assert_eq!(requests.len(), 6);
    assert!(requests.iter().all(|request| {
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer operator-fetch-live-token")
    }));

    let cached_v2 = agent
        .request(
            "execute-v2-from-cache",
            "run_operator_task_ir",
            json!({ "mode": "execute", "task_ir": task_v2 }),
        )
        .expect("execute v2 from cache after central server exits");
    assert_eq!(cached_v2["ok"], true, "response: {cached_v2}");
    assert_eq!(
        cached_v2["result"]["operator_package_execution"]["cache_status"],
        "verified_cache_hit"
    );
    assert_eq!(cached_v2["result"]["result"]["summary"]["max"], 8.0);

    let generations = generation_roots(&work_root.join("store"));
    assert_eq!(generations.len(), 1, "retired generation was not reaped");
    let active_manifest = fs::read_to_string(
        generations[0]
            .join("packages")
            .join(PACKAGE_ID)
            .join("kyuubiki-operator.json"),
    )
    .expect("read active generation manifest");
    let active_manifest: Value =
        serde_json::from_str(&active_manifest).expect("decode active generation manifest");
    assert_eq!(active_manifest["package_version"], PACKAGE_VERSION_V2);
}

fn central_operator_task(package_version: &str, entrypoint_sha256: &str) -> Value {
    let mut task = external_operator_task(entrypoint_sha256);
    let package_ref = format!("orchestra://operator-package/{PACKAGE_ID}");
    task["execution_program"]["package_ref"] = json!(package_ref);
    task["runtime_hints"]["package_ref"] = json!(package_ref);
    task["runtime_hints"]["authority_mode"] = json!("central_operator_library");
    task["runtime_hints"]["execution_mode"] = json!("orchestra_fetch");
    task["runtime_hints"]["cache_scope"] = json!("job");
    task["runtime_hints"]["agent_fetchable"] = json!(true);
    task["execution_program"]["package_version"] = json!(package_version);
    task["runtime_hints"]["package_version"] = json!(package_version);
    refresh_task_digest(&mut task);
    task
}

#[derive(Clone)]
struct CentralVersionFixture {
    version: String,
    resolution: Vec<u8>,
    manifest: Vec<u8>,
}

struct CentralFixture {
    versions: Vec<CentralVersionFixture>,
    entrypoint: Vec<u8>,
    target: String,
}

impl CentralFixture {
    fn new(entrypoint: Vec<u8>) -> Self {
        let target = current_platform_target_id();
        let entrypoint_name = current_platform_library_file_name("kyuubiki_operator_template");
        let versions = [PACKAGE_VERSION_V1, PACKAGE_VERSION_V2]
            .into_iter()
            .map(|version| {
                let manifest = serde_json::to_vec(&json!({
            "schema_version": "kyuubiki.operator-package/v1",
            "sdk_api_version": "kyuubiki.operator-sdk/v1",
            "package_id": PACKAGE_ID,
            "package_version": version,
            "minimum_host_version": "1.15.0",
            "validation_status": "partial",
            "validation_notes": "Bound Orchestra Agent fetch qualification fixture.",
            "runtime": "rust_crate",
            "entrypoint": "target/debug/{lib_prefix}kyuubiki_operator_template.{lib_extension}",
            "operators": [{
                "operator_id": PACKAGE_ID,
                "kind": "extract",
                "entry_symbol": "register_template_operator"
            }]
        }))
        .expect("encode central fixture manifest");
                let base =
                    format!("/api/v1/central/operator-packages/{PACKAGE_ID}/{version}/{target}");
                let resolution = serde_json::to_vec(&json!({
                    "schema_version": "kyuubiki.operator-package-resolution/v1",
                    "package_ref": format!("orchestra://operator-package/{PACKAGE_ID}"),
                    "package_id": PACKAGE_ID,
                    "package_version": version,
                    "sdk_api_version": "kyuubiki.operator-sdk/v1",
                    "target": target,
                    "authority_mode": "bound_orchestra",
                    "cache_scope": "task_required_disposable",
                    "distribution_sha256": "d".repeat(64),
                    "manifest": {
                        "path": format!("{PACKAGE_ID}/{version}/{target}/kyuubiki-operator.json"),
                        "sha256": sha256(&manifest),
                        "size_bytes": manifest.len(),
                        "download_path": format!("{base}/manifest")
                    },
                    "entrypoint": {
                        "path": format!("{PACKAGE_ID}/{version}/{target}/{entrypoint_name}"),
                        "sha256": sha256(&entrypoint),
                        "size_bytes": entrypoint.len(),
                        "download_path": format!("{base}/entrypoint")
                    }
                }))
                .expect("encode central fixture resolution");
                CentralVersionFixture {
                    version: version.to_string(),
                    resolution,
                    manifest,
                }
            })
            .collect();
        Self {
            versions,
            entrypoint,
            target,
        }
    }

    fn serve(&self) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind central fixture");
        let address = listener.local_addr().expect("central fixture address");
        let versions = self.versions.clone();
        let entrypoint = self.entrypoint.clone();
        let target = self.target.clone();
        let handle = thread::spawn(move || {
            let mut requests = Vec::new();
            while requests.len() < versions.len() * 3 {
                let (mut stream, _) = listener.accept().expect("accept central request");
                let request = read_request(&mut stream);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .expect("central request path");
                if path.starts_with("/api/v1/agents/") {
                    write_response(&mut stream, b"{}");
                    continue;
                }
                let body = versions
                    .iter()
                    .find_map(|fixture| {
                        let base = format!(
                            "/api/v1/central/operator-packages/{PACKAGE_ID}/{}/{target}",
                            fixture.version
                        );
                        match path {
                            value if value == format!("{base}/resolve") => {
                                Some(fixture.resolution.as_slice())
                            }
                            value if value == format!("{base}/manifest") => {
                                Some(fixture.manifest.as_slice())
                            }
                            value if value == format!("{base}/entrypoint") => {
                                Some(entrypoint.as_slice())
                            }
                            _ => None,
                        }
                    })
                    .unwrap_or_else(|| panic!("unexpected central fixture request {path}"));
                write_response(&mut stream, body);
                requests.push(request);
            }
            requests
        });
        (format!("http://{address}"), handle)
    }
}

fn generation_roots(store_root: &std::path::Path) -> Vec<PathBuf> {
    let root = store_root.join("agent-runtime-generations");
    let mut generations = fs::read_dir(root)
        .expect("read Agent package generations")
        .map(|entry| entry.expect("read generation entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    generations.sort();
    generations
}

fn read_request(stream: &mut impl Read) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        let count = stream.read(&mut buffer).expect("read central request");
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    String::from_utf8(bytes).expect("central request is UTF-8")
}

fn write_response(stream: &mut impl Write, body: &[u8]) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .expect("write central response headers");
    stream.write_all(body).expect("write central response");
    stream.flush().expect("flush central response");
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("kyuubiki-{label}-{nonce}"))
}

struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
