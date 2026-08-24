use crate::{fetch_operator_package_into, verify_managed_operator_package};
use kyuubiki_operator_sdk::{current_platform_library_file_name, current_platform_target_id};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

const PACKAGE_ID: &str = "operator.fetch.fixture";
const PACKAGE_VERSION: &str = "0.1.0";

#[test]
fn fetches_verifies_and_installs_current_platform_package() {
    let root = temp_dir("operator-package-fetch");
    let _cleanup = Cleanup(root.clone());
    let store = root.join("store");
    let fixture = Fixture::new(false);
    let (central_url, server) = fixture.serve();

    let receipt = fetch_operator_package_into(
        &central_url,
        PACKAGE_ID,
        PACKAGE_VERSION,
        &store,
        Some("fixture-token"),
    )
    .expect("fetch and install operator package");

    assert_eq!(receipt.package_id, PACKAGE_ID);
    assert_eq!(receipt.package_version, PACKAGE_VERSION);
    assert_eq!(receipt.entrypoint_sha256, sha256(&fixture.entrypoint));
    verify_managed_operator_package(&store.join(&receipt.relative_root))
        .expect("installed package remains verifiable");
    let requests = server.join().expect("fixture server");
    assert_eq!(requests.len(), 3);
    assert!(requests.iter().all(|request| {
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer fixture-token")
    }));
}

#[test]
fn rejects_digest_mismatch_without_leaving_an_installed_package() {
    let root = temp_dir("operator-package-fetch-digest-mismatch");
    let _cleanup = Cleanup(root.clone());
    let store = root.join("store");
    let fixture = Fixture::new(true);
    let (central_url, server) = fixture.serve();

    let error =
        fetch_operator_package_into(&central_url, PACKAGE_ID, PACKAGE_VERSION, &store, None)
            .expect_err("digest mismatch must fail");

    assert!(error.contains("entrypoint digest mismatch"));
    assert!(!store.join("packages").join(PACKAGE_ID).exists());
    assert_eq!(server.join().expect("fixture server").len(), 3);
}

struct Fixture {
    resolution: Vec<u8>,
    manifest: Vec<u8>,
    entrypoint: Vec<u8>,
    target: String,
}

impl Fixture {
    fn new(bad_entrypoint_digest: bool) -> Self {
        let target = current_platform_target_id();
        let entrypoint_name = current_platform_library_file_name("operator_fetch_fixture");
        let entrypoint = b"operator-fetch-fixture".to_vec();
        let manifest = serde_json::to_vec(&serde_json::json!({
            "schema_version": "kyuubiki.operator-package/v1",
            "sdk_api_version": "kyuubiki.operator-sdk/v1",
            "package_id": PACKAGE_ID,
            "package_version": PACKAGE_VERSION,
            "minimum_host_version": "1.15.0",
            "validation_status": "partial",
            "validation_notes": "Installer central fetch fixture.",
            "runtime": "rust_crate",
            "entrypoint": "target/debug/{lib_prefix}operator_fetch_fixture.{lib_extension}",
            "operators": [{
                "operator_id": "extract.operator_fetch_fixture",
                "kind": "extract",
                "entry_symbol": "register_operator_fetch_fixture"
            }]
        }))
        .expect("encode fixture manifest");
        let base_path =
            format!("/api/v1/central/operator-packages/{PACKAGE_ID}/{PACKAGE_VERSION}/{target}");
        let entrypoint_digest = if bad_entrypoint_digest {
            "0".repeat(64)
        } else {
            sha256(&entrypoint)
        };
        let resolution = serde_json::to_vec(&serde_json::json!({
            "schema_version": "kyuubiki.operator-package-resolution/v1",
            "package_ref": format!("orchestra://operator-package/{PACKAGE_ID}"),
            "package_id": PACKAGE_ID,
            "package_version": PACKAGE_VERSION,
            "sdk_api_version": "kyuubiki.operator-sdk/v1",
            "target": target,
            "authority_mode": "bound_orchestra",
            "cache_scope": "task_required_disposable",
            "distribution_sha256": "d".repeat(64),
            "manifest": {
                "path": format!("{PACKAGE_ID}/{PACKAGE_VERSION}/{target}/kyuubiki-operator.json"),
                "sha256": sha256(&manifest),
                "size_bytes": manifest.len(),
                "download_path": format!("{base_path}/manifest")
            },
            "entrypoint": {
                "path": format!("{PACKAGE_ID}/{PACKAGE_VERSION}/{target}/{entrypoint_name}"),
                "sha256": entrypoint_digest,
                "size_bytes": entrypoint.len(),
                "download_path": format!("{base_path}/entrypoint")
            }
        }))
        .expect("encode fixture resolution");
        Self {
            resolution,
            manifest,
            entrypoint,
            target,
        }
    }

    fn serve(&self) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture server");
        let address = listener.local_addr().expect("fixture address");
        let resolution = self.resolution.clone();
        let manifest = self.manifest.clone();
        let entrypoint = self.entrypoint.clone();
        let target = self.target.clone();
        let handle = thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().expect("accept request");
                let request = read_request(&mut stream);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .expect("request path");
                let base = format!(
                    "/api/v1/central/operator-packages/{PACKAGE_ID}/{PACKAGE_VERSION}/{target}"
                );
                let body = match path {
                    value if value == format!("{base}/resolve") => &resolution,
                    value if value == format!("{base}/manifest") => &manifest,
                    value if value == format!("{base}/entrypoint") => &entrypoint,
                    _ => panic!("unexpected fixture request {path}"),
                };
                write_response(&mut stream, body);
                requests.push(request);
            }
            requests
        });
        (format!("http://{address}"), handle)
    }
}

fn read_request(stream: &mut impl Read) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        let count = stream.read(&mut buffer).expect("read fixture request");
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    String::from_utf8(bytes).expect("HTTP request is UTF-8")
}

fn write_response(stream: &mut impl Write, body: &[u8]) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .expect("write fixture response headers");
    stream.write_all(body).expect("write fixture response body");
    stream.flush().expect("flush fixture response");
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
