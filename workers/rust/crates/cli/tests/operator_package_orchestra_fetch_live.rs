mod support;

use kyuubiki_operator_sdk::{current_platform_library_file_name, current_platform_target_id};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
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
const JOB_ID: &str = "operator-fetch-live-job";
const SHARED_JOB_ID: &str = "operator-fetch-shared-job";
const REFETCH_JOB_ID: &str = "operator-refetch-live-job";

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
    let fixture = CentralFixture::new(entrypoint.clone());
    let (central_url, server) = fixture.serve();
    let agent = LiveAgent::start_orchestrated(&packages_root, &central_url, TOKEN)
        .expect("start orchestrated Agent with empty package cache");

    let task_v1 = central_operator_task(PACKAGE_VERSION_V1, &entrypoint_sha256);
    let missing_job = agent
        .request(
            "reject-job-cache-without-job-id",
            "run_operator_task_ir",
            json!({ "mode": "execute", "task_ir": task_v1.clone() }),
        )
        .expect("reject missing job identity before package fetch");
    assert_eq!(missing_job["ok"], false, "response: {missing_job}");
    assert_eq!(
        missing_job["error"]["code"],
        "operator_package_job_id_missing"
    );
    let first = agent
        .request(
            "fetch-and-execute",
            "run_operator_task_ir",
            json!({ "mode": "execute", "job_id": JOB_ID, "task_ir": task_v1.clone() }),
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
    assert_eq!(
        first["result"]["operator_package_execution"]["cache_generation"]["janitor"]["removed_stale_session_count"],
        0
    );

    let cached_v1 = agent
        .request(
            "execute-v1-from-cache",
            "run_operator_task_ir",
            json!({ "mode": "execute", "job_id": JOB_ID, "task_ir": task_v1 }),
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
            json!({ "mode": "execute", "job_id": JOB_ID, "task_ir": task_v2.clone() }),
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
            json!({ "mode": "execute", "job_id": JOB_ID, "task_ir": task_v2 }),
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

    let peer_fixture = CentralFixture::for_versions(entrypoint.clone(), &[PACKAGE_VERSION_V2]);
    let (peer_url, peer_server) = peer_fixture.serve();
    let peer_agent = LiveAgent::start_orchestrated(&packages_root, &peer_url, TOKEN)
        .expect("start peer Agent over the same managed cache");
    let peer_execution = peer_agent
        .request(
            "fetch-while-peer-session-is-live",
            "run_operator_task_ir",
            json!({
                "mode": "execute",
                "job_id": JOB_ID,
                "task_ir": central_operator_task(PACKAGE_VERSION_V2, &entrypoint_sha256)
            }),
        )
        .expect("execute while another cache session remains live");
    assert_eq!(peer_execution["ok"], true, "response: {peer_execution}");
    assert_eq!(
        peer_execution["result"]["operator_package_execution"]["cache_generation"]["janitor"]["retained_active_session_count"],
        1
    );
    assert_eq!(peer_server.join().expect("peer central fixture").len(), 3);
    let stale_sessions = session_roots(&work_root.join("store"));
    assert_eq!(stale_sessions.len(), 2);
    drop(peer_agent);
    drop(agent);
    assert!(
        stale_sessions.iter().all(|session| session.exists()),
        "abrupt exits should leave recovery work"
    );

    let restart_fixture = CentralFixture::for_versions(
        entrypoint.clone(),
        &[PACKAGE_VERSION_V2, PACKAGE_VERSION_V2],
    );
    let (restart_url, restart_server) = restart_fixture.serve();
    let restarted_agent = LiveAgent::start_orchestrated(&packages_root, &restart_url, TOKEN)
        .expect("restart Agent over the same managed cache");
    let restarted = restarted_agent
        .request(
            "fetch-after-crash-recovery",
            "run_operator_task_ir",
            json!({
                "mode": "execute",
                "job_id": JOB_ID,
                "task_ir": central_operator_task(PACKAGE_VERSION_V2, &entrypoint_sha256)
            }),
        )
        .expect("execute after stale generation recovery");
    assert_eq!(restarted["ok"], true, "response: {restarted}");
    assert_eq!(
        restarted["result"]["operator_package_execution"]["cache_generation"]["janitor"]["removed_stale_session_count"],
        2
    );
    assert!(
        stale_sessions.iter().all(|session| !session.exists()),
        "stale sessions were not reclaimed"
    );
    assert_eq!(session_roots(&work_root.join("store")).len(), 1);
    assert_eq!(generation_roots(&work_root.join("store")).len(), 1);
    let shared = restarted_agent
        .request(
            "share-job-scoped-package",
            "run_operator_task_ir",
            json!({
                "mode": "execute",
                "job_id": SHARED_JOB_ID,
                "task_ir": central_operator_task(PACKAGE_VERSION_V2, &entrypoint_sha256)
            }),
        )
        .expect("share package with a second job owner");
    assert_eq!(shared["ok"], true, "response: {shared}");
    assert_eq!(
        shared["result"]["operator_package_execution"]["cache_status"],
        "verified_cache_hit"
    );
    let released = restarted_agent
        .request(
            "release-job-scoped-package",
            "release_operator_package_job",
            json!({ "job_id": JOB_ID }),
        )
        .expect("release job-scoped package");
    assert_eq!(released["ok"], true, "response: {released}");
    let release = &released["result"];
    assert_eq!(release["disposition"], "released_retained_packages");
    assert_eq!(release["released_package_ids"], json!([PACKAGE_ID]));
    assert_eq!(release["retained_package_ids"], json!([PACKAGE_ID]));
    assert_eq!(release["remaining_activated_package_count"], 1);
    let repeated_release = restarted_agent
        .request(
            "release-job-scoped-package-again",
            "release_operator_package_job",
            json!({ "job_id": JOB_ID }),
        )
        .expect("repeat job-scoped package release");
    assert_eq!(repeated_release["ok"], true, "response: {repeated_release}");
    assert_eq!(
        repeated_release["result"]["disposition"],
        "already_released"
    );
    let shared_release = restarted_agent
        .request(
            "release-final-shared-job-owner",
            "release_operator_package_job",
            json!({ "job_id": SHARED_JOB_ID }),
        )
        .expect("release final shared job owner");
    assert_eq!(shared_release["ok"], true, "response: {shared_release}");
    assert_eq!(
        shared_release["result"]["disposition"],
        "evicted_after_job_release"
    );
    assert_eq!(
        shared_release["result"]["evicted_package_ids"],
        json!([PACKAGE_ID])
    );
    assert_eq!(
        shared_release["result"]["remaining_activated_package_count"],
        0
    );

    let disposable = restarted_agent
        .request(
            "execute-and-evict-task-scoped-package",
            "run_operator_task_ir",
            json!({
                "mode": "execute",
                "task_ir": central_operator_task_with_scope(
                    PACKAGE_VERSION_V2,
                    &entrypoint_sha256,
                    "none"
                )
            }),
        )
        .expect("execute and evict disposable package");
    assert_eq!(disposable["ok"], true, "response: {disposable}");
    let eviction = &disposable["result"]["operator_package_execution"]["cache_eviction"];
    assert_eq!(eviction["disposition"], "evicted_after_execution");
    assert_eq!(eviction["requested_cache_scope"], "none");
    assert_eq!(
        eviction["resolved_cache_policy"],
        "task_required_disposable"
    );
    assert_eq!(eviction["remaining_activated_package_count"], 0);
    let disposable_generations = generation_roots(&work_root.join("store"));
    assert_eq!(disposable_generations.len(), 1);
    assert!(
        !disposable_generations[0]
            .join("packages")
            .join(PACKAGE_ID)
            .exists(),
        "task-scoped package remained in the active generation"
    );
    assert_eq!(
        restart_server
            .join()
            .expect("restart central fixture")
            .len(),
        6
    );
    drop(restarted_agent);

    let refetch_fixture = CentralFixture::for_versions(
        entrypoint.clone(),
        &[PACKAGE_VERSION_V2, PACKAGE_VERSION_V2],
    );
    let (refetch_url, refetch_server) = refetch_fixture.serve();
    let refetch_agent = LiveAgent::start_orchestrated(&packages_root, &refetch_url, TOKEN)
        .expect("restart Agent after disposable package eviction");
    let refetched = refetch_agent
        .request(
            "refetch-after-task-scope-eviction",
            "run_operator_task_ir",
            json!({
                "mode": "execute",
                "job_id": REFETCH_JOB_ID,
                "task_ir": central_operator_task(PACKAGE_VERSION_V2, &entrypoint_sha256)
            }),
        )
        .expect("refetch package after task-scope eviction");
    assert_eq!(refetched["ok"], true, "response: {refetched}");
    assert_eq!(
        refetched["result"]["operator_package_execution"]["cache_status"],
        "fetched_and_activated"
    );
    assert_eq!(
        refetched["result"]["operator_package_execution"]["cache_generation"]["janitor"]["removed_stale_session_count"],
        1
    );
    let refetch_release = refetch_agent
        .request(
            "cancel-refetched-job-package",
            "cancel_job",
            json!({ "job_id": REFETCH_JOB_ID }),
        )
        .expect("cancel and release refetched job package");
    assert_eq!(refetch_release["ok"], true, "response: {refetch_release}");
    assert_eq!(
        refetch_release["result"]["operator_package_job_release"]["disposition"],
        "evicted_after_job_release"
    );

    let mut failing_disposable =
        central_operator_task_with_scope(PACKAGE_VERSION_V2, &entrypoint_sha256, "none");
    failing_disposable["input_artifact"]["values"] = json!([]);
    refresh_task_digest(&mut failing_disposable);
    let rejected = refetch_agent
        .request(
            "failed-dispatch-still-evicts-task-package",
            "run_operator_task_ir",
            json!({ "mode": "execute", "task_ir": failing_disposable }),
        )
        .expect("receive failed disposable dispatch response");
    assert_eq!(rejected["ok"], false, "response: {rejected}");
    assert_eq!(
        rejected["error"]["code"],
        "operator_package_dispatch_failed"
    );
    let rejected_eviction =
        &rejected["error"]["details"]["operator_task_failure_receipt"]["cache_eviction"];
    assert_eq!(rejected_eviction["disposition"], "evicted_after_execution");
    assert_eq!(rejected_eviction["remaining_activated_package_count"], 0);
    let failed_dispatch_generations = generation_roots(&work_root.join("store"));
    assert_eq!(failed_dispatch_generations.len(), 1);
    assert!(
        !failed_dispatch_generations[0]
            .join("packages")
            .join(PACKAGE_ID)
            .exists(),
        "failed task left its disposable package active"
    );
    assert_eq!(refetch_server.join().expect("refetch fixture").len(), 6);
    drop(refetch_agent);

    let concurrent_fixture =
        CentralFixture::for_versions(entrypoint, &[PACKAGE_VERSION_V2, PACKAGE_VERSION_V2]);
    let (concurrent_url, concurrent_server) = concurrent_fixture.serve();
    let concurrent_agent = Arc::new(
        LiveAgent::start_orchestrated(&packages_root, &concurrent_url, TOKEN)
            .expect("start Agent for concurrent disposable tasks"),
    );
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for index in 0..2 {
        let agent = Arc::clone(&concurrent_agent);
        let barrier = Arc::clone(&barrier);
        let digest = entrypoint_sha256.clone();
        workers.push(thread::spawn(move || {
            barrier.wait();
            agent
                .request(
                    &format!("concurrent-disposable-{index}"),
                    "run_operator_task_ir",
                    json!({
                        "mode": "execute",
                        "task_ir": central_operator_task_with_scope(
                            PACKAGE_VERSION_V2,
                            &digest,
                            "none"
                        )
                    }),
                )
                .map_err(|error| error.to_string())
        }));
    }
    barrier.wait();
    for worker in workers {
        let response = worker
            .join()
            .expect("concurrent disposable request thread")
            .expect("concurrent disposable request");
        assert_eq!(response["ok"], true, "response: {response}");
        assert_eq!(
            response["result"]["operator_package_execution"]["cache_eviction"]["disposition"],
            "evicted_after_execution"
        );
    }
    assert_eq!(
        concurrent_server
            .join()
            .expect("concurrent central fixture")
            .len(),
        6
    );
    let concurrent_generations = generation_roots(&work_root.join("store"));
    assert_eq!(concurrent_generations.len(), 1);
    assert!(
        !concurrent_generations[0]
            .join("packages")
            .join(PACKAGE_ID)
            .exists(),
        "concurrent disposable tasks left package residue"
    );
}

fn central_operator_task(package_version: &str, entrypoint_sha256: &str) -> Value {
    central_operator_task_with_scope(package_version, entrypoint_sha256, "job")
}

fn central_operator_task_with_scope(
    package_version: &str,
    entrypoint_sha256: &str,
    cache_scope: &str,
) -> Value {
    let mut task = external_operator_task(entrypoint_sha256);
    let package_ref = format!("orchestra://operator-package/{PACKAGE_ID}");
    task["execution_program"]["package_ref"] = json!(package_ref);
    task["runtime_hints"]["package_ref"] = json!(package_ref);
    task["runtime_hints"]["authority_mode"] = json!("central_operator_library");
    task["runtime_hints"]["execution_mode"] = json!("orchestra_fetch");
    task["runtime_hints"]["cache_scope"] = json!(cache_scope);
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
        Self::for_versions(entrypoint, &[PACKAGE_VERSION_V1, PACKAGE_VERSION_V2])
    }

    fn for_versions(entrypoint: Vec<u8>, package_versions: &[&str]) -> Self {
        let target = current_platform_target_id();
        let entrypoint_name = current_platform_library_file_name("kyuubiki_operator_template");
        let versions = package_versions
            .iter()
            .copied()
            .map(|version| {
                let manifest = serde_json::to_vec(&json!({
            "schema_version": "kyuubiki.operator-package/v1",
            "sdk_api_version": "kyuubiki.operator-sdk/v1",
            "execution_abi": "kyuubiki.operator-json-c/v1",
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
                "entry_symbol": "run_template_operator_json"
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
                    "execution_abi": "kyuubiki.operator-json-c/v1",
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
    let mut generations = session_roots(store_root)
        .into_iter()
        .flat_map(|session| {
            fs::read_dir(session.join("generations"))
                .expect("read session generations")
                .map(|entry| entry.expect("read generation entry").path())
        })
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    generations.sort();
    generations
}

fn session_roots(store_root: &std::path::Path) -> Vec<PathBuf> {
    let root = store_root.join("agent-runtime-generations/sessions");
    let mut sessions = fs::read_dir(root)
        .expect("read Agent package sessions")
        .map(|entry| entry.expect("read session entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    sessions.sort();
    sessions
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
